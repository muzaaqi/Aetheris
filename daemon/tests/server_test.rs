use std::time::Duration;

use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::protocol::{
    ClientMessage, SendMessagePayload, ServerMessage, WorkspaceSyncPayload,
};
use aetheris_daemon::server::run_server;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn server_accepts_websocket_connection() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    let server_shutdown = shutdown.clone();
    let server_handle = tokio::spawn(async move {
        run_server(&config, event_tx, server_shutdown, addr_tx)
            .await
            .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (ws_stream, response) = connect_async(&ws_url).await.expect("Failed to connect");
    assert_eq!(response.status(), 101);

    drop(ws_stream);
    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_broadcasts_event_to_connected_client() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_handle = tokio::spawn(async move {
        run_server(&config, server_event_tx, server_shutdown, addr_tx)
            .await
            .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");

    // Broadcast a WorkspaceSync event
    let sync_payload = WorkspaceSyncPayload {
        workspace_id: 1,
        path: "/home/muzaaqi/Projects/Aetheris".into(),
        git_branch: Some("feat/daemon-core-scaffold".into()),
        skills_count: 5,
        mcp_servers: vec!["context7".into()],
    };
    event_tx
        .send(ServerMessage::WorkspaceSync(sync_payload.clone()))
        .unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for message")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
        assert_eq!(server_msg, ServerMessage::WorkspaceSync(sync_payload));
    } else {
        panic!("Expected text message, got {:?}", msg);
    }

    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_handles_client_disconnect_gracefully() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_handle = tokio::spawn(async move {
        run_server(&config, server_event_tx, server_shutdown, addr_tx)
            .await
            .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    // Client 1 connects and immediately drops
    let (ws_stream_1, _) = connect_async(&ws_url)
        .await
        .expect("Client 1 failed to connect");
    drop(ws_stream_1);

    // Give server a brief moment to observe disconnect
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client 2 connects and should work normally
    let (mut ws_stream_2, _) = connect_async(&ws_url)
        .await
        .expect("Client 2 failed to connect");

    event_tx
        .send(ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
            workspace_id: 2,
            path: "/tmp/test".into(),
            git_branch: None,
            skills_count: 0,
            mcp_servers: vec![],
        }))
        .unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), ws_stream_2.next())
        .await
        .expect("Timeout waiting for message")
        .expect("Stream ended")
        .expect("WebSocket error");

    assert!(matches!(msg, Message::Text(_)));

    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_handles_malformed_client_message() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_handle = tokio::spawn(async move {
        run_server(&config, server_event_tx, server_shutdown, addr_tx)
            .await
            .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");

    // Send malformed JSON
    ws_stream
        .send(Message::Text("}{broken".into()))
        .await
        .unwrap();

    // Verify connection remains alive by receiving a broadcast
    tokio::time::sleep(Duration::from_millis(50)).await;
    event_tx
        .send(ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
            workspace_id: 3,
            path: "/tmp/alive".into(),
            git_branch: None,
            skills_count: 0,
            mcp_servers: vec![],
        }))
        .unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for message")
        .expect("Stream ended")
        .expect("WebSocket error");

    assert!(matches!(msg, Message::Text(_)));

    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_echoes_send_message_as_token_stream() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_handle = tokio::spawn(async move {
        run_server(&config, server_event_tx, server_shutdown, addr_tx)
            .await
            .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");

    let client_msg = ClientMessage::SendMessage(SendMessagePayload {
        text: "Hello Aetheris".into(),
        attachments: vec![],
        quote: None,
    });
    let client_json = serde_json::to_string(&client_msg).unwrap();

    ws_stream
        .send(Message::Text(client_json.into()))
        .await
        .unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for echo")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
        if let ServerMessage::TokenStream(token_payload) = server_msg {
            assert_eq!(token_payload.token, "Hello Aetheris");
            assert!(token_payload.done);
        } else {
            panic!("Expected TokenStream, got {:?}", server_msg);
        }
    } else {
        panic!("Expected text message, got {:?}", msg);
    }

    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_fails_to_bind_when_port_in_use() {
    let existing_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = existing_listener.local_addr().unwrap().port();

    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, _addr_rx) = oneshot::channel();

    let result = run_server(&config, event_tx, shutdown, addr_tx).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        aetheris_daemon::error::AetherisError::Io(_)
    ));
}
