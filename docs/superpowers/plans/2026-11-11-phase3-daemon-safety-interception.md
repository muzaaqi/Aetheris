# Phase 3 (Part 1 — Daemon Safety Modes & Tool Interception) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the backend execution safety engine in the Rust daemon, featuring multi-mode execution (`Safe`, `YOLO`, `Plan-Only`), asynchronous human-in-the-loop tool interception via Tokio `oneshot` channels, unified code diff generation for file operations, and MCP server configuration discovery.

**Architecture:** 
- A shared `ApprovalRegistry` coordinates pending tool calls between async agent tasks and connected WebSocket clients using `tokio::sync::oneshot` channels.
- An `ExecutionMode` engine enforces security policies: `Safe` broadcasts `tool_call_request` and blocks until approval; `YOLO` executes immediately; `Plan-Only` intercepts and rejects state mutations.
- A `diff` module computes unified line diffs for `write_file` and `edit_file` tool requests.
- An `mcp` scanner discovers global and workspace MCP servers and populates `workspace_sync` payloads.

**Tech Stack:** Rust Edition 2024, Tokio 1.43+ (async runtime & channels), Serde / Serde JSON, Similar / Diff-match-patch algorithms, thiserror.

**Spec:** `PRODUCT.md` (Epic 4: Human-in-the-Loop Safety, Phase 3 Milestones) & `DESIGN.md` (Section 3: Human-in-the-Loop Safety Interception, Section 4: Protocol Contracts).

## Global Constraints

- Daemon memory footprint must strictly remain between **5 MB – 15 MB RAM** (AGENTS.md Rule 2).
- Zero blocking operations on tokio threads (`std::fs` banned on async paths; use `tokio::fs` or `spawn_blocking`) (AGENTS.md Rule 1).
- Zero unvetted `unsafe` blocks, zero `.unwrap()` or `.expect()` in non-test production code (AGENTS.md Rule 5).
- 100% test coverage with TDD for all new modules (`rust-testing` skill).
- Clean imperative commits adhering to Conventional Commits with zero AI attribution (AGENTS.md Rule 9 & 10).

## Review Focus

1. **Timeout on Unanswered Approvals:** In `Safe Mode`, if a user closes the HUD or ignores an approval request, the pending `oneshot` must cleanly timeout (default 300s) and reject the execution without leaking memory in `ApprovalRegistry`.
2. **Concurrent Tool Requests:** Multiple tool calls issued in parallel or in rapid succession must each receive distinct `tool_call_id`s without channel crosstalk or lock contention.
3. **Plan-Only Mutating Tool Rejections:** In `Plan-Only` mode, any mutating tools (`run_command`, `write_file`, `edit_file`, mutating MCP calls) must be intercepted and rejected with a clear descriptive message, while read-only tools are permitted.
4. **Invalid or Disconnected Approval Responses:** When a client sends a `ToolApproval` for an expired or non-existent `tool_call_id`, the daemon must log a debug warning and not panic or corrupt state.
5. **Diff Generation on Non-Existent or Binary Files:** Generating a diff for `write_file` on a brand new file must format all lines as additions (`+`), and gracefully handle non-UTF8/binary content without crashing.

---

### Task 1: Protocol Models & Mode Enums (`protocol.rs`)

**Files:**
- Modify: `daemon/src/protocol.rs`
- Test: `daemon/tests/protocol_test.rs`

**Interfaces:**
- Produces:
  - `enum ExecutionMode { Safe, Yolo, PlanOnly }` with `Display`, `FromStr`, `Serialize`, `Deserialize`
  - `enum ThinkingBudget { Off, Low, Medium, High }` with `Display`, `FromStr`, `Serialize`, `Deserialize`
  - `ToolCallRequestPayload.diff: Option<String>`
  - `struct ToolCallResultPayload { tool_call_id: String, success: bool, output: String }`
  - `ServerMessage::ToolCallResult(ToolCallResultPayload)`

- [ ] **Step 1: Write failing unit tests for `ExecutionMode`, `ThinkingBudget`, and `ToolCallResult`**

