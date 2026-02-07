//! Shared protocol types for Tunex gateway and client.

use serde::{Deserialize, Serialize};

/// Service type for the tunnel (HTTP or raw TCP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceType {
    Http,
    Tcp,
}

/// Client sends this to register a tunnel with the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub local_port: u16,
    pub service_type: ServiceType,
    pub tunnel_name: String,
}

/// Gateway responds with tunnel id and public URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub tunnel_id: String,
    pub public_url: String,
}

/// HTTP POST /register body (gateway connects to client scenario).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRegisterRequest {
    pub tunnel_name: String,
    pub client_address: String,
    /// Optional token for future auth; ignored in MVP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// HTTP POST /register response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRegisterResponse {
    pub public_url: String,
}

/// Raw HTTP request bytes (method, path, headers, body) sent gateway -> client.
pub type HttpRequestBytes = Vec<u8>;

/// Raw HTTP response bytes sent client -> gateway.
pub type HttpResponseBytes = Vec<u8>;

// Future frame structure (roadmap): [stream_id][type][length][payload]
// Placeholder for multiplexing support.
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TunnelFrame {
//     pub stream_id: u32,
//     pub frame_type: u8,
//     pub length: u32,
//     pub payload: Vec<u8>,
// }
