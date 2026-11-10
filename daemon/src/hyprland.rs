//! Hyprland IPC event listener for tracking workspaces and active windows.
//!
//! Subscribes to Hyprland's socket2 IPC via `hyprland-rs` and streams workspace
//! transitions to the broadcast channel with zero CPU polling.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use hyprland::data::Client;
use hyprland::event_listener::AsyncEventListener;
use hyprland::shared::HyprDataActiveOptional;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::protocol::{ServerMessage, WorkspaceSyncPayload};

/// Resolves the current working directory of a process by inspecting `/proc/{pid}/cwd`.
pub fn resolve_window_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

/// Detects the active git branch for a directory by inspecting `.git/HEAD`.
///
/// Returns:
/// - `Some("branch_name")` if `.git/HEAD` points to `ref: refs/heads/branch_name`.
/// - `Some("0123456")` (first 7 chars) if detached HEAD.
/// - `None` if the directory is not a git repository or `.git/HEAD` is missing.
pub fn detect_git_branch(path: &Path) -> Option<String> {
    let head_path = path.join(".git/HEAD");
    let content = std::fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();

    if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
        Some(branch.to_string())
    } else if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(trimmed[..7].to_string())
    } else {
        None
    }
}

/// Runs the Hyprland event listener loop until shutdown is signaled.
///
/// Dispatches `WorkspaceSync` messages over `event_tx` whenever the active
/// workspace or active window changes in Hyprland, and synchronizes the active
/// workspace path with `active_workspace`.
pub async fn run_hyprland_listener(
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    active_workspace: Arc<RwLock<Option<PathBuf>>>,
) -> Result<()> {
    let mut listener = AsyncEventListener::new();

    let ws_tx = event_tx.clone();
    let ws_active_ws = active_workspace.clone();
    listener.add_workspace_changed_handler(move |data| {
        let ws_tx = ws_tx.clone();
        let ws_active_ws = ws_active_ws.clone();
        Box::pin(async move {
            let ws_id = data.id;
            tracing::info!("Hyprland workspace changed: id={ws_id}");

            // Try to resolve focused client on the new workspace
            let (path_str, branch) = if let Ok(Some(client)) = Client::get_active_async().await
                && client.pid > 0
                && let Some(cwd) = resolve_window_cwd(client.pid as u32)
            {
                let path_str = cwd.to_string_lossy().into_owned();
                let branch = detect_git_branch(&cwd);
                *ws_active_ws.write().await = Some(cwd);
                (path_str, branch)
            } else {
                let ws_guard = ws_active_ws.read().await;
                let path_str = ws_guard
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| data.name.to_string());
                let branch = ws_guard.as_ref().and_then(|p| detect_git_branch(p));
                (path_str, branch)
            };

            let sync_msg = ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
                workspace_id: ws_id,
                path: path_str,
                git_branch: branch,
                skills_count: 0,
                mcp_servers: vec![],
            });

            if let Err(e) = ws_tx.send(sync_msg) {
                tracing::debug!("No active WebSocket subscribers for workspace event: {e}");
            }
        })
    });

    let win_tx = event_tx.clone();
    let win_active_ws = active_workspace.clone();
    listener.add_active_window_changed_handler(move |data| {
        let win_tx = win_tx.clone();
        let win_active_ws = win_active_ws.clone();
        Box::pin(async move {
            if let Some(win) = data {
                tracing::info!(
                    "Hyprland active window changed: class={}, title={}",
                    win.class,
                    win.title
                );

                if let Ok(Some(client)) = Client::get_active_async().await {
                    if client.pid <= 0 {
                        return;
                    }
                    let pid = client.pid as u32;
                    if let Some(cwd) = resolve_window_cwd(pid) {
                        let path_str = cwd.to_string_lossy().into_owned();
                        let branch = detect_git_branch(&cwd);
                        tracing::info!(
                            "Resolved active window cwd: {path_str} (branch: {branch:?})"
                        );

                        // Update shared active workspace for RPC search operations
                        *win_active_ws.write().await = Some(cwd.clone());

                        let sync_msg = ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
                            workspace_id: client.workspace.id,
                            path: path_str,
                            git_branch: branch,
                            skills_count: 0,
                            mcp_servers: vec![],
                        });

                        if let Err(e) = win_tx.send(sync_msg) {
                            tracing::debug!(
                                "No active WebSocket subscribers for window event: {e}"
                            );
                        }
                    }
                }
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
