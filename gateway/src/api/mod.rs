//! HTTP and WebSocket API: tunnel registration and WS protocol.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::app::AppState;
use tunex_common::{RegisterRequest, RegisterResponse};

/// Upgrade to WebSocket and handle tunnel protocol.
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(state, socket))
}

async fn handle_websocket(state: AppState, socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let (mut request_rx, tunnel_name) = match ws_rx.next().await {
        Some(Ok(Message::Text(text))) => {
            let reg: RegisterRequest = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(_) => {
                    let _ = ws_tx
                        .send(Message::Text("{\"error\":\"invalid RegisterRequest\"}".into()))
                        .await;
                    return;
                }
            };

            let (request_tx, request_rx_inner) = tokio::sync::mpsc::channel(1);
            let tunnel_id = format!("{:?}", std::time::SystemTime::now());
            let public_url = format!("http://localhost:{}/tunnel/{}", state.port, reg.tunnel_name);
            state.registry.register(reg.tunnel_name.clone(), request_tx).await;

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
            let _ = ws_tx
                .send(Message::Text("{\"error\":\"expected JSON registration\"}".into()))
                .await;
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
        state.registry.unregister(&name).await;
    }
}
