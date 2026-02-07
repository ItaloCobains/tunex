//! Tunnel proxy: forward /tunnel/:name/* to the client over TCP.
//! Looks up client address in Redis, opens TCP, sends request, returns response.
//! One connection per request in MVP; helper is reusable for future keep-alive/pooling.

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

/// Forward request to tunnel `name`; 404 if path is not /tunnel/:name/... or tunnel not found.
pub async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let (name, backend_path) = match path.strip_prefix("/tunnel/") {
        Some(rest) => {
            let mut segments = rest.splitn(2, '/');
            let name = segments.next().unwrap_or("").to_string();
            let backend_path = segments.next().unwrap_or("");
            (name, backend_path)
        }
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("use /tunnel/<name>/... to reach a tunnel"))
                .unwrap();
        }
    };
    if name.is_empty() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("missing tunnel name"))
            .unwrap();
    }

    let backend_path_owned = backend_path.to_string();

    let client_address = match state.registry.get(&name).await {
        Ok(Some(addr)) => addr,
        Ok(None) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("tunnel not found: {}", name)))
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
    let req_bytes = build_http_request(&parts, &backend_path_owned, body).await;

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

async fn build_http_request(parts: &Parts, backend_path: &str, body: Body) -> Vec<u8> {
    let method = parts.method.as_str();
    let uri = &parts.uri;
    let path: String = if backend_path.is_empty() {
        "/".into()
    } else if backend_path.starts_with('/') {
        backend_path.into()
    } else {
        format!("/{}", backend_path)
    };
    let path_and_query = match uri.query() {
        Some(q) => format!("{}?{}", path, q),
        None => path,
    };

    let mut buf = format!("{} {} HTTP/1.1\r\n", method, path_and_query);
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
