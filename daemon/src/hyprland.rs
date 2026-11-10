//! Hyprland IPC event listener for tracking workspaces and active windows.
//!
//! Subscribes to Hyprland's socket2 IPC via `hyprland-rs` and streams workspace
//! transitions to the broadcast channel with zero CPU polling.

use hyprland::event_listener::AsyncEventListener;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::protocol::{ServerMessage, WorkspaceSyncPayload};

/// Runs the Hyprland event listener loop until shutdown is signaled.
///
/// Dispatches `WorkspaceSync` messages over `event_tx` whenever the active
/// workspace or active window changes in Hyprland.
pub async fn run_hyprland_listener(
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
) -> Result<()> {
    let mut listener = AsyncEventListener::new();

    let ws_tx = event_tx.clone();
    listener.add_workspace_changed_handler(move |data| {
        let ws_tx = ws_tx.clone();
        Box::pin(async move {
            let ws_id = data.id;
            let ws_name = data.name.to_string();
            tracing::info!("Hyprland workspace changed: id={ws_id}, name={ws_name}");

            let sync_msg = ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
                workspace_id: ws_id,
                path: ws_name,
                git_branch: None,
                skills_count: 0,
                mcp_servers: vec![],
            });

            if let Err(e) = ws_tx.send(sync_msg) {
                tracing::debug!("No active WebSocket subscribers for workspace event: {e}");
            }
        })
    });

    listener.add_active_window_changed_handler(|data| {
        Box::pin(async move {
            if let Some(win) = data {
                tracing::info!(
                    "Hyprland active window changed: class={}, title={}",
                    win.class,
                    win.title
                );
                // Window path resolution from PID is slated for Phase 2
            }
        })
    });

    tracing::info!("Hyprland AsyncEventListener initialized; starting socket2 loop");

    tokio::select! {
        _ = shutdown.cancelled() => {
            tracing::info!("Hyprland listener observed shutdown signal");
            Ok(())
        }
        res = listener.start_listener_async() => {
            res.map_err(Into::into)
        }
    }
}
