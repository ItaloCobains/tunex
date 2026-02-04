//! Application configuration (env-based).

const DEFAULT_DATABASE_URL: &str = "postgres://tunex:tunex@localhost:5434/tunex";
const DEFAULT_PORT: u16 = 8080;

/// Gateway configuration loaded from environment.
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
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
        Self {
            database_url,
            port,
        }
    }
}
