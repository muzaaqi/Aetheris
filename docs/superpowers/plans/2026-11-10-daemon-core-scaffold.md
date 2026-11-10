# Daemon Core Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational Rust daemon that listens on a local WebSocket (`ws://127.0.0.1:21420`), receives/sends JSON-RPC messages, and tracks Hyprland workspace changes via event-driven IPC — all with zero polling and under 15 MB RAM.

**Architecture:** Three concurrent `tokio::select!` branches in `main.rs` — a WebSocket accept loop, a Hyprland event listener, and a shutdown signal. A `broadcast` channel fans workspace events from the Hyprland listener out to all connected WebSocket clients. Each client is a spawned task that splits the WebSocket stream into a read half (deserialize `ClientMessage`) and a write half (forward `ServerMessage`s from the broadcast receiver). Graceful shutdown via `CancellationToken`.

**Tech Stack:** Rust 2024 edition, `tokio` 1.43+ (multi-threaded), `tokio-tungstenite` 0.26 (WebSocket), `hyprland` 0.4.0-beta.3 (IPC events), `serde`/`serde_json` (JSON-RPC), `thiserror` 2.0 (errors), `tokio-util` 0.7 (`CancellationToken`), `tracing`/`tracing-subscriber` (logging).

**Spec:** [`DESIGN.md`](file:///home/muzaaqi/Projects/Aetheris/DESIGN.md) (Section 4: Communication Protocol) and [`PRODUCT.md`](file:///home/muzaaqi/Projects/Aetheris/PRODUCT.md) (Phase 1 acceptance criteria).

## Global Constraints

- Rust edition 2024, rustc 1.98+.
- Zero `.unwrap()` or `.expect()` in non-test code; all errors via `Result<T, AetherisError>` with `?`.
- Zero `std::fs`, `std::thread::sleep`, or blocking calls on tokio threads.
- Daemon memory footprint < 15 MB RAM at steady state, CPU < 0.1% when idle.
- Zero Hyprland polling — pure event-driven via `AsyncEventListener` on socket2.
- `broadcast` channel capacity bounded (64 slots).
- WebSocket frame size capped at 1 MiB.
- All three quality gates must pass: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Conventional commits in imperative English.

## Review Focus

1. **Malformed JSON over WebSocket** — a client sends `}{broken` or a valid JSON object with an unknown `"type"` field; the daemon must log a warning and continue serving, not panic or drop other clients.
2. **Broadcast lag** — a slow WebSocket client falls behind the bounded broadcast channel; `RecvError::Lagged` must be handled by logging the skip count and continuing, not crashing the client task.
3. **Hyprland socket absent** — the daemon starts before Hyprland or after a compositor crash; `start_listener_async` returns an `Io` error; the daemon must log the failure and continue running the WebSocket server without workspace tracking.
4. **Concurrent client disconnect** — a client disconnects mid-write while the server is broadcasting; the `send` on the sink returns an error; the per-client task must exit cleanly without affecting the accept loop or other clients.
5. **Graceful shutdown ordering** — `Ctrl+C` fires; all spawned tasks (accept loop, per-client handlers, Hyprland listener) must observe the `CancellationToken` and drain within a bounded timeout, not hang.

---

### Task 1: Error Types and Protocol Models

**Files:**
- Create: `daemon/src/error.rs`
- Create: `daemon/src/protocol.rs`
- Create: `daemon/tests/protocol_test.rs`
- Modify: `daemon/src/main.rs` (add `mod` declarations)

**Interfaces:**
- Consumes: nothing (foundation layer)
- Produces:
  - `error::AetherisError` — enum with variants `Io(std::io::Error)`, `WebSocket(tokio_tungstenite::tungstenite::Error)`, `Hyprland(hyprland::shared::HyprError)`, `Json(serde_json::Error)`, `Config(String)`
  - `error::Result<T>` — type alias for `std::result::Result<T, AetherisError>`
  - `protocol::ClientMessage` — `#[serde(tag = "type", content = "payload")]` enum: `SendMessage(SendMessagePayload)`, `ToolApproval(ToolApprovalPayload)`, `UpdateConfig(UpdateConfigPayload)`
  - `protocol::ServerMessage` — `#[serde(tag = "type", content = "payload")]` enum: `TokenStream(TokenStreamPayload)`, `ToolCallRequest(ToolCallRequestPayload)`, `WorkspaceSync(WorkspaceSyncPayload)`
  - `protocol::SendMessagePayload` — `{ text: String, attachments: Vec<String>, quote: Option<QuotePayload> }`
  - `protocol::QuotePayload` — `{ source: String, text: String }`
  - `protocol::ToolApprovalPayload` — `{ tool_call_id: String, approved: bool }`
  - `protocol::UpdateConfigPayload` — `{ platform: String, mode: String, thinking: String }`
  - `protocol::TokenStreamPayload` — `{ token: String, done: bool }`
  - `protocol::ToolCallRequestPayload` — `{ tool_call_id: String, tool_name: String, args: serde_json::Value, requires_approval: bool }`
  - `protocol::WorkspaceSyncPayload` — `{ workspace_id: i32, path: String, git_branch: Option<String>, skills_count: u32, mcp_servers: Vec<String> }`

- [ ] **Step 1: Write failing tests in `daemon/tests/protocol_test.rs`**

```rust
use aetheris_daemon::protocol::{ClientMessage, ServerMessage};

#[test]
fn send_message_deserializes_from_spec_json() {
    let json = r#"{"type":"send_message","payload":{"text":"audit this","attachments":["/tmp/shot.png"],"quote":{"source":"src/db.ts:12-20","text":"const res = await db.query..."}}}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, ClientMessage::SendMessage(p) if p.text == "audit this"));
}

#[test]
fn tool_approval_roundtrips() {
    let msg = ClientMessage::ToolApproval(ToolApprovalPayload {
        tool_call_id: "call_12345".into(),
        approved: true,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn workspace_sync_serializes_to_spec_json() {
    let msg = ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
        workspace_id: 1,
        path: "/home/muzaaqi/Projects/MuMu".into(),
        git_branch: Some("main".into()),
        skills_count: 14,
        mcp_servers: vec!["neon".into(), "cloudflare".into(), "context7".into()],
    });
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"workspace_sync""#));
    assert!(json.contains(r#""workspace_id":1"#));
}

#[test]
fn token_stream_serializes_done_flag() {
    let msg = ServerMessage::TokenStream(TokenStreamPayload {
        token: "Here is ".into(),
        done: false,
    });
    let v: serde_json::Value = serde_json::to_value(&msg).unwrap();
    assert_eq!(v["payload"]["done"], false);
}

#[test]
fn unknown_type_returns_json_error() {
    let json = r#"{"type":"unknown_thing","payload":{}}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}

#[test]
fn malformed_json_returns_error() {
    let result = serde_json::from_str::<ClientMessage>("}{broken");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path daemon/Cargo.toml --test protocol_test 2>&1`
Expected: Compile error — `aetheris_daemon::protocol` does not exist.

- [ ] **Step 3: Implement `error.rs`**

`pub enum AetherisError` with `#[derive(Debug, thiserror::Error)]`, five `#[from]` variants matching the Interfaces block. `pub type Result<T>` alias.

- [ ] **Step 4: Implement `protocol.rs`**

All six payload structs and both tagged enums with `#[derive(Debug, Clone, Serialize, Deserialize)]`, `#[serde(tag = "type", content = "payload")]`, rename variants to snake_case strings matching DESIGN.md Section 4.

- [ ] **Step 5: Wire modules in `main.rs`**

Add `pub mod error;` and `pub mod protocol;`. Add `lib.rs` with the same `pub mod` declarations so integration tests can `use aetheris_daemon::protocol`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path daemon/Cargo.toml --test protocol_test 2>&1`
Expected: All 6 tests PASS.

- [ ] **Step 7: Run quality gates**

Run: `cargo fmt --check --manifest-path daemon/Cargo.toml && cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings`
Expected: Exit 0.

- [ ] **Step 8: Commit**

```bash
git add daemon/src/error.rs daemon/src/protocol.rs daemon/src/lib.rs daemon/src/main.rs daemon/tests/protocol_test.rs
git commit -m "feat(protocol): add JSON-RPC message types and error enum with TDD"
```

---

### Task 2: Daemon Configuration

**Files:**
- Create: `daemon/src/config.rs`
- Modify: `daemon/src/lib.rs` (add `pub mod config;`)

**Interfaces:**
- Consumes: nothing
- Produces:
  - `config::DaemonConfig` — `{ host: String, port: u16, broadcast_capacity: usize, max_frame_size: usize }`
  - `impl Default for DaemonConfig` — `host: "127.0.0.1"`, `port: 21420`, `broadcast_capacity: 64`, `max_frame_size: 1_048_576`
  - `DaemonConfig::addr(&self) -> String` — returns `"{host}:{port}"`

- [ ] **Step 1: Write failing test (inline `#[cfg(test)]` module in `config.rs`)**

```rust
#[test]
fn default_config_binds_to_expected_address() {
    let config = DaemonConfig::default();
    assert_eq!(config.addr(), "127.0.0.1:21420");
    assert_eq!(config.broadcast_capacity, 64);
    assert_eq!(config.max_frame_size, 1_048_576);
}
```

- [ ] **Step 2: Implement `DaemonConfig` struct, `Default` impl, and `addr()` method in `config.rs`**

- [ ] **Step 3: Add `pub mod config;` to `lib.rs`**

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path daemon/Cargo.toml config`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/config.rs daemon/src/lib.rs
git commit -m "feat(config): add daemon configuration with sensible defaults"
```

---

### Task 3: WebSocket Server

**Files:**
- Create: `daemon/src/server.rs`
- Create: `daemon/tests/server_test.rs`
- Modify: `daemon/src/lib.rs` (add `pub mod server;`)

**Interfaces:**
- Consumes:
  - `config::DaemonConfig` — `addr()`, `broadcast_capacity`, `max_frame_size`
  - `protocol::ClientMessage`, `protocol::ServerMessage` — deserialization/serialization
  - `error::Result<T>`
- Produces:
  - `server::run_server(config: &DaemonConfig, event_rx: broadcast::Receiver<ServerMessage>, shutdown: CancellationToken) -> Result<()>` — binds TCP, accepts WebSocket connections, spawns per-client handler tasks
  - Per-client handler (private): splits stream into read/write halves; read half deserializes `ClientMessage` and logs; write half forwards `ServerMessage`s from `event_rx` subscription

- [ ] **Step 1: Write failing integration tests in `daemon/tests/server_test.rs`**

```rust
use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::protocol::ServerMessage;
use aetheris_daemon::server::run_server;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn server_accepts_websocket_connection() {
    let config = DaemonConfig { port: 0, ..Default::default() }; // port 0 = OS-assigned
    // start server, connect a WS client, assert handshake succeeds
}

#[tokio::test]
async fn server_broadcasts_event_to_connected_client() {
    // start server, connect client, send WorkspaceSync through broadcast tx,
    // assert client receives the JSON message
}

#[tokio::test]
async fn server_handles_client_disconnect_gracefully() {
    // start server, connect client, drop client, send another broadcast,
    // assert server is still running (connect a second client and receive)
}

#[tokio::test]
async fn server_handles_malformed_client_message() {
    // start server, connect client, send "}{broken", assert server stays up
}

#[tokio::test]
async fn server_echoes_send_message_as_token_stream() {
    // start server, connect client, send a valid SendMessage JSON,
    // assert client receives a TokenStream with token == original text and done == true
}
```

Test helper: the server must bind to port 0 and report the actual bound address so tests don't collide. Adjust `run_server` to return the bound `SocketAddr` via a `oneshot::channel`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path daemon/Cargo.toml --test server_test 2>&1`
Expected: Compile error — `server` module not found.

- [ ] **Step 3: Implement `server::run_server` in `daemon/src/server.rs`**

Approach:
- `TcpListener::bind(config.addr())`, send bound addr via oneshot.
- `WebSocketConfig { max_frame_size: Some(config.max_frame_size), ..Default::default() }`.
- Accept loop with `tokio::select!` on `listener.accept()` and `shutdown.cancelled()`.
- Per-client: `accept_async_with_config(stream, Some(ws_config))`, split into `(sink, stream)`, subscribe to `event_tx`, `tokio::select!` loop forwarding broadcast messages to sink and reading client messages from stream.
- On `ClientMessage::SendMessage` receive: log with `tracing::info!`, then echo back a `ServerMessage::TokenStream { token: payload.text.clone(), done: true }` to the sender's sink only (Phase 1 echo MVP, replaced by real agent dispatch in Phase 2). On other `ClientMessage` variants: log with `tracing::debug!`. On malformed text: `tracing::warn!`, continue.
- On `broadcast::RecvError::Lagged(n)`: `tracing::warn!("client lagged {n} messages")`, continue.
- On write error or stream end: break cleanly.

- [ ] **Step 4: Adjust `run_server` signature to pass bound address back**

Add parameter `addr_tx: oneshot::Sender<std::net::SocketAddr>` so tests can discover the OS-assigned port.

- [ ] **Step 5: Add `pub mod server;` to `lib.rs`**

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path daemon/Cargo.toml --test server_test 2>&1`
Expected: All 4 tests PASS.

- [ ] **Step 7: Run quality gates**

Run: `cargo fmt --check --manifest-path daemon/Cargo.toml && cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings`
Expected: Exit 0.

- [ ] **Step 8: Commit**

```bash
git add daemon/src/server.rs daemon/src/lib.rs daemon/tests/server_test.rs
git commit -m "feat(server): add WebSocket server with per-client broadcast and graceful shutdown"
```

---

### Task 4: Hyprland Event Listener

**Files:**
- Create: `daemon/src/hyprland.rs`
- Modify: `daemon/src/lib.rs` (add `pub mod hyprland;`)

**Interfaces:**
- Consumes:
  - `protocol::ServerMessage`, `protocol::WorkspaceSyncPayload` — constructing events
  - `error::Result<T>`
  - `tokio::sync::broadcast::Sender<ServerMessage>` — sending workspace events
  - `tokio_util::sync::CancellationToken` — shutdown
- Produces:
  - `hyprland::run_hyprland_listener(event_tx: broadcast::Sender<ServerMessage>, shutdown: CancellationToken) -> Result<()>` — creates `AsyncEventListener`, registers `workspace_changed` and `active_window_changed` handlers, sends `WorkspaceSync` messages through `event_tx`

- [ ] **Step 1: Implement `hyprland::run_hyprland_listener` in `daemon/src/hyprland.rs`**

Approach:
- Create `AsyncEventListener::new()`.
- `add_workspace_changed_handler`: capture `event_tx.clone()`, construct `WorkspaceSyncPayload` from `WorkspaceEventData` (use `data.id` as `workspace_id`, `data.name.to_string()` as fallback path, `None` git branch, 0 skills, empty mcp_servers). Send via `event_tx.send()`, log if no receivers.
- `add_active_window_changed_handler`: capture `event_tx.clone()`, on `Some(WindowEventData { class, title, .. })`, log class and title. Path resolution from window PID is deferred to Phase 2.
- Run `listener.start_listener_async().await` inside a `tokio::select!` with `shutdown.cancelled()`.

Note: `AsyncEventListener` handlers are sync closures per the macro API (`Fn`, not `async`), so `event_tx.send()` (which is non-blocking) is correct inside them.

- [ ] **Step 2: Add `pub mod hyprland;` to `lib.rs`**

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path daemon/Cargo.toml`
Expected: Exit 0. (No unit test for this module — it requires a live Hyprland socket; integration tested manually.)

- [ ] **Step 4: Run quality gates**

Run: `cargo fmt --check --manifest-path daemon/Cargo.toml && cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings`
Expected: Exit 0.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/hyprland.rs daemon/src/lib.rs
git commit -m "feat(hyprland): add event-driven workspace listener via AsyncEventListener"
```

---

### Task 5: Main Orchestrator and Final Wiring

**Files:**
- Modify: `daemon/src/main.rs`
- Modify: `daemon/Cargo.toml` (already done — verify `tokio-util` present)

**Interfaces:**
- Consumes:
  - `config::DaemonConfig` — `broadcast_capacity`, `addr()`
  - `server::run_server(config, event_rx, shutdown)` — WebSocket server
  - `hyprland::run_hyprland_listener(event_tx, shutdown)` — Hyprland events
  - `tokio::sync::broadcast::channel` — event fan-out
  - `tokio_util::sync::CancellationToken` — unified shutdown
- Produces: the daemon binary entry point (no public API)

- [ ] **Step 1: Implement `main.rs`**

Wire:
- `tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env().add_directive("aetheris_daemon=info".parse()?))` init.
- `let config = DaemonConfig::default();`
- `let (event_tx, _) = broadcast::channel(config.broadcast_capacity);`
- `let shutdown = CancellationToken::new();`
- `let (addr_tx, addr_rx) = oneshot::channel();`
- `tokio::select!` racing three branches:
  1. `run_server(&config, event_tx.subscribe(), shutdown.clone(), addr_tx)` — with `?` propagation.
  2. `run_hyprland_listener(event_tx.clone(), shutdown.clone())` — match result: `Ok(())` logs clean exit, `Err(e)` logs warning "Hyprland listener failed: {e}, workspace tracking disabled" (Review Focus #3).
  3. `tokio::signal::ctrl_c()` — on receive, `tracing::info!("shutting down")`, `shutdown.cancel()`.
- Log `addr_rx.await` to confirm bound address.

- [ ] **Step 2: Verify the full build**

Run: `cargo build --manifest-path daemon/Cargo.toml`
Expected: Exit 0.

- [ ] **Step 3: Run all tests**

Run: `cargo test --manifest-path daemon/Cargo.toml`
Expected: All tests from Tasks 1–3 pass.

- [ ] **Step 4: Run all quality gates**

Run: `cargo fmt --check --manifest-path daemon/Cargo.toml && cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings && cargo test --manifest-path daemon/Cargo.toml`
Expected: All three exit 0.

- [ ] **Step 5: Spot-check memory footprint**

Run: `cargo run --release --manifest-path daemon/Cargo.toml &` then `sleep 2 && ps -o rss= -p $(pgrep aetheris)` then `kill $(pgrep aetheris)`.
Expected: RSS < 15360 KB (15 MB).

- [ ] **Step 6: Commit**

```bash
git add daemon/src/main.rs
git commit -m "feat(daemon): wire main orchestrator with tokio::select! and graceful shutdown"
```
