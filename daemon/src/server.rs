//! Local WebSocket server for Aetheris HUD communication.
//!
//! Listens on `127.0.0.1:21420` (or configured host:port) and serves as the
//! bidirectional JSON-RPC message conduit between Caelestia Shell and the daemon.

use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_util::sync::CancellationToken;

use crate::config::DaemonConfig;
use crate::error::Result;
use crate::protocol::{ClientMessage, ServerMessage, TokenStreamPayload};

/// Runs the local WebSocket server until shutdown is requested.
///
/// Binds to the configured address, announces the bound socket address via
/// `addr_tx`, and accepts client connections in an event-driven async loop.
pub async fn run_server(
    config: &DaemonConfig,
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    addr_tx: oneshot::Sender<SocketAddr>,
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

                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, peer_addr, client_event_tx, client_shutdown, client_ws_config).await {
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
async fn handle_client(
    stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    event_tx: broadcast::Sender<ServerMessage>,
    shutdown: CancellationToken,
    ws_config: WebSocketConfig,
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
                                // Phase 1 Echo MVP: stream single completed token chunk back
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
                            }
                            Ok(ClientMessage::UpdateConfig(payload)) => {
                                tracing::debug!("Received UpdateConfig from {peer_addr}: platform={}, mode={}, thinking={}", payload.platform, payload.mode, payload.thinking);
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
