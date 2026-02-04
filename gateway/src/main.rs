//! Tunex Gateway binary: load config and run the server.

use tunex_gateway::{run, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    run(config).await
}
