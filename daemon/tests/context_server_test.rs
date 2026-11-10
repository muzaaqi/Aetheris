use std::sync::Arc;
use std::time::Duration;

use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::protocol::{ClientMessage, SearchFilesPayload, ServerMessage};
use aetheris_daemon::server::run_server;
use futures_util::{SinkExt, StreamExt};
use tempfile::tempdir;
use tokio::sync::{RwLock, broadcast, oneshot};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn server_responds_to_search_files_and_list_skills() {
    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();
    let active_ws = Arc::new(RwLock::new(None));

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_ws = active_ws.clone();
    let server_handle = tokio::spawn(async move {
        run_server(
            &config,
            server_event_tx,
            server_shutdown,
            addr_tx,
            server_ws,
        )
        .await
        .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");

    // 1. Send SearchFiles
    let search_msg = ClientMessage::SearchFiles(SearchFilesPayload {
        query: "Cargo".into(),
        limit: Some(5),
    });
    let search_json = serde_json::to_string(&search_msg).unwrap();
    ws_stream
        .send(Message::Text(search_json.into()))
        .await
        .unwrap();

    let resp_msg = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("Timeout waiting for SearchFiles response")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = resp_msg {
        let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
        match server_msg {
            ServerMessage::FileResults(payload) => {
                assert_eq!(payload.query, "Cargo");
                assert!(!payload.matches.is_empty());
                assert!(
                    payload
                        .matches
                        .iter()
                        .any(|m| m.path.contains("Cargo.toml"))
                );
            }
            _ => panic!("Expected FileResults, got {:?}", server_msg),
        }
    } else {
        panic!("Expected text message");
    }

    // 2. Send ListSkills
    let skills_msg = ClientMessage::ListSkills;
    let skills_json = serde_json::to_string(&skills_msg).unwrap();
    ws_stream
        .send(Message::Text(skills_json.into()))
        .await
        .unwrap();

    let resp_msg2 = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("Timeout waiting for ListSkills response")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = resp_msg2 {
        let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
        match server_msg {
            ServerMessage::SkillsList(payload) => {
                assert!(payload.skills.iter().any(|s| s.name.contains("rust")));
            }
            _ => panic!("Expected SkillsList, got {:?}", server_msg),
        }
    } else {
        panic!("Expected text message");
    }

    shutdown.cancel();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn server_searches_explicit_active_workspace() {
    let dir = tempdir().unwrap();
    let ws_path = dir.path();
    std::fs::write(ws_path.join("special_feature_file.rs"), "fn special() {}").unwrap();

    let config = DaemonConfig {
        host: "127.0.0.1".into(),
        port: 0,
        ..Default::default()
    };
    let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
    let shutdown = CancellationToken::new();
    let (addr_tx, addr_rx) = oneshot::channel();
    let active_ws = Arc::new(RwLock::new(Some(ws_path.to_path_buf())));

    let server_shutdown = shutdown.clone();
    let server_event_tx = event_tx.clone();
    let server_ws = active_ws.clone();
    let server_handle = tokio::spawn(async move {
        run_server(
            &config,
            server_event_tx,
            server_shutdown,
            addr_tx,
            server_ws,
        )
        .await
        .unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let ws_url = format!("ws://{addr}");

    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");

    let search_msg = ClientMessage::SearchFiles(SearchFilesPayload {
        query: "special_feature".into(),
        limit: Some(5),
    });
    let search_json = serde_json::to_string(&search_msg).unwrap();
    ws_stream
        .send(Message::Text(search_json.into()))
        .await
        .unwrap();

    let resp_msg = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("Timeout waiting for SearchFiles response")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = resp_msg {
        let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
        match server_msg {
            ServerMessage::FileResults(payload) => {
                assert_eq!(payload.query, "special_feature");
                assert_eq!(payload.matches.len(), 1);
                assert!(payload.matches[0].path.contains("special_feature_file.rs"));
            }
            _ => panic!("Expected FileResults, got {:?}", server_msg),
        }
    } else {
        panic!("Expected text message");
    }

    shutdown.cancel();
    server_handle.await.unwrap();
}
