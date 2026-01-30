//! Tunex Client Agent: CLI that connects to gateway and exposes local services.

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tunex_common::{RegisterRequest, RegisterResponse, ServiceType};

#[derive(Parser, Debug)]
#[command(name = "tunex-client")]
#[command(about = "Tunex Client — expose local services through the Tunex Gateway")]
struct Args {
    /// Local port to forward (e.g. 3000)
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Gateway WebSocket URL
    #[arg(short, long, default_value = "ws://127.0.0.1:8080/ws")]
    gateway: String,

    /// Tunnel name (used in gateway path /tunnel/<name>/...)
    #[arg(short, long, default_value = "default")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    println!("Connecting to gateway {} ...", args.gateway);
    let (ws_stream, _) = connect_async(&args.gateway).await?;
    println!("Connected.");

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Send registration
    let reg = RegisterRequest {
        local_port: args.port,
        service_type: ServiceType::Http,
        tunnel_name: args.name.clone(),
    };
    let reg_json = serde_json::to_string(&reg)?;
    ws_tx.send(Message::Text(reg_json.into())).await?;

    // Expect RegisterResponse
    match ws_rx.next().await {
        Some(Ok(Message::Text(text))) => {
            let resp: RegisterResponse = serde_json::from_str(&text)?;
            println!("Tunnel registered. Public URL: {}", resp.public_url);
        }
        Some(Ok(Message::Close(_))) => {
            eprintln!("Gateway closed connection during registration");
            return Ok(());
        }
        other => {
            eprintln!("Unexpected message from gateway: {:?}", other);
            return Ok(());
        }
    }

    let local_addr = format!("127.0.0.1:{}", args.port);
    println!("Forwarding to {} (tunnel: {})", local_addr, args.name);

    // Loop: receive request from gateway, forward to localhost, send response back
    while let Some(Ok(msg)) = ws_rx.next().await {
        let request_bytes = match msg {
            Message::Binary(b) => b,
            Message::Text(t) => t.into_bytes(),
            Message::Close(_) => break,
            _ => continue,
        };

        let response_bytes = forward_to_local(&local_addr, &request_bytes).await;
        if ws_tx.send(Message::Binary(response_bytes)).await.is_err() {
            break;
        }
    }

    println!("Connection closed.");
    Ok(())
}

/// Forward raw HTTP request to local service and return raw response bytes.
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
