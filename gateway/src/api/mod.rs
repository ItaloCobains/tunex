//! HTTP API: tunnel registration (POST /register).

use axum::{extract::State, Json};
use http::StatusCode;
use rand::Rng;

use crate::app::AppState;
use tunex_common::HttpRegisterRequest;

const SUBDOMAIN_LEN: usize = 10;
const SUBDOMAIN_MAX_RETRIES: u32 = 5;

type ApiResponse = (StatusCode, Json<serde_json::Value>);

fn err_json(msg: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({"error": msg}))
}

/// POST /register: register a tunnel (client address) in Redis.
/// When tunnel_id is absent, gateway generates a new subdomain; when present (heartbeat), refreshes that tunnel's TTL.
pub async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<HttpRegisterRequest>,
) -> ApiResponse {
    if !parse_client_address(&body.client_address) {
        return (StatusCode::BAD_REQUEST, err_json("client_address must be host:port"));
    }

    let tunnel_id = match &body.tunnel_id {
        None => {
            match generate_and_register_subdomain(&state, &body.client_address).await {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Redis register error: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        err_json("registration failed"),
                    );
                }
            }
        }
        Some(id) => {
            if !is_valid_subdomain(id) {
                return (
                    StatusCode::BAD_REQUEST,
                    err_json("tunnel_id must be a valid subdomain (a-z, 0-9, hyphen)"),
                );
            }
            if let Err(e) = state.registry.register(id.clone(), body.client_address.clone()).await {
                eprintln!("Redis register error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    err_json("registration failed"),
                );
            }
            id.clone()
        }
    };

    let public_url = build_public_url(&state.public_scheme, &tunnel_id, &state.base_domain, state.port);
    println!(
        "Tunnel registered: {} -> {}",
        tunnel_id, body.client_address
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "public_url": public_url,
            "tunnel_id": tunnel_id
        })),
    )
}

fn parse_client_address(addr: &str) -> bool {
    let parts: Vec<&str> = addr.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[1].parse::<u16>().is_ok()
}

/// Subdomain must be 1-63 chars, only a-z, 0-9, hyphen (not at start or end).
fn is_valid_subdomain(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() > 63 {
        return false;
    }
    if s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Generate one random alphanumeric lowercase subdomain (sync; no await so no Send issues).
fn generate_subdomain_id() -> String {
    let mut rng = rand::thread_rng();
    (0..SUBDOMAIN_LEN)
        .map(|_| {
            let n = rng.gen_range(0..36u8);
            if n < 10 {
                (b'0' + n) as char
            } else {
                (b'a' + (n - 10)) as char
            }
        })
        .collect()
}

/// Generate a random alphanumeric lowercase subdomain and register in Redis. Retries on collision.
async fn generate_and_register_subdomain(
    state: &AppState,
    client_address: &str,
) -> Result<String, redis::RedisError> {
    for _ in 0..SUBDOMAIN_MAX_RETRIES {
        let id = generate_subdomain_id();
        if state.registry.get(&id).await?.is_some() {
            continue;
        }
        state.registry.register(id.clone(), client_address.to_string()).await?;
        return Ok(id);
    }
    let id = generate_subdomain_id();
    state.registry.register(id.clone(), client_address.to_string()).await?;
    Ok(id)
}

fn build_public_url(scheme: &str, tunnel_id: &str, base_domain: &str, port: u16) -> String {
    let host = format!("{}.{}", tunnel_id, base_domain);
    let default_port = if scheme == "https" { 443 } else { 80 };
    if port == default_port {
        format!("{}://{}", scheme, host)
    } else {
        format!("{}://{}:{}", scheme, host, port)
    }
}
