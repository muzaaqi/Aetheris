//! Aetheris Daemon
//! OS-Level AI Agent HUD background service for Hyprland and Caelestia Shell.

pub mod config;
pub mod error;
pub mod hyprland;
pub mod protocol;
pub mod server;

use std::time::Duration;

use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::config::DaemonConfig;
use crate::error::Result;
use crate::hyprland::run_hyprland_listener;
use crate::server::run_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber with env filter (default: info for aetheris)
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("aetheris_daemon=info,tokio=warn"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    tracing::info!("Starting Aetheris daemon...");

    let config = DaemonConfig::default();
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    // Spawn server task
    let server_config = config.clone();
    let server_event_tx = event_tx.clone();
    let server_shutdown = shutdown.clone();
    let server_task = tokio::spawn(async move {
        run_server(&server_config, server_event_tx, server_shutdown, addr_tx).await
    });

    // Spawn Hyprland listener task
    let hypr_event_tx = event_tx.clone();
    let hypr_shutdown = shutdown.clone();
    let hypr_task = tokio::spawn(async move {
        match run_hyprland_listener(hypr_event_tx, hypr_shutdown).await {
            Ok(()) => {
                tracing::info!("Hyprland event listener exited cleanly");
            }
            Err(e) => {
                // Review Focus #3: Hyprland socket absent / failed
                tracing::warn!(
                    "Hyprland listener failed: {e}. Workspace tracking disabled, continuing WebSocket server."
                );
            }
        }
    });

    // Log bound address once available
    if let Ok(addr) = addr_rx.await {
        tracing::info!("Aetheris ready for connections at ws://{addr}");
    }

    // Wait for shutdown signal (Ctrl+C) or server termination
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C interrupt; initiating graceful shutdown");
            shutdown.cancel();
        }
        res = server_task => {
            tracing::info!("Server task finished: {:?}", res);
            shutdown.cancel();
        }
    }

    // Await Hyprland task with a short timeout for graceful drain
    let _ = tokio::time::timeout(Duration::from_secs(2), hypr_task).await;

    tracing::info!("Aetheris daemon stopped.");
    Ok(())
}
