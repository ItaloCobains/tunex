//! HTTP API: tunnel registration (POST /register).

use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;

use crate::app::AppState;
use tunex_common::{HttpRegisterRequest, HttpRegisterResponse};

/// POST /register: register a tunnel (client address) in Redis. Optional Authorization header
/// or token field are accepted and ignored in MVP to not block future auth.
pub async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<HttpRegisterRequest>,
) -> impl IntoResponse {
    // Optional auth: Authorization header or body.token accepted and ignored in MVP.

    if body.tunnel_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "tunnel_name must be non-empty"})),
        )
            .into_response();
    }
    if !parse_client_address(&body.client_address) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "client_address must be host:port"})),
        )
            .into_response();
    }

    match state
        .registry
        .register(body.tunnel_name.clone(), body.client_address.clone())
        .await
    {
        Ok(()) => {
            let public_url = format!("http://localhost:{}/tunnel/{}", state.port, body.tunnel_name);
            println!(
                "Tunnel registered: {} -> {}",
                body.tunnel_name, body.client_address
            );
            (
                StatusCode::OK,
                Json(HttpRegisterResponse { public_url }),
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("Redis register error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "registration failed"})),
            )
                .into_response()
        }
    }
}

fn parse_client_address(addr: &str) -> bool {
    let parts: Vec<&str> = addr.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[1].parse::<u16>().is_ok()
}
