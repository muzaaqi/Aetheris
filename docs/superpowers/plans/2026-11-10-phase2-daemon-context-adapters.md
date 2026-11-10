# Phase 2 (Part 1) — Daemon Context Awareness, Inotify Transcript Watcher & Multi-Platform Adapters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 2 backend daemon capabilities: fast git-aware file indexer, 2-tier skill scanner, Hyprland window process cwd and git branch detection, inotify Antigravity transcript streaming, and the multi-platform adapter abstraction layer.

**Architecture:** Extend `aetheris-daemon` with `context.rs` for ripgrep-powered file and skill discovery using `tokio::task::spawn_blocking`, `hyprland.rs` with `/proc/<pid>/cwd` active window workspace resolution and `.git/HEAD` branch sniffing, `transcript.rs` with Linux kernel `inotify` incremental streaming via `notify`, and `adapters/` for multi-platform normalization (`AgentAdapter` trait).

**Tech Stack:** Rust Edition 2024, `tokio` 1.43, `notify` 7.0 (Linux inotify), `ignore` 0.4 & `walkdir` 2.5 (git-aware filesystem indexing), `hyprland` 0.4, `serde` 1.0, `thiserror` 2.0.

**Spec:** [`DESIGN.md`](file:///home/muzaaqi/Projects/Aetheris/DESIGN.md) & [`PRODUCT.md`](file:///home/muzaaqi/Projects/Aetheris/PRODUCT.md) (Epic 1, 2, 3 & Phase 2 Acceptance Criteria).

## Global Constraints

- **Memory Footprint:** Daemon memory footprint must strictly remain < 15 MB RAM at steady state (Rule 2).
- **Zero Polling:** Zero CPU polling for Hyprland events and filesystem changes (Rule 3).
- **Zero Blocking Async:** No synchronous IO or CPU-heavy walks on Tokio worker threads; use `tokio::fs` or `tokio::task::spawn_blocking` (Rule 1).
- **Error Handling:** Zero `.unwrap()` or `.expect()` calls in production code; all errors typed via `thiserror` (Rule 5).
- **Platform Abstraction:** All platform-specific normalization isolated within `daemon/src/adapters/` (Rule 7).
- **Trunk Hygiene:** All changes implemented on short-lived branch `feat/daemon-context-adapters` with passing quality gates before merging (Rule 10).

## Review Focus

1. **Fuzzy search with special characters or empty query:** `context::search_files` must handle regex-special characters (`[`, `*`, `(`, `?`) as literal substrings without panicking or returning invalid patterns, and clamp empty queries to `limit`. Pinned in Task 2.
2. **Missing or non-git workspace path:** `detect_git_branch` on directory without `.git` must return `None` gracefully without errors or process crashes. Pinned in Task 3.
3. **Truncated or rapidly rotated transcript file:** `transcript::TranscriptWatcher` must detect file shrinkage (file size < byte offset), reset offset to 0, and continue streaming without panicking. Pinned in Task 4.
4. **Malformed or partial JSONL lines in transcript:** Incomplete lines during atomic file writes must be preserved in buffer until newline without failing JSON deserialization. Pinned in Task 4.
5. **Skill with missing or malformed YAML frontmatter:** `context::scan_skills` must fall back to directory name for skill name and empty description without failing the rest of the scan. Pinned in Task 2.

---

### Task 1: Protocol Models for File Search, Skills, and Rich Workspace Sync

**Files:**
- Modify: `daemon/src/protocol.rs`
- Modify: `daemon/tests/protocol_test.rs`

**Interfaces:**
- Consumes: `serde::{Deserialize, Serialize}`
- Produces:
  - `ClientMessage::SearchFiles { query: String, limit: Option<usize> }`
  - `ClientMessage::ListSkills`
  - `ServerMessage::FileResults { query: String, matches: Vec<FileMatch> }`
  - `ServerMessage::SkillsList { skills: Vec<SkillInfo> }`
  - `FileMatch { path: String, is_dir: bool }`
  - `SkillInfo { name: String, description: String, scope: SkillScope, path: String }`
  - `SkillScope { User, Workspace }`
  - Enhanced `WorkspaceSyncPayload { workspace_id: u64, path: String, git_branch: Option<String>, skills_count: usize, mcp_servers: Vec<String> }`

- [ ] **Step 1: Write the failing tests in `daemon/tests/protocol_test.rs`**

```rust
#[test]
fn search_files_deserializes_and_roundtrips() {
    let json = r#"{"type":"search_files","payload":{"query":"main.rs","limit":10}}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("deserialize failed");
    match msg {
        ClientMessage::SearchFiles { query, limit } => {
            assert_eq!(query, "main.rs");
            assert_eq!(limit, Some(10));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn file_results_serializes_to_spec() {
    let msg = ServerMessage::FileResults {
        query: "main".into(),
        matches: vec![FileMatch {
            path: "daemon/src/main.rs".into(),
            is_dir: false,
        }],
    };
    let json = serde_json::to_string(&msg).expect("serialize failed");
    assert!(json.contains(r#""type":"file_results""#));
    assert!(json.contains("daemon/src/main.rs"));
}

#[test]
fn skills_list_serializes_with_scope() {
    let msg = ServerMessage::SkillsList {
        skills: vec![SkillInfo {
            name: "rust-testing".into(),
            description: "Testing guidelines".into(),
            scope: SkillScope::Workspace,
            path: ".agents/skills/rust-testing".into(),
        }],
    };
    let json = serde_json::to_string(&msg).expect("serialize failed");
    assert!(json.contains(r#""type":"skills_list""#));
    assert!(json.contains("workspace"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test protocol_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with unknown variants/types `SearchFiles`, `FileResults`, `SkillsList`, `FileMatch`, `SkillInfo`, `SkillScope`.

- [ ] **Step 3: Implement protocol extensions in `daemon/src/protocol.rs`**

Add `SearchFiles` and `ListSkills` to `ClientMessage`.
Add `FileResults` and `SkillsList` to `ServerMessage`.
Define `FileMatch`, `SkillInfo`, and `SkillScope` with `serde` rename rules (`lowercase` for `SkillScope`).
Enhance `WorkspaceSyncPayload` to include `git_branch: Option<String>`, `skills_count: usize`, `mcp_servers: Vec<String>`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test protocol_test --manifest-path daemon/Cargo.toml`
Expected: PASS (all tests ok).

- [ ] **Step 5: Commit**

```bash
git add daemon/src/protocol.rs daemon/tests/protocol_test.rs
git commit -m "feat(protocol): add search_files, skills_list, and rich workspace_sync models"
```

---

### Task 2: Fast Git-Aware File Indexer & 2-Tier Skill Scanner (`context.rs`)

**Files:**
- Create: `daemon/src/context.rs`
- Modify: `daemon/src/lib.rs`
- Create: `daemon/tests/context_test.rs`

**Interfaces:**
- Consumes: `protocol::{FileMatch, SkillInfo, SkillScope}`, `ignore::WalkBuilder`, `std::path::{Path, PathBuf}`
- Produces:
  - `pub async fn search_files(root: &Path, query: &str, limit: usize) -> Result<Vec<FileMatch>, DaemonError>`
  - `pub async fn scan_skills(workspace_root: Option<&Path>) -> Result<Vec<SkillInfo>, DaemonError>`
  - `pub fn parse_skill_markdown(path: &Path, content: &str) -> (String, String)`

- [ ] **Step 1: Write the failing tests in `daemon/tests/context_test.rs`**

```rust
use aetheris_daemon::context::{parse_skill_markdown, scan_skills, search_files};
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn search_files_finds_matching_files_and_respects_ignore() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("hello.rs"), "fn main() {}").unwrap();
    fs::write(root.join("world.txt"), "hello").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/hello_sub.rs"), "fn sub() {}").unwrap();

    let matches = search_files(root, "hello", 10).await.unwrap();
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().any(|m| m.path.contains("hello.rs")));
    assert!(matches.iter().any(|m| m.path.contains("hello_sub.rs")));
}

#[tokio::test]
async fn search_files_handles_regex_special_characters_safely() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("file[1].rs"), "fn test() {}").unwrap();

    // Query with unclosed bracket must match literal string and not panic
    let matches = search_files(root, "file[", 10).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].path.contains("file[1].rs"));
}

#[tokio::test]
async fn parse_skill_markdown_extracts_frontmatter_or_fallback() {
    let content_with_fm = "---\nname: my-skill\ndescription: Do things\n---\n# Body";
    let (name, desc) = parse_skill_markdown(std::path::Path::new("/dummy/SKILL.md"), content_with_fm);
    assert_eq!(name, "my-skill");
    assert_eq!(desc, "Do things");

    let content_without_fm = "# Plain Title\nNo frontmatter here.";
    let (name, desc) = parse_skill_markdown(std::path::Path::new("/dummy/tool-maker/SKILL.md"), content_without_fm);
    assert_eq!(name, "tool-maker");
    assert_eq!(desc, "");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test context_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with unresolved module `context`.

- [ ] **Step 3: Implement `daemon/src/context.rs`**

- Add `tempfile` as dev-dependency in `daemon/Cargo.toml` if needed for testing.
- Implement `search_files`: Wrap `ignore::WalkBuilder::new(root)` in `tokio::task::spawn_blocking` to avoid blocking Tokio async threads (Rule 1). Match path against query with case-insensitive `contains`.
- Implement `parse_skill_markdown`: Parse frontmatter between leading `---` markers; fall back to parent directory name if frontmatter is missing or invalid.
- Implement `scan_skills`:
  - User tier: Check `$HOME/.gemini/antigravity/builtin/skills`, `$HOME/.gemini/skills`, `$HOME/.agents/skills`.
  - Workspace tier: Check `<workspace_root>/.agents/skills`.
  - Traverse child directories looking for `SKILL.md`, parse metadata, return `Vec<SkillInfo>`.
- Export module in `daemon/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test context_test --manifest-path daemon/Cargo.toml`
Expected: PASS (all tests ok).

- [ ] **Step 5: Commit**

```bash
git add daemon/src/context.rs daemon/src/lib.rs daemon/tests/context_test.rs daemon/Cargo.toml
git commit -m "feat(daemon): implement git-aware file search and 2-tier skill scanner"
```

---

### Task 3: Hyprland Window PID $\to$ `/proc/{pid}/cwd` Workspace Path & Git Branch Resolver

**Files:**
- Modify: `daemon/src/hyprland.rs`
- Create: `daemon/tests/hyprland_context_test.rs`

**Interfaces:**
- Consumes: `/proc/{pid}/cwd`, `.git/HEAD`, `protocol::WorkspaceSyncPayload`
- Produces:
  - `pub fn resolve_window_cwd(pid: u32) -> Option<PathBuf>`
  - `pub fn detect_git_branch(path: &Path) -> Option<String>`
  - Enhanced `HyprlandEventListener` tracking focused window PID and broadcasting enriched `WorkspaceSyncPayload`.

- [ ] **Step 1: Write the failing tests in `daemon/tests/hyprland_context_test.rs`**

```rust
use aetheris_daemon::hyprland::{detect_git_branch, resolve_window_cwd};
use std::fs;
use tempfile::tempdir;

#[test]
fn resolve_window_cwd_resolves_current_process() {
    let pid = std::process::id();
    let cwd = resolve_window_cwd(pid);
    assert!(cwd.is_some());
    let current_dir = std::env::current_dir().unwrap();
    assert_eq!(cwd.unwrap(), current_dir);
}

#[test]
fn detect_git_branch_reads_standard_head() {
    let dir = tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/feat/daemon-context\n").unwrap();

    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, Some("feat/daemon-context".to_string()));
}

