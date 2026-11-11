//! Local WebSocket server for Aetheris HUD communication.
//!
//! Listens on `127.0.0.1:21420` (or configured host:port) and serves as the
//! bidirectional JSON-RPC message conduit between Caelestia Shell and the daemon.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, oneshot};
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_util::sync::CancellationToken;

use crate::config::DaemonConfig;
use crate::error::Result;
use crate::interception::ApprovalRegistry;
use crate::protocol::{ClientMessage, ExecutionMode, ServerMessage, TokenStreamPayload};

/// Runs the local WebSocket server until shutdown is requested using default safety state.
pub async fn run_server(
    config: &DaemonConfig,
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    addr_tx: oneshot::Sender<SocketAddr>,
    active_workspace: Arc<RwLock<Option<PathBuf>>>,
) -> Result<()> {
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());
    run_server_with_state(
        config,
        event_tx,
        shutdown,
        addr_tx,
        active_workspace,
        mode,
        registry,
    )
    .await
}

/// Runs the local WebSocket server with explicitly shared execution mode and approval registry.
pub async fn run_server_with_state(
    config: &DaemonConfig,
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    addr_tx: oneshot::Sender<SocketAddr>,
    active_workspace: Arc<RwLock<Option<PathBuf>>>,
    execution_mode: Arc<RwLock<ExecutionMode>>,
    approval_registry: Arc<ApprovalRegistry>,
) -> Result<()> {
    let listener = TcpListener::bind(config.addr()).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!("Aetheris WebSocket server listening on ws://{local_addr}");

    // Announce the bound address (used by tests and logging)
    let _ = addr_tx.send(local_addr);

    let ws_config = WebSocketConfig::default().max_frame_size(Some(config.max_frame_size));

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("WebSocket server accept loop received shutdown signal");
                break;
            }
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((stream, peer_addr)) => {
                        tracing::debug!("Accepted TCP connection from {peer_addr}");
                        let client_shutdown = shutdown.clone();
                        let client_event_tx = event_tx.clone();
                        let client_ws_config = ws_config;
                        let client_active_ws = active_workspace.clone();
                        let client_mode = execution_mode.clone();
                        let client_registry = approval_registry.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_client(
                                stream,
                                peer_addr,
                                client_event_tx,
                                client_shutdown,
                                client_ws_config,
                                client_active_ws,
                                client_mode,
                                client_registry,
                            ).await {
                                tracing::debug!("Client {peer_addr} disconnected or error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to accept incoming TCP connection: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handles an individual WebSocket client session.
#[allow(clippy::too_many_arguments)]
async fn handle_client(
    stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    ws_config: WebSocketConfig,
    active_workspace: Arc<RwLock<Option<PathBuf>>>,
    execution_mode: Arc<RwLock<ExecutionMode>>,
    approval_registry: Arc<ApprovalRegistry>,
) -> Result<()> {
    let ws_stream = accept_async_with_config(stream, Some(ws_config)).await?;
    tracing::debug!("WebSocket handshake completed for {peer_addr}");

    let (mut sink, mut reader) = ws_stream.split();
    let mut event_rx = event_tx.subscribe();

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::debug!("Client handler for {peer_addr} observed shutdown");
                break;
            }
            broadcast_msg = event_rx.recv() => {
                match broadcast_msg {
                    Ok(server_msg) => {
                        match serde_json::to_string(&server_msg) {
                            Ok(json) => {
                                if let Err(e) = sink.send(Message::Text(json.into())).await {
                                    tracing::debug!("Failed to send message to {peer_addr}: {e}");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize server message: {e}");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Client {peer_addr} lagged {n} broadcast messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::debug!("Broadcast channel closed; terminating client loop");
                        break;
                    }
                }
            }
            ws_item = reader.next() => {
                match ws_item {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::SendMessage(payload)) => {
                                tracing::info!("Received SendMessage from {peer_addr}: {}", payload.text);
                                // Echo token stream response
                                let echo = ServerMessage::TokenStream(TokenStreamPayload {
                                    token: payload.text,
                                    done: true,
                                });
                                match serde_json::to_string(&echo) {
                                    Ok(json) => {
                                        if let Err(e) = sink.send(Message::Text(json.into())).await {
                                            tracing::debug!("Failed to send echo response to {peer_addr}: {e}");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to serialize echo message: {e}");
                                    }
                                }
                            }
                            Ok(ClientMessage::ToolApproval(payload)) => {
                                tracing::debug!("Received ToolApproval from {peer_addr}: id={}, approved={}", payload.tool_call_id, payload.approved);
                                let resolved = approval_registry.resolve(&payload.tool_call_id, payload.approved);
                                tracing::debug!("Tool approval for {} resolved: {}", payload.tool_call_id, resolved);
                            }
                            Ok(ClientMessage::UpdateConfig(payload)) => {
                                tracing::debug!("Received UpdateConfig from {peer_addr}: platform={}, mode={}, thinking={}", payload.platform, payload.mode, payload.thinking);
                                if let Ok(new_mode) = ExecutionMode::from_str(&payload.mode) {
                                    *execution_mode.write().await = new_mode;
                                    tracing::info!("Updated daemon execution mode to: {new_mode:?}");
                                }
                            }
                            Ok(ClientMessage::SearchFiles(payload)) => {
                                tracing::debug!("Received SearchFiles from {peer_addr}: query={}", payload.query);
                                let current_dir = {
                                    let ws_guard = active_workspace.read().await;
                                    ws_guard.clone().unwrap_or_else(|| {
                                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                                    })
                                };
                                let root = crate::context::find_workspace_root(&current_dir);
                                let limit = payload.limit.unwrap_or(20);
                                match crate::context::search_files(&root, &payload.query, limit).await {
                                    Ok(matches) => {
                                        let resp = ServerMessage::FileResults(crate::protocol::FileResultsPayload {
                                            query: payload.query,
                                            matches,
                                        });
                                        if let Ok(json) = serde_json::to_string(&resp)
                                            && let Err(e) = sink.send(Message::Text(json.into())).await
                                        {
                                            tracing::debug!("Failed to send FileResults to {peer_addr}: {e}");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("SearchFiles failed for {peer_addr}: {e}");
                                    }
                                }
                            }
                            Ok(ClientMessage::ListSkills) => {
                                tracing::debug!("Received ListSkills from {peer_addr}");
                                let current_dir = {
                                    let ws_guard = active_workspace.read().await;
                                    ws_guard.clone().unwrap_or_else(|| {
                                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                                    })
                                };
                                let root = crate::context::find_workspace_root(&current_dir);
                                match crate::context::scan_skills(Some(&root)).await {
                                    Ok(skills) => {
                                        let resp = ServerMessage::SkillsList(crate::protocol::SkillsListPayload { skills });
                                        if let Ok(json) = serde_json::to_string(&resp)
                                            && let Err(e) = sink.send(Message::Text(json.into())).await
                                        {
                                            tracing::debug!("Failed to send SkillsList to {peer_addr}: {e}");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("ListSkills failed for {peer_addr}: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Received invalid ClientMessage JSON from {peer_addr}: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = sink.send(Message::Pong(data)).await {
                            tracing::debug!("Failed to send pong to {peer_addr}: {e}");
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::debug!("Client {peer_addr} closed connection");
                        break;
                    }
                    Some(Ok(_)) => {
                        // Binary or Pong messages ignored
                    }
                    Some(Err(e)) => {
                        tracing::debug!("WebSocket read error from {peer_addr}: {e}");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