In `daemon/tests/protocol_test.rs`:
```rust
#[test]
fn execution_mode_serialization_and_parsing() {
    use aetheris_daemon::protocol::ExecutionMode;
    use std::str::FromStr;

    assert_eq!(ExecutionMode::from_str("safe").unwrap(), ExecutionMode::Safe);
    assert_eq!(ExecutionMode::from_str("yolo").unwrap(), ExecutionMode::Yolo);
    assert_eq!(ExecutionMode::from_str("plan").unwrap(), ExecutionMode::PlanOnly);
    assert_eq!(ExecutionMode::from_str("plan-only").unwrap(), ExecutionMode::PlanOnly);
    assert_eq!(ExecutionMode::from_str("plan_only").unwrap(), ExecutionMode::PlanOnly);

    assert_eq!(serde_json::to_string(&ExecutionMode::Safe).unwrap(), "\"safe\"");
    assert_eq!(serde_json::to_string(&ExecutionMode::Yolo).unwrap(), "\"yolo\"");
    assert_eq!(serde_json::to_string(&ExecutionMode::PlanOnly).unwrap(), "\"plan_only\"");

    let parsed: ExecutionMode = serde_json::from_str("\"plan\"").unwrap();
    assert_eq!(parsed, ExecutionMode::PlanOnly);
}

#[test]
fn thinking_budget_serialization_and_parsing() {
    use aetheris_daemon::protocol::ThinkingBudget;
    use std::str::FromStr;

    assert_eq!(ThinkingBudget::from_str("off").unwrap(), ThinkingBudget::Off);
    assert_eq!(ThinkingBudget::from_str("low").unwrap(), ThinkingBudget::Low);
    assert_eq!(ThinkingBudget::from_str("medium").unwrap(), ThinkingBudget::Medium);
    assert_eq!(ThinkingBudget::from_str("high").unwrap(), ThinkingBudget::High);
}

#[test]
fn tool_call_request_with_diff_roundtrips() {
    use aetheris_daemon::protocol::{ServerMessage, ToolCallRequestPayload};

    let msg = ServerMessage::ToolCallRequest(ToolCallRequestPayload {
        tool_call_id: "call_abc".to_string(),
        tool_name: "write_file".to_string(),
        args: serde_json::json!({ "path": "main.rs" }),
        requires_approval: true,
        diff: Some("--- a/main.rs\n+++ b/main.rs\n@@ -1 +1 @@\n-old\n+new".to_string()),
    });

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --test protocol_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with missing types/variants in `protocol.rs`.

- [ ] **Step 3: Implement `ExecutionMode`, `ThinkingBudget`, and updated payloads in `protocol.rs`**

- Define `ExecutionMode` enum:
  - Variants: `Safe` (default), `Yolo`, `PlanOnly`.
  - Custom deserialization supporting aliases `"safe"`, `"yolo"`, `"auto"`, `"plan"`, `"plan-only"`, `"plan_only"`.
- Define `ThinkingBudget` enum:
  - Variants: `Off` (default), `Low`, `Medium`, `High`.
  - Custom deserialization supporting `"off"`, `"low"`, `"medium"`, `"high"`.
- Add `#[serde(default)] pub diff: Option<String>` to `ToolCallRequestPayload`.
- Add `ToolCallResultPayload` and `ServerMessage::ToolCallResult`.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --test protocol_test --manifest-path daemon/Cargo.toml`
Expected: PASS with all tests passing.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/protocol.rs daemon/tests/protocol_test.rs
git commit -m "feat(protocol): add ExecutionMode, ThinkingBudget, and tool diff schemas"
```

---

### Task 2: Unified Diff Generator (`diff.rs`)

**Files:**
- Create: `daemon/src/diff.rs`
- Modify: `daemon/src/lib.rs`
- Test: `daemon/tests/diff_test.rs`

**Interfaces:**
- Produces:
  - `pub fn generate_unified_diff(old_content: Option<&str>, new_content: &str, file_label: &str) -> String`
  - `pub async fn compute_file_diff(target_path: &std::path::Path, new_content: &str) -> Option<String>`

- [ ] **Step 1: Write failing unit tests for unified diff generation**

