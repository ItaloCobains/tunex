//! Application configuration (env-based).

const DEFAULT_BASE_DOMAIN: &str = "localhost";
const DEFAULT_DATABASE_URL: &str = "postgres://tunex:tunex@localhost:5434/tunex";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_PUBLIC_SCHEME: &str = "http";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379";
const DEFAULT_TUNNEL_TTL_SECS: u64 = 60;

/// Gateway configuration loaded from environment.
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub redis_url: String,
    /// TTL in seconds for tunnel entries in Redis; client must heartbeat before expiry.
    pub tunnel_ttl_secs: u64,
    /// Base domain for subdomain extraction and public URL (e.g. localhost or tunnex.com).
    pub base_domain: String,
    /// Scheme for public URL returned on register (http or https).
    pub public_scheme: String,
    /// Reserved for future: require auth on tunnel registration.
    #[allow(dead_code)]
    pub auth_required: bool,
}

impl Config {
    /// Load config from environment variables with defaults for local dev.
    pub fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_PORT);
        let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
        let tunnel_ttl_secs = std::env::var("TUNEX_TUNNEL_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TUNNEL_TTL_SECS);
        let auth_required = std::env::var("TUNEX_AUTH_REQUIRED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);
        let base_domain = std::env::var("TUNEX_BASE_DOMAIN")
            .unwrap_or_else(|_| DEFAULT_BASE_DOMAIN.to_string());
        let public_scheme = std::env::var("TUNEX_PUBLIC_SCHEME")
            .unwrap_or_else(|_| DEFAULT_PUBLIC_SCHEME.to_string());
        Self {
            database_url,
            port,
            redis_url,
            tunnel_ttl_secs,
            base_domain,
            public_scheme,
            auth_required,
        }
    }
}
