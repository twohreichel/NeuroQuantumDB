use anyhow::Result;
use neuroquantum_api::{init_observability, ApiConfig, ApiServer};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = ApiConfig::load()?;

    // Validate configuration
    config.validate()?;

    // Initialize observability (logging and metrics)
    init_observability(&config)?;

    info!(
        "Starting NeuroQuantumDB API Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    info!(
        "Configuration loaded - Host: {}, Port: {}, Workers: {}",
        config.server.host, config.server.port, config.server.workers
    );

    // Create and start the API server
    let server = ApiServer::new(config);
    server.start().await?;

    Ok(())
}