In `daemon/tests/diff_test.rs`:
```rust
use aetheris_daemon::diff::generate_unified_diff;

#[test]
fn generates_diff_for_new_file() {
    let diff = generate_unified_diff(None, "line1\nline2\n", "test.txt");
    assert!(diff.contains("--- /dev/null"));
    assert!(diff.contains("+++ b/test.txt"));
    assert!(diff.contains("+line1"));
    assert!(diff.contains("+line2"));
}

#[test]
fn generates_diff_for_modified_file() {
    let old = "fn main() {\n    println!(\"hello\");\n}\n";
    let new = "fn main() {\n    println!(\"hello world\");\n}\n";
    let diff = generate_unified_diff(Some(old), new, "src/main.rs");
    assert!(diff.contains("--- a/src/main.rs"));
    assert!(diff.contains("+++ b/src/main.rs"));
    assert!(diff.contains("-    println!(\"hello\");"));
    assert!(diff.contains("+    println!(\"hello world\");"));
}

#[test]
fn identical_files_produce_empty_diff() {
    let content = "same content\n";
    let diff = generate_unified_diff(Some(content), content, "same.txt");
    assert!(diff.is_empty());
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --test diff_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with `cannot find module diff`.

- [ ] **Step 3: Implement `daemon/src/diff.rs`**

- Use line-based diff comparison (e.g. Myers algorithm or standard line-by-line diff chunking).
- When `old_content` is `None`:
  Outputs standard patch header `--- /dev/null\n+++ b/{file_label}\n@@ -0,0 +1,{count} @@` and prepends `+` to each line.
- When `old_content` is `Some(old)`:
  If `old == new_content`, returns `String::new()`.
  Otherwise, computes diff hunks with unified context lines (3 lines context), `-` deletions, and `+` additions.
- Implement `compute_file_diff(target_path, new_content)`:
  Reads existing file asynchronously via `tokio::fs::read_to_string`.
  If file exists and is readable, passes `Some(&old)`. If file does not exist, passes `None`.
  Returns `Some(diff)` if non-empty, or `None`.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --test diff_test --manifest-path daemon/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/diff.rs daemon/src/lib.rs daemon/tests/diff_test.rs
git commit -m "feat(daemon): implement unified diff generator for file tool calls"
```

---

### Task 3: MCP Server Discovery Scanner (`mcp.rs`)

**Files:**
- Create: `daemon/src/mcp.rs`
- Modify: `daemon/src/lib.rs`, `daemon/src/hyprland.rs`
- Test: `daemon/tests/mcp_test.rs`

**Interfaces:**
- Produces:
  - `pub async fn scan_mcp_servers(workspace_root: Option<&std::path::Path>) -> Vec<String>`

- [ ] **Step 1: Write failing unit test for MCP server scanning**

In `daemon/tests/mcp_test.rs`:
```rust
use std::fs;
use tempfile::tempdir;
use aetheris_daemon::mcp::scan_mcp_servers;

#[tokio::test]
async fn scans_workspace_and_global_mcp_servers() {
    let dir = tempdir().unwrap();
    let mcp_dir = dir.path().join(".gemini/antigravity/mcp");
    fs::create_dir_all(&mcp_dir.join("neon")).unwrap();
    fs::create_dir_all(&mcp_dir.join("cloudflare")).unwrap();
    fs::write(mcp_dir.join("neon/run_sql.json"), "{}").unwrap();

    let ws_dir = tempdir().unwrap();
    let ws_mcp = ws_dir.path().join(".mcp/custom_server");
    fs::create_dir_all(&ws_mcp).unwrap();

    // Set custom env or pass paths to test scanner
    let servers = aetheris_daemon::mcp::scan_mcp_servers_with_roots(
        Some(ws_dir.path()),
        &[mcp_dir],
    ).await;

    assert!(servers.contains(&"neon".to_string()));
    assert!(servers.contains(&"cloudflare".to_string()));
    assert!(servers.contains(&"custom_server".to_string()));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test mcp_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with missing `mcp` module.

- [ ] **Step 3: Implement `daemon/src/mcp.rs`**

- `scan_mcp_servers(workspace_root: Option<&Path>) -> Vec<String>`:
  - Checks global locations:
    - `~/.gemini/antigravity/mcp/`
    - `~/.gemini/mcp/`
    - `~/.config/Claude/claude_desktop_config.json` (if exists, parses keys from `mcpServers` object)
  - Checks workspace locations:
    - `<workspace>/.mcp/`
    - `<workspace>/.agents/mcp/`
    - `<workspace>/.gemini/mcp/`
  - Collects unique, sorted server names.
- Update `hyprland.rs` context sync:
  Calls `scan_mcp_servers(Some(&root)).await` to populate `WorkspaceSyncPayload.mcp_servers`.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test mcp_test --manifest-path daemon/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/mcp.rs daemon/src/lib.rs daemon/src/hyprland.rs daemon/tests/mcp_test.rs
git commit -m "feat(daemon): add MCP server discovery scanner"
```

