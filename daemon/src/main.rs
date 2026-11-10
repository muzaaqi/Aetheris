//! Aetheris Daemon
//! OS-Level AI Agent HUD background service for Hyprland and Caelestia Shell.

pub mod adapters;
pub mod config;
pub mod context;
pub mod error;
pub mod hyprland;
pub mod protocol;
pub mod server;
pub mod transcript;

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
    let active_workspace = std::sync::Arc::new(tokio::sync::RwLock::new(None));

    // Spawn server task
    let server_config = config.clone();
    let server_event_tx = event_tx.clone();
    let server_shutdown = shutdown.clone();
    let server_active_ws = active_workspace.clone();
    let mut server_task = tokio::spawn(async move {
        run_server(
            &server_config,
            server_event_tx,
            server_shutdown,
            addr_tx,
            server_active_ws,
        )
        .await
    });

    // Spawn Hyprland listener task
    let hypr_event_tx = event_tx.clone();
    let hypr_shutdown = shutdown.clone();
    let hypr_active_ws = active_workspace.clone();
    let mut hypr_task = tokio::spawn(async move {
        match run_hyprland_listener(hypr_event_tx, hypr_shutdown, hypr_active_ws).await {
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

    // Initialize Transcript Watcher for Antigravity session streaming
    let transcript_event_tx = event_tx.clone();
    let transcript_shutdown = shutdown.clone();
    let mut transcript_task = tokio::spawn(async move {
        let mut watcher = crate::transcript::TranscriptWatcher::new(transcript_event_tx);
        if let Some(home) = std::env::var_os("HOME") {
            let brain_dir = std::path::PathBuf::from(home).join(".gemini/antigravity/brain");
            if brain_dir.exists() {
                if let Some(latest) = crate::transcript::find_latest_transcript(&brain_dir) {
                    tracing::info!(
                        "Monitoring active Antigravity session at {}",
                        latest.display()
                    );
                    let _ = watcher.watch_file(latest).await;
                } else {
                    tracing::info!(
                        "Monitoring Antigravity brain dir at {}",
                        brain_dir.display()
                    );
                    let _ = watcher.watch_file(brain_dir).await;
                }
            }
        }
        transcript_shutdown.cancelled().await;
    });

    // Log bound address once available
    if let Ok(addr) = addr_rx.await {
        tracing::info!("Aetheris ready for connections at ws://{addr}");
    }

    let mut server_err = None;

    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    // Wait for shutdown signal (Ctrl+C, SIGTERM) or server termination
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C interrupt; initiating graceful shutdown");
            shutdown.cancel();
        }
        _ = async {
            #[cfg(unix)]
            { sigterm.recv().await }
            #[cfg(not(unix))]
            { std::future::pending::<()>().await }
        } => {
            tracing::info!("Received termination signal; initiating graceful shutdown");
            shutdown.cancel();
        }
        res = &mut server_task => {
            shutdown.cancel();
            match res {
                Ok(Ok(())) => {
                    tracing::info!("Server task completed successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!("Server task failed: {e}");
                    server_err = Some(e);
                }
                Err(join_err) => {
                    tracing::error!("Server task panicked: {join_err}");
                }
            }
        }
    }

    // Gracefully drain all tasks within a bounded timeout
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        let _ = tokio::join!(&mut server_task, &mut hypr_task, &mut transcript_task);
    })
    .await;

    tracing::info!("Aetheris daemon stopped.");

    if let Some(err) = server_err {
        return Err(err);
    }

    Ok(())
}
