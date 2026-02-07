//! Tunex Client Agent: listens for gateway TCP connections and forwards to local service.
//! Registers with the gateway via HTTP POST; gateways discover client address from Redis.

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tunex_common::{HttpRegisterRequest, HttpRegisterResponse};
use std::time::Duration;

const HEARTBEAT_INTERVAL_SECS: u64 = 30;

#[derive(Parser, Debug)]
#[command(name = "tunex-client")]
#[command(about = "Tunex Client — expose local services through the Tunex Gateway (TCP + Redis)")]
struct Args {
    /// Local port of the service to forward (e.g. 3000)
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Port to listen on for gateway connections (gateways connect here)
    #[arg(short, long, default_value = "9090")]
    listen: u16,

    /// Address to advertise to the gateway (host:port gateways will connect to)
    #[arg(long, default_value = "127.0.0.1:9090")]
    advertise_address: String,

    /// Gateway base URL for registration (e.g. http://localhost:8080)
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    gateway: String,

    /// Tunnel name (used in gateway path /tunnel/<name>/...)
    #[arg(short, long, default_value = "default")]
    name: String,

    /// Optional auth token for registration; ignored by gateway in MVP
    #[arg(long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let listen_addr = format!("0.0.0.0:{}", args.listen);
    let listener = TcpListener::bind(&listen_addr).await?;
    println!("Listening for gateways on {}", listen_addr);

    let gateway_url = args.gateway.trim_end_matches('/');
    let register_url = format!("{}/register", gateway_url);

    // Initial registration
    if let Err(e) = do_register(&register_url, &args.name, &args.advertise_address, args.token.as_deref()).await {
        eprintln!("Initial registration failed: {}", e);
        return Err(e.into());
    }
    println!("Tunnel registered. Public URL: {}/tunnel/{}", gateway_url, args.name);

    let local_addr = format!("127.0.0.1:{}", args.port);
    println!("Forwarding to {} (tunnel: {})", local_addr, args.name);

    // Heartbeat task: re-register before TTL expires
    let heartbeat_url = register_url.clone();
    let heartbeat_name = args.name.clone();
    let heartbeat_address = args.advertise_address.clone();
    let heartbeat_token = args.token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        loop {
            interval.tick().await;
            if do_register(&heartbeat_url, &heartbeat_name, &heartbeat_address, heartbeat_token.as_deref()).await.is_err() {
                eprintln!("Heartbeat registration failed");
            }
        }
    });

    // Accept loop: one request per connection (MVP); structure allows multiple requests per connection later
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Accept error: {}", e);
                continue;
            }
        };
        let local = local_addr.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_gateway_connection(stream, &local).await {
                eprintln!("Connection from {} failed: {}", peer, e);
            }
        });
    }
}

async fn do_register(
    url: &str,
    tunnel_name: &str,
    client_address: &str,
    token: Option<&str>,
) -> Result<HttpRegisterResponse, Box<dyn std::error::Error + Send + Sync>> {
    let body = HttpRegisterRequest {
        tunnel_name: tunnel_name.to_string(),
        client_address: client_address.to_string(),
        token: token.map(String::from),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let resp = client.post(url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("register failed {}: {}", status, text).into());
    }
    let out: HttpRegisterResponse = resp.json().await?;
    Ok(out)
}

/// Handle one gateway TCP connection: read one HTTP request, forward to local, write response, close.
/// MVP: one request per connection; reading/writing is structured so multiple requests per connection can be added later.
async fn handle_gateway_connection(
    mut stream: TcpStream,
    local_addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request_bytes = read_http_request(&mut stream).await?;
    let response_bytes = forward_to_local(local_addr, &request_bytes).await;
    stream.write_all(&response_bytes).await?;
    let _ = stream.shutdown().await;
    Ok(())
}

/// Read a single HTTP request from the stream (headers + body by Content-Length or until close).
async fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let mut content_length: Option<usize> = None;

    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);

        if let Some(sep) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header_section = String::from_utf8_lossy(&buf[..sep]);
                for line in header_section.lines() {
                    if line.trim().to_lowercase().starts_with("content-length:") {
                        if let Some(num) = line.split(':').nth(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                            content_length = Some(num);
                        }
                        break;
                    }
                }
                let body_start = sep + 4;
                if let Some(cl) = content_length {
                    let need = body_start + cl;
                    while buf.len() < need {
                        let n = stream.read(&mut tmp).await?;
                        if n == 0 {
                            break;
                        }
                        buf.extend_from_slice(&tmp[..n]);
                    }
                }
                break;
        }
    }
    Ok(buf)
}

async fn forward_to_local(local_addr: &str, request_bytes: &[u8]) -> Vec<u8> {
    let mut stream = match TcpStream::connect(local_addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", local_addr, e);
            return error_response(502, "Bad Gateway", &format!("Connection to {} failed: {}", local_addr, e));
        }
    };

    if stream.write_all(request_bytes).await.is_err() {
        return error_response(502, "Bad Gateway", "Failed to send request to local service");
    }
    let _ = stream.shutdown().await;

    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => {
                return error_response(502, "Bad Gateway", "Error reading from local service");
            }
        }
    }
    buf
}

fn error_response(code: u16, reason: &str, body: &str) -> Vec<u8> {
    let status = format!("HTTP/1.1 {} {}\r\n", code, reason);
    let content = format!(
        "Content-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    format!("{}{}", status, content).into_bytes()
}