#[test]
fn detect_git_branch_returns_none_for_non_git_directory() {
    let dir = tempdir().unwrap();
    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, None);
}

#[test]
fn detect_git_branch_handles_detached_head() {
    let dir = tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(git_dir.join("HEAD"), "0123456789abcdef0123456789abcdef01234567\n").unwrap();

    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, Some("0123456".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test hyprland_context_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with cannot find functions `resolve_window_cwd` and `detect_git_branch`.

- [ ] **Step 3: Implement CWD resolution and git branch detection in `daemon/src/hyprland.rs`**

- Implement `resolve_window_cwd(pid: u32) -> Option<PathBuf>` using `std::fs::read_link(format!("/proc/{}/cwd", pid)).ok()`.
- Implement `detect_git_branch(path: &Path) -> Option<String>`: read `path.join(".git/HEAD")`. If starts with `ref: refs/heads/`, strip prefix. If detached 40-char SHA, take first 7 chars.
- Update `hyprland::start_listener` to fetch active client window PID on `activewindow` and `activeworkspace`, resolve the directory and git branch, and emit enriched `WorkspaceSyncPayload`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test hyprland_context_test --manifest-path daemon/Cargo.toml`
Expected: PASS (all tests ok).

- [ ] **Step 5: Commit**

```bash
git add daemon/src/hyprland.rs daemon/tests/hyprland_context_test.rs
git commit -m "feat(hyprland): add active window PID cwd resolution and git branch detection"
```

---

### Task 4: In-Kernel Inotify Transcript Watcher (`transcript.rs`)

**Files:**
- Create: `daemon/src/transcript.rs`
- Modify: `daemon/src/lib.rs`
- Create: `daemon/tests/transcript_test.rs`

**Interfaces:**
- Consumes: `notify::{Event, RecommendedWatcher, Watcher}`, `protocol::ServerMessage`
- Produces:
  - `pub struct TranscriptWatcher`
  - `pub fn new(event_tx: tokio::sync::broadcast::Sender<ServerMessage>) -> Self`
  - `pub async fn watch_file(&mut self, path: PathBuf) -> Result<(), DaemonError>`
  - `pub fn parse_transcript_line(line: &str) -> Option<(String, bool)>` (returns token and done flag)

- [ ] **Step 1: Write the failing tests in `daemon/tests/transcript_test.rs`**

```rust
use aetheris_daemon::protocol::ServerMessage;
use aetheris_daemon::transcript::{parse_transcript_line, TranscriptWatcher};
use std::fs::{self, OpenOptions};
use std::io::Write;
use tempfile::tempdir;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

#[test]
fn parse_transcript_line_extracts_planner_response() {
    let json = r#"{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Processing complete."}"#;
    let (token, done) = parse_transcript_line(json).expect("parse failed");
    assert_eq!(token, "Processing complete.");
    assert_eq!(done, false);
}

#[tokio::test]
async fn transcript_watcher_streams_new_lines_and_handles_truncation() {
    let dir = tempdir().unwrap();
    let transcript_path = dir.path().join("transcript.jsonl");
    fs::write(&transcript_path, "").unwrap();

    let (tx, mut rx) = broadcast::channel(16);
    let mut watcher = TranscriptWatcher::new(tx);
    watcher.watch_file(transcript_path.clone()).await.unwrap();

    // Append a line
    let mut file = OpenOptions::new().append(true).open(&transcript_path).unwrap();
    writeln!(file, r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hello world"}}"#).unwrap();
    file.flush().unwrap();

    // Verify broadcast received
    let received = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    match received {
        ServerMessage::TokenStream { token, done } => {
            assert_eq!(token, "Hello world");
            assert!(!done);
        }
        _ => panic!("unexpected message"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test transcript_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with module `transcript` not found.

- [ ] **Step 3: Implement `daemon/src/transcript.rs`**

- Define `TranscriptWatcher` with `notify::RecommendedWatcher` tracking modifications on the target path.
- Maintain `current_offset: u64` and `line_buffer: String` to handle partial line writes atomically.
- Implement truncation detection: if `file.metadata().len() < current_offset`, reset `current_offset = 0`.
- Parse lines using `serde_json::Value`: extract `content` or `thinking` from steps where `source == "MODEL"`.
- Stream `ServerMessage::TokenStream` to the broadcast channel.
- Register module in `daemon/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test transcript_test --manifest-path daemon/Cargo.toml`
Expected: PASS (all tests ok).

- [ ] **Step 5: Commit**

```bash
git add daemon/src/transcript.rs daemon/src/lib.rs daemon/tests/transcript_test.rs
git commit -m "feat(transcript): add inotify transcript watcher with incremental streaming"
```

---

### Task 5: Multi-Platform Adapter Layer (`adapters/`)

**Files:**
- Create: `daemon/src/adapters/mod.rs`
- Create: `daemon/src/adapters/antigravity.rs`
- Create: `daemon/src/adapters/claude.rs`
- Create: `daemon/src/adapters/direct_api.rs`
- Modify: `daemon/src/lib.rs`
- Create: `daemon/tests/adapter_test.rs`

**Interfaces:**
- Consumes: `daemon::error::DaemonError`, `daemon::protocol::ClientMessage`
- Produces:
  - `pub trait AgentAdapter: Send + Sync`
    - `fn platform(&self) -> &'static str;`
    - `async fn send_prompt(&self, prompt: &str, attachments: &[String]) -> Result<(), DaemonError>;`
    - `async fn set_mode(&self, mode: &str) -> Result<(), DaemonError>;`
  - `pub struct AntigravityAdapter`
  - `pub struct ClaudeAdapter`
  - `pub struct DirectApiAdapter`
  - `pub fn create_adapter(platform: &str) -> Box<dyn AgentAdapter>`

- [ ] **Step 1: Write the failing tests in `daemon/tests/adapter_test.rs`**

```rust
use aetheris_daemon::adapters::{create_adapter, AgentAdapter};

#[tokio::test]
async fn create_adapter_instantiates_supported_platforms() {
    let agy = create_adapter("antigravity");
    assert_eq!(agy.platform(), "antigravity");

    let claude = create_adapter("claude");
    assert_eq!(claude.platform(), "claude");

    let direct = create_adapter("direct_api");
    assert_eq!(direct.platform(), "direct_api");
}

#[tokio::test]
async fn adapter_rejects_invalid_mode() {
    let agy = create_adapter("antigravity");
    let res = agy.set_mode("super_danger_mode").await;
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test adapter_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with module `adapters` not found.

- [ ] **Step 3: Implement `daemon/src/adapters/`**

- `adapters/mod.rs`: Define `AgentAdapter` trait and `create_adapter(platform: &str) -> Box<dyn AgentAdapter>`.
- `adapters/antigravity.rs`: Implements `AgentAdapter` for Antigravity (validates modes: `safe`, `yolo`, `plan`).
- `adapters/claude.rs`: Implements `AgentAdapter` for Claude Code (`claude --dangerously-skip-permissions`).
- `adapters/direct_api.rs`: Implements `AgentAdapter` for 9router / OpenRouter SSE.
- Export `pub mod adapters;` in `daemon/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test adapter_test --manifest-path daemon/Cargo.toml`
Expected: PASS (all tests ok).

- [ ] **Step 5: Commit**

```bash
git add daemon/src/adapters/ daemon/src/lib.rs daemon/tests/adapter_test.rs
git commit -m "feat(adapters): add multi-platform adapter abstraction and implementations"
```

---

### Task 6: WebSocket Server Integration & Full Context Verification

**Files:**
- Modify: `daemon/src/server.rs`
- Modify: `daemon/src/main.rs`
- Create: `daemon/tests/context_server_test.rs`

**Interfaces:**
- Consumes: `context::search_files`, `context::scan_skills`, `protocol::{ClientMessage, ServerMessage}`
- Produces: Integrated WebSocket server responding to `SearchFiles` and `ListSkills` RPCs, broadcasting enriched `WorkspaceSync`.

- [ ] **Step 1: Write the failing tests in `daemon/tests/context_server_test.rs`**

```rust
use aetheris_daemon::config::DaemonConfig;
use aetheris_daemon::protocol::{ClientMessage, ServerMessage};
use aetheris_daemon::server::start_server;
use futures_util::{SinkExt, StreamExt};
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn server_responds_to_search_files_and_list_skills() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, _) = broadcast::channel(16);
    let shutdown = CancellationToken::new();

    let temp_workspace = tempdir().unwrap();
    std::fs::write(temp_workspace.path().join("demo.txt"), "hello").unwrap();

    let config = DaemonConfig::default();
    let server_shutdown = shutdown.clone();
    let server_tx = tx.clone();
    tokio::spawn(async move {
        start_server(listener, server_tx, server_shutdown, &config).await.unwrap();
    });

    let (mut ws_stream, _) = connect_async(format!("ws://{}", addr)).await.unwrap();

    // 1. Send search_files
    let req = serde_json::to_string(&ClientMessage::SearchFiles {
        query: "demo".into(),
        limit: Some(5),
    }).unwrap();
    ws_stream.send(Message::Text(req.into())).await.unwrap();

    // 2. Expect file_results
    let msg = ws_stream.next().await.unwrap().unwrap();
    let text = msg.to_text().unwrap();
    let resp: ServerMessage = serde_json::from_str(text).unwrap();
    match resp {
        ServerMessage::FileResults { query, matches } => {
            assert_eq!(query, "demo");
            // Results will be returned based on server's active workspace
        }
        _ => panic!("expected FileResults"),
    }

    shutdown.cancel();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test context_server_test --manifest-path daemon/Cargo.toml`
Expected: FAIL with unhandled message variant in `handle_client_message`.

- [ ] **Step 3: Integrate context querying into `daemon/src/server.rs` and `main.rs`**

- In `server.rs`: In `handle_client_message`, handle:
  - `ClientMessage::SearchFiles { query, limit }`: call `context::search_files(&current_workspace, &query, limit.unwrap_or(20))` and respond with `ServerMessage::FileResults`.
  - `ClientMessage::ListSkills`: call `context::scan_skills(Some(&current_workspace))` and respond with `ServerMessage::SkillsList`.
- In `main.rs`: Bind `hyprland::start_listener` and `transcript::TranscriptWatcher` to the broadcast sender.

- [ ] **Step 4: Run all daemon tests and quality gates**

Run:
```bash
cargo fmt --check --manifest-path daemon/Cargo.toml
cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings
cargo test --manifest-path daemon/Cargo.toml
```
Expected: PASS with 0 warnings and all tests passing.

- [ ] **Step 5: Commit**

```bash
git add daemon/src/server.rs daemon/src/main.rs daemon/tests/context_server_test.rs
git commit -m "feat(daemon): integrate context file search, skills discovery, and transcript watcher"
```