---

### Task 4: Approval Registry & Interception Engine (`interception.rs`)

**Files:**
- Create: `daemon/src/interception.rs`
- Modify: `daemon/src/lib.rs`
- Test: `daemon/tests/interception_test.rs`

**Interfaces:**
- Produces:
  - `pub struct ApprovalRegistry`: thread-safe registry of pending approval channels
    - `fn register(&self, tool_call_id: &str) -> oneshot::Receiver<bool>`
    - `fn resolve(&self, tool_call_id: &str, approved: bool) -> bool`
    - `fn remove(&self, tool_call_id: &str) -> Option<oneshot::Sender<bool>>`
  - `pub struct ToolInterceptor`:
    - `fn new(mode: Arc<RwLock<ExecutionMode>>, registry: Arc<ApprovalRegistry>, event_tx: broadcast::Sender<ServerMessage>) -> Self`
    - `async fn evaluate_tool_call(&self, tool_call_id: &str, tool_name: &str, args: serde_json::Value, diff: Option<String>, timeout: std::time::Duration) -> Result<bool, DaemonError>`

- [ ] **Step 1: Write failing unit tests for ApprovalRegistry & ToolInterceptor**

In `daemon/tests/interception_test.rs`:
```rust
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
        interceptor.evaluate_tool_call(
            "call_1",
            "run_command",
            serde_json::json!({ "command": "ls" }),
            None,
            Duration::from_secs(5),
        ).await
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
async fn yolo_mode_auto_approves_without_blocking() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Yolo));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry, event_tx);

    let result = interceptor.evaluate_tool_call(
        "call_2",
        "write_file",
        serde_json::json!({ "path": "test.txt" }),
        None,
        Duration::from_secs(1),
    ).await.unwrap();

    assert!(result, "YOLO mode must immediately approve");
}

#[tokio::test]
async fn plan_only_mode_rejects_mutating_tools() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::PlanOnly));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry, event_tx);

    let result = interceptor.evaluate_tool_call(
        "call_3",
        "run_command",
        serde_json::json!({ "command": "rm -rf /" }),
        None,
        Duration::from_secs(1),
    ).await;

    assert!(result.is_err(), "Plan-Only mode must reject mutating tools");
}

#[tokio::test]
async fn safe_mode_timeout_cleans_registry() {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let mode = Arc::new(RwLock::new(ExecutionMode::Safe));
    let registry = Arc::new(ApprovalRegistry::new());
    let interceptor = ToolInterceptor::new(mode, registry.clone(), event_tx);

    let result = interceptor.evaluate_tool_call(
        "call_timeout",
        "run_command",
        serde_json::json!({}),
        None,
        Duration::from_millis(50),
    ).await;

    assert!(result.is_err(), "Expected timeout error");
    assert!(!registry.has_pending("call_timeout"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test interception_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with missing `interception` module.

- [ ] **Step 3: Implement `daemon/src/interception.rs`**

- Implement `ApprovalRegistry`:
  - Internal storage: `tokio::sync::Mutex<HashMap<String, oneshot::Sender<bool>>>`.
  - Methods: `register(&self, id) -> oneshot::Receiver<bool>`, `resolve(&self, id, bool) -> bool`, `has_pending(&self, id) -> bool`, `remove(&self, id)`.
- Implement `ToolInterceptor`:
  - Methods: `is_mutating_tool(name) -> bool` (`run_command`, `write_file`, `edit_file`, etc.).
  - `evaluate_tool_call(...)`:
    - Checks mode:
      - If `PlanOnly` and mutating: returns `Err(DaemonError::InterceptionError("Tool execution rejected: Plan-Only mode is active".into()))`.
      - If `Yolo`: returns `Ok(true)`.
      - If `Safe`:
        - Registers `tool_call_id` in registry.
        - Emits `ServerMessage::ToolCallRequest(...)`.
        - Awaits `tokio::time::timeout(timeout, receiver)`.
        - On timeout: cleans up ID from registry, returns `Err(DaemonError::ApprovalTimeout(...))`.
        - On approval result: returns `Ok(approved)`.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test interception_test --manifest-path daemon/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/interception.rs daemon/src/lib.rs daemon/tests/interception_test.rs
git commit -m "feat(daemon): implement ApprovalRegistry and Safe/YOLO/Plan-Only ToolInterceptor"
```

