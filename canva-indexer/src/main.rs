mod api;
mod config;
mod db;
mod pixel;
mod watcher;

use anyhow::Result;
use pubky::{Pubky, PubkyHttpClient};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());

    let config = config::Config::load(std::path::Path::new(&config_path))?;
    info!("Loaded config from {config_path}");

    // Open database
    let database = db::open(&config.database.path)?;
    db::set_initial_size(&database, config.canvas.initial_size)?;
    info!("Database opened at {}", config.database.path);

    // Initialize Pubky client (mainnet)
    let client = PubkyHttpClient::new()?;
    let pubky = Arc::new(Pubky::with_client(client));
    info!("Pubky client initialized");

    // SSE broadcast channel
    let (sse_tx, _) = broadcast::channel::<watcher::SseEvent>(256);

    // Shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Build API
    let app_state = api::AppState {
        db: database.clone(),
        pubky: pubky.clone(),
        config: config.clone(),
        sse_tx: sse_tx.clone(),
    };
    let app = api::router(app_state);

    let listen_addr = config.server.listen.clone();
    let api_shutdown_rx = shutdown_rx.clone();

    // Run API server and watcher concurrently
    let api_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
        info!("API server listening on {listen_addr}");
        let mut rx = api_shutdown_rx;
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = rx.changed().await;
            })
            .await?;
        Ok::<(), anyhow::Error>(())
    });

    let watcher_handle = tokio::spawn(watcher::run(
        database,
        pubky,
        config,
        sse_tx,
        shutdown_rx,
    ));

    // Wait for Ctrl-C
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received");
    let _ = shutdown_tx.send(true);

    // Wait for tasks
    let _ = api_handle.await;
    let _ = watcher_handle.await;

    info!("Shutdown complete");
    Ok(())
}
