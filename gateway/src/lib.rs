//! Tunex Gateway: modular monolith.
//!
//! Modules:
//! - **app** – Application state and router assembly
//! - **config** – Configuration from environment
//! - **db** – Database (SeaORM entities, migrations, connection)
//! - **registry** – In-memory tunnel registry
//! - **api** – WebSocket and HTTP API handlers
//! - **proxy** – Tunnel proxy (forward /tunnel/:name to client)

pub mod api;
pub mod app;
pub mod config;
pub mod db;
pub mod proxy;
pub mod registry;

pub use app::run;
pub use config::Config;