---

### Task 5: WebSocket Server Integration & Runtime Mode Switcher (`server.rs`, `main.rs`)

**Files:**
- Modify: `daemon/src/server.rs`, `daemon/src/main.rs`, `daemon/src/error.rs`
- Test: `daemon/tests/safety_server_test.rs`

**Interfaces:**
- Produces:
  - WebSocket client can send `update_config` with `{ platform, mode, thinking }` to switch `ExecutionMode` on the fly.
  - WebSocket client can send `tool_approval` with `{ tool_call_id, approved }` to resolve pending approvals.
  - Server maintains and passes `Arc<RwLock<ExecutionMode>>` and `Arc<ApprovalRegistry>` across client connections.

- [ ] **Step 1: Write integration tests for WebSocket config updates and tool approvals**

In `daemon/tests/safety_server_test.rs`:
```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, oneshot};
use tokio_util::sync::CancellationToken;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::interception::ApprovalRegistry;
use aetheris_daemon::protocol::{ClientMessage, ExecutionMode, ServerMessage, ToolApprovalPayload, UpdateConfigPayload};

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
            &config,
            event_tx,
            shutdown,
            addr_tx,
            active_ws,
            srv_mode,
            srv_reg,
        ).await.unwrap();
    });

    let addr = addr_rx.await.unwrap();
    let url = format!("ws://{addr}");
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut sink, mut reader) = ws_stream.split();

    // 1. Update mode to YOLO
    let update_msg = ClientMessage::UpdateConfig(UpdateConfigPayload {
        platform: "antigravity".to_string(),
        mode: "yolo".to_string(),
        thinking: "high".to_string(),
    });
    sink.send(Message::Text(serde_json::to_string(&update_msg).unwrap().into())).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(*mode.read().await, ExecutionMode::Yolo);

    // 2. Register mock tool approval in Safe mode
    *mode.write().await = ExecutionMode::Safe;
    let rx = registry.register("call_test_123");

    let approval_msg = ClientMessage::ToolApproval(ToolApprovalPayload {
        tool_call_id: "call_test_123".to_string(),
        approved: true,
    });
    sink.send(Message::Text(serde_json::to_string(&approval_msg).unwrap().into())).await.unwrap();

    let approved = rx.await.unwrap();
    assert!(approved);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test safety_server_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with `run_server_with_state` not found.

- [ ] **Step 3: Implement server state wiring in `daemon/src/server.rs` and `main.rs`**

- In `server.rs`:
  - Add `execution_mode: Arc<RwLock<ExecutionMode>>` and `approval_registry: Arc<ApprovalRegistry>` to `run_server`.
  - In `handle_client`:
    - On `ClientMessage::UpdateConfig(payload)`:
      Parse mode using `ExecutionMode::from_str(&payload.mode)`. If valid, update `*execution_mode.write().await = new_mode`.
    - On `ClientMessage::ToolApproval(payload)`:
      Call `approval_registry.resolve(&payload.tool_call_id, payload.approved)`.
- In `main.rs`:
  - Instantiate `execution_mode = Arc::new(RwLock::new(ExecutionMode::Safe))` and `approval_registry = Arc::new(ApprovalRegistry::new())`.
  - Pass to `run_server`.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test safety_server_test --manifest-path daemon/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/server.rs daemon/src/main.rs daemon/tests/safety_server_test.rs
git commit -m "feat(daemon): wire runtime mode switcher and tool approval resolver into WebSocket server"
```

