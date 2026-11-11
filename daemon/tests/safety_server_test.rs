use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, oneshot};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::interception::ApprovalRegistry;
use aetheris_daemon::protocol::{
    ClientMessage, ExecutionMode, ToolApprovalPayload, UpdateConfigPayload,
};

#[tokio::test]
async fn server_handles_mode_update_and_tool_approval() {
    let mut config = DaemonConfig::default();
    config.port = 0; // OS assigned port
    let (event_tx, _) = broadcast::channel(16);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();
    let active_ws = Arc::new(RwLock::new(None));
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());

    let srv_mode = mode.clone();
    let srv_reg = registry.clone();
    tokio::spawn(async move {
        aetheris_daemon::server::run_server_with_state(
            &config, event_tx, shutdown, addr_tx, active_ws, srv_mode, srv_reg,
        )
        .await
        .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let url = format!("ws://{addr}");
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut sink, _reader) = ws_stream.split();

    // 1. Update mode to YOLO
    let update_msg = ClientMessage::UpdateConfig(UpdateConfigPayload {
        platform: "antigravity".to_string(),
        mode: "yolo".to_string(),
        thinking: "high".to_string(),
    });
    sink.send(Message::Text(
        serde_json::to_string(&update_msg).unwrap().into(),
    ))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(*mode.read().await, ExecutionMode::Yolo);

    // 2. Register mock tool approval in Safe mode
    *mode.write().await = ExecutionMode::Safe;
    let rx = registry.register("call_test_123");

    let approval_msg = ClientMessage::ToolApproval(ToolApprovalPayload {
        tool_call_id: "call_test_123".to_string(),
        approved: true,
    });
    sink.send(Message::Text(
        serde_json::to_string(&approval_msg).unwrap().into(),
    ))
    .await
    .unwrap();

    let approved = rx.await.unwrap();
    assert!(approved);
}
