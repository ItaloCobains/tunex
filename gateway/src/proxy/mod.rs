//! Tunnel proxy: route by Host header (subdomain); forward full path to the client over TCP.
//! Looks up client address in Redis by tunnel id (subdomain), opens TCP, sends request, returns response.

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
};
use http::request::Parts;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use std::time::Duration;

use crate::app::AppState;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TCP_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Extract tunnel id (subdomain) from Host header. Host may include port (e.g. subdomain.localhost:8080).
fn tunnel_id_from_host(host: Option<&str>, base_domain: &str) -> Option<String> {
    let host = host?.trim();
    let hostname = host.split(':').next().unwrap_or(host);
    if hostname.is_empty() || hostname == base_domain {
        return None;
    }
    let suffix = format!(".{}", base_domain);
    if hostname.ends_with(&suffix) {
        let sub = hostname.strip_suffix(&suffix).unwrap_or("");
        if !sub.is_empty() {
            return Some(sub.to_string());
        }
    }
    None
}

/// Forward request to tunnel identified by Host subdomain; 404 if missing subdomain or tunnel not found.
pub async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let host = req.headers().get(http::header::HOST).and_then(|v| v.to_str().ok());
    let tunnel_id = match tunnel_id_from_host(host, &state.base_domain) {
        Some(id) => id,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("missing subdomain (use <tunnel_id>.<base_domain>)"))
                .unwrap();
        }
    };

    let client_address = match state.registry.get(&tunnel_id).await {
        Ok(Some(addr)) => addr,
        Ok(None) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("tunnel not found: {}", tunnel_id)))
                .unwrap();
        }
        Err(e) => {
            eprintln!("Redis get error: {}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("registry error"))
                .unwrap();
        }
    };

    let (parts, body) = req.into_parts();
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let req_bytes = build_http_request(&parts, &path_and_query, body).await;

    match send_request_over_tcp(&client_address, &req_bytes).await {
        Ok(response_bytes) => parse_http_response(response_bytes),
        Err(e) => {
            eprintln!("TCP to client {} failed: {}", client_address, e);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(format!("tunnel disconnected: {}", e)))
                .unwrap()
        }
    }
}

/// Send raw HTTP request over a new TCP connection to the client and return raw response bytes.
/// One connection per call in MVP; can be reused with a connection pool for keep-alive later.
pub async fn send_request_over_tcp(
    client_address: &str,
    request_bytes: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let stream = timeout(
        TCP_CONNECT_TIMEOUT,
        TcpStream::connect(client_address),
    )
    .await
    .map_err(|_| "connect timeout")?
    .map_err(|e| format!("connect failed: {}", e))?;

    let mut stream = stream;
    stream.write_all(request_bytes).await?;
    let _ = stream.shutdown().await;

    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        let n = timeout(TCP_READ_TIMEOUT, stream.read(&mut tmp)).await;
        match n {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => buf.extend_from_slice(&tmp[..n]),
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err("read timeout".into()),
        }
    }
    Ok(buf)
}

async fn build_http_request(parts: &Parts, path_and_query: &str, body: Body) -> Vec<u8> {
    let method = parts.method.as_str();
    let path = if path_and_query.is_empty() || !path_and_query.starts_with('/') {
        "/"
    } else {
        path_and_query
    };

    let mut buf = format!("{} {} HTTP/1.1\r\n", method, path);
    for (k, v) in parts.headers.iter() {
        if k.as_str().eq_ignore_ascii_case("transfer-encoding") {
            continue;
        }
        buf.push_str(&format!("{}: {}\r\n", k.as_str(), v.to_str().unwrap_or("")));
    }
    let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024).await.unwrap_or_default();
    buf.push_str("\r\n");
    let mut out = buf.into_bytes();
    out.extend(&body_bytes);
    out
}

fn parse_http_response(mut bytes: Vec<u8>) -> Response<Body> {
    let sep = b"\r\n\r\n";
    let pos = bytes.windows(sep.len()).position(|w| w == sep).unwrap_or(0);
    let headers_end = pos + sep.len();
    let (head, body) = if pos > 0 {
        let (h, b) = bytes.split_at_mut(headers_end);
        (h.to_vec(), b.to_vec())
    } else {
        (bytes.clone(), vec![])
    };

    let head_str = String::from_utf8_lossy(&head);
    let mut lines = head_str.lines();
    let status_line = lines.next().unwrap_or("HTTP/1.1 500 Internal Server Error");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(500);

    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder().status(status);

    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if let Ok(name) = http::header::HeaderName::try_from(k.trim()) {
                if let Ok(value) = http::header::HeaderValue::try_from(v.trim()) {
                    builder = builder.header(name, value);
                }
            }
        }
    }

    builder.body(Body::from(body)).unwrap()
}
