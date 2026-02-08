//! Application state and router assembly (modular monolith entry).

use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::api;
use crate::config::Config;
use crate::db;
use crate::proxy;
use crate::registry::Registry;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub registry: Registry,
    pub redis: redis::aio::ConnectionManager,
    pub port: u16,
    pub base_domain: String,
    pub public_scheme: String,
    pub db: sea_orm::DatabaseConnection,
}

/// Build the Axum router with all routes and shared state.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/register", axum::routing::post(api::register_handler))
        .fallback(proxy::proxy_handler)
        .with_state(state)
}

/// Run the gateway: connect DB, run migrations, Redis, bind and serve.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::connect_and_migrate(&config.database_url).await?;
    let redis_client = redis::Client::open(config.redis_url.as_str())
        .map_err(|e| format!("Redis client: {}", e))?;
    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .map_err(|e| format!("Redis connection: {}", e))?;
    let state = AppState {
        registry: Registry::new(redis.clone(), Some(config.tunnel_ttl_secs)),
        redis,
        port: config.port,
        base_domain: config.base_domain,
        public_scheme: config.public_scheme,
        db,
    };

    let app = router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;
    println!("Tunex Gateway listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
