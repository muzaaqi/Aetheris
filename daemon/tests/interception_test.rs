use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};

use aetheris_daemon::interception::{ApprovalRegistry, ToolInterceptor};
use aetheris_daemon::protocol::{ExecutionMode, ServerMessage};

#[tokio::test]
async fn safe_mode_blocks_until_approved() {
    let (event_tx, mut event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry.clone(), event_tx);

    let interceptor_handle = tokio::spawn(async move {
        interceptor
            .evaluate_tool_call(
                "call_1",
                "run_command",
                serde_json::json!({ "command": "ls" }),
                None,
                Duration::from_secs(5),
            )
            .await
    });

    // Verify broadcast event was received
    let event = event_rx.recv().await.unwrap();
    match event {
        ServerMessage::ToolCallRequest(payload) => {
            assert_eq!(payload.tool_call_id, "call_1");
            assert_eq!(payload.tool_name, "run_command");
            assert!(payload.requires_approval);
        }
        _ => panic!("Expected ToolCallRequest"),
    }

    // Resolve approval
    assert!(registry.resolve("call_1", true));

    let result = interceptor_handle.await.unwrap().unwrap();
    assert!(result, "Expected tool execution to be approved");
}

#[tokio::test]
async fn safe_mode_blocks_and_rejects_when_denied() {
    let (event_tx, mut event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry.clone(), event_tx);

    let interceptor_handle = tokio::spawn(async move {
        interceptor
            .evaluate_tool_call(
                "call_reject",
                "write_file",
                serde_json::json!({ "path": "danger.sh" }),
                None,
                Duration::from_secs(5),
            )
            .await
    });

    let _ = event_rx.recv().await.unwrap();
    assert!(registry.resolve("call_reject", false));

    let result = interceptor_handle.await.unwrap().unwrap();
    assert!(!result, "Expected tool execution to be rejected");
}

#[tokio::test]
async fn yolo_mode_auto_approves_without_blocking() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Yolo));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry, event_tx);

    let result = interceptor
        .evaluate_tool_call(
            "call_2",
            "write_file",
            serde_json::json!({ "path": "test.txt" }),
            None,
            Duration::from_secs(1),
        )
        .await
        .unwrap();

    assert!(result, "YOLO mode must immediately approve");
}

#[tokio::test]
async fn plan_only_mode_rejects_mutating_tools() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::PlanOnly));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry, event_tx);

    let tools_to_test = [
        "run_command",
        "write_file",
        "Write",
        "Edit",
        "patch",
        "bash",
        "execute",
        "run_sql",
    ];
    for tool in tools_to_test {
        let result = interceptor
            .evaluate_tool_call(
                "call_mut",
                tool,
                serde_json::json!({ "command": "rm -rf /" }),
                None,
                Duration::from_secs(1),
            )
            .await;

        assert!(
            result.is_err(),
            "Plan-Only mode must reject mutating tool '{tool}'"
        );
    }
}

#[tokio::test]
async fn plan_only_mode_allows_read_only_tools() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::PlanOnly));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry, event_tx);

    let result = interceptor
        .evaluate_tool_call(
            "call_read",
            "read_file",
            serde_json::json!({ "path": "readme.md" }),
            None,
            Duration::from_secs(1),
        )
        .await;

    assert_eq!(result.unwrap(), true, "Read-only tools are permitted");
}

#[tokio::test]
async fn safe_mode_timeout_cleans_registry() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry.clone(), event_tx);

    let result = interceptor
        .evaluate_tool_call(
            "call_timeout",
            "run_command",
            serde_json::json!({}),
            None,
            Duration::from_millis(50),
        )
        .await;

    assert!(result.is_err(), "Expected timeout error");
    assert!(!registry.has_pending("call_timeout"));
}
