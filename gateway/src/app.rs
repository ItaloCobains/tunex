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
    pub port: u16,
    pub db: sea_orm::DatabaseConnection,
}

/// Build the Axum router with all routes and shared state.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", axum::routing::get(api::ws_handler))
        .fallback(proxy::proxy_handler)
        .with_state(state)
}

/// Run the gateway: connect DB, run migrations, bind and serve.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::connect_and_migrate(&config.database_url).await?;
    let state = AppState {
        registry: Registry::new(),
        port: config.port,
        db,
    };

    let app = router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;
    println!("Tunex Gateway listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
