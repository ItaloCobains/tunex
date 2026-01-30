//! Tunex Gateway Server: WebSocket tunnel registry and HTTP ingress proxy.

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tunex_common::{RegisterRequest, RegisterResponse};

/// Per-tunnel handle: send request bytes and get response bytes via oneshot.
type TunnelHandle = mpsc::Sender<(Vec<u8>, tokio::sync::oneshot::Sender<Vec<u8>>)>;

#[derive(Clone)]
struct AppState {
    /// tunnel_name -> channel to send requests to the client's WS handler
    registry: Arc<RwLock<HashMap<String, TunnelHandle>>>,
    /// Default port for display (e.g. 8080)
    port: u16,
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let state = AppState {
        registry: Arc::new(RwLock::new(HashMap::new())),
        port,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(proxy_handler)
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Tunex Gateway listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(state, socket))
}

async fn handle_websocket(state: AppState, socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // First message must be RegisterRequest (JSON)
    let (mut request_rx, tunnel_name) = match ws_rx.next().await {
        Some(Ok(Message::Text(text))) => {
            let reg: RegisterRequest = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(_) => {
                    let _ = ws_tx.send(Message::Text("{\"error\":\"invalid RegisterRequest\"}".into())).await;
                    return;
                }
            };

            let (request_tx, request_rx_inner) = mpsc::channel(1);
            let tunnel_id = format!("{:?}", std::time::SystemTime::now());
            let public_url = format!("http://localhost:{}/tunnel/{}", state.port, reg.tunnel_name);
            {
                let mut registry = state.registry.write().await;
                registry.insert(reg.tunnel_name.clone(), request_tx);
            }

            let response = RegisterResponse {
                tunnel_id: tunnel_id.clone(),
                public_url: public_url.clone(),
            };
            let response_json = serde_json::to_string(&response).unwrap();
            if ws_tx.send(Message::Text(response_json.into())).await.is_err() {
                return;
            }
            println!("Tunnel registered: {} -> {}", reg.tunnel_name, public_url);
            (request_rx_inner, Some(reg.tunnel_name))
        }
        _ => {
            let _ = ws_tx.send(Message::Text("{\"error\":\"expected JSON registration\"}".into())).await;
            return;
        }
    };
    let mut pending_reply: Option<tokio::sync::oneshot::Sender<Vec<u8>>> = None;

    loop {
        tokio::select! {
            Some((req_bytes, reply_tx)) = request_rx.recv() => {
                if ws_tx.send(Message::Binary(req_bytes)).await.is_err() {
                    break;
                }
                pending_reply = Some(reply_tx);
            }
            Some(Ok(msg)) = ws_rx.next() => {
                let response_bytes = match msg {
                    Message::Binary(b) => b,
                    Message::Text(t) => t.into_bytes(),
                    _ => continue,
                };
                if let Some(tx) = pending_reply.take() {
                    let _ = tx.send(response_bytes);
                }
            }
            else => break,
        }
    }

    if let Some(name) = tunnel_name {
        let mut registry = state.registry.write().await;
        registry.remove(&name);
    }
}

/// Proxy: /tunnel/:name/* -> forward to tunnel `name`.
async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
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
    let registry = state.registry.read().await;
    let Some(tunnel_tx) = registry.get(&name).cloned() else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("tunnel not found: {}", name)))
            .unwrap();
    };
    drop(registry);

    let (parts, body) = req.into_parts();
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let req_bytes = build_http_request(&parts, &name, &backend_path_owned, body).await;
    if tunnel_tx.send((req_bytes, reply_tx)).await.is_err() {
        return Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from("tunnel disconnected"))
            .unwrap();
    }

    let response_bytes = match reply_rx.await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("tunnel timeout or closed"))
                .unwrap();
        }
    };

    parse_http_response(response_bytes)
}

/// Build raw HTTP request bytes to send to the client (for forwarding to localhost).
async fn build_http_request(
    parts: &http::request::Parts,
    _tunnel_name: &str,
    backend_path: &str,
    body: Body,
) -> Vec<u8> {
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

/// Parse raw HTTP response bytes into an axum Response.
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