---

### Task 6: Multi-Platform Adapter Safety Hooking (`adapters/`)

**Files:**
- Modify: `daemon/src/adapters/mod.rs`, `daemon/src/adapters/antigravity.rs`, `daemon/src/adapters/claude.rs`, `daemon/src/adapters/direct_api.rs`
- Test: `daemon/tests/adapter_test.rs`, `daemon/tests/adapter_safety_test.rs`

**Interfaces:**
- Modifies: `AgentAdapter::set_mode` takes `ExecutionMode`
- Produces:
  - `ClaudeAdapter::build_cli_args` appends `--dangerously-skip-permissions` in `Yolo` mode, and `--permission-prompt-tool` or safe defaults in `Safe` mode.
  - `DirectApiAdapter` / `AntigravityAdapter` store active `ExecutionMode`.

- [ ] **Step 1: Write unit tests for adapter mode handling and CLI flags**

In `daemon/tests/adapter_safety_test.rs`:
```rust
use aetheris_daemon::adapters::ClaudeAdapter;
use aetheris_daemon::protocol::ExecutionMode;

#[test]
fn claude_adapter_sets_yolo_flag() {
    let adapter = ClaudeAdapter::new();
    let safe_args = adapter.build_cli_args("hello", ExecutionMode::Safe);
    assert!(!safe_args.contains(&"--dangerously-skip-permissions".to_string()));

    let yolo_args = adapter.build_cli_args("hello", ExecutionMode::Yolo);
    assert!(yolo_args.contains(&"--dangerously-skip-permissions".to_string()));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test adapter_safety_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with `build_cli_args` not found or signature mismatch.

- [ ] **Step 3: Update `AgentAdapter` and implementations in `daemon/src/adapters/`**

- In `daemon/src/adapters/mod.rs`:
  - `fn set_mode<'a>(&'a self, mode: ExecutionMode) -> BoxFuture<'a, Result<(), DaemonError>>;`
- In `daemon/src/adapters/claude.rs`:
  - Store `current_mode: Arc<RwLock<ExecutionMode>>`.
  - Implement `build_cli_args`: appends `--dangerously-skip-permissions` only when mode is `Yolo`.
- In `daemon/src/adapters/antigravity.rs`:
  - Store `current_mode: Arc<RwLock<ExecutionMode>>`.
- In `daemon/src/adapters/direct_api.rs`:
  - Store `current_mode: Arc<RwLock<ExecutionMode>>`.
- Update `daemon/tests/adapter_test.rs` to match new signature.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --manifest-path daemon/Cargo.toml`
Expected: PASS with all 40+ tests passing.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/adapters/ daemon/tests/adapter_test.rs daemon/tests/adapter_safety_test.rs
git commit -m "feat(adapters): bind ExecutionMode to Claude, Antigravity, and Direct API adapters"
```

---

### Task 7: Full Daemon Safety Quality Gates & Release Verification

**Files:**
- Verify: Entire `daemon/` workspace

- [ ] **Step 1: Run complete quality gate checks**

```bash
cargo fmt --check --manifest-path daemon/Cargo.toml
cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings
cargo test --manifest-path daemon/Cargo.toml
cargo build --release --manifest-path daemon/Cargo.toml
```
Expected:
- `cargo fmt`: 0 formatting issues.
- `cargo clippy`: 0 warnings.
- `cargo test`: 100% tests passing.
- `cargo build --release`: binary created under `daemon/target/release/aetheris-daemon` with size < 10 MB.

- [ ] **Step 2: Commit any remaining test or documentation updates**

```bash
git commit --allow-empty -m "chore(daemon): verify complete Phase 3 Part 1 quality gates"
```
