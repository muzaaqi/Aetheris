# AGENTS.md — Aetheris Project

> Primary reference for all AI agents working in this codebase.
> Read this file in full before writing any code, creating plans, or executing tasks.
> When this file conflicts with a feature document, this file wins for conventions.
> When this file conflicts with `PRODUCT.md`, `PRODUCT.md` wins for product requirements.

---

## 1. Project Overview

**Aetheris** is an operating-system-level AI agent conversational and orchestration HUD (Heads-Up Display) embedded directly into the top bar of the **Hyprland** desktop environment on Linux, powered by **Caelestia Shell (Quickshell)** and an ultra-lightweight **Rust** background daemon.

- **Backend Daemon:** Rust daemon (`aetheris-daemon`) providing Hyprland workspace tracking, inotify transcript watchers, multi-platform agent adapters, and a local WebSocket server.
- **Frontend HUD:** Caelestia Quickshell QML component (`AgentHudButton.qml` & `AgentHudPopout.qml`) rendering a native glassmorphic Material 3 interface directly on the top panel.
- **IPC Protocol:** Local WebSocket JSON-RPC (`ws://127.0.0.1:21420`).

**Core Design Philosophy:**
- **Zero Overhead:** Rust daemon memory footprint must remain strictly between **5 MB – 15 MB RAM**.
- **Context Awareness:** Automatically tracks active Hyprland workspaces and window paths with zero CPU polling.
- **Native Visual Harmony:** 100% adherence to Caelestia Shell's design system (`scheme.current`, Material 3, glassmorphism, blur).

---

## 2. Tech Stack

| Layer | Technology | Version / Notes |
|---|---|---|
| **Daemon Language** | Rust | Edition 2024 (rustc 1.98+) |
| **Async Runtime** | `tokio` | v1.43+ (Multi-threaded async runtime) |
| **IPC Protocol** | `tokio-tungstenite` | Local WebSocket server (`ws://127.0.0.1:21420`) |
| **Hyprland IPC** | `hyprland` crate | v0.4+ (Event-driven socket listener via `/tmp/hypr/.../.socket2.sock`) |
| **Filesystem Watcher**| `notify` | v7.0+ (Linux inotify for real-time transcript streaming) |
| **Fuzzy File Indexer** | `ignore` & `walkdir` | v0.4+ (Ripgrep engine for instant git-aware `@file` mentions) |
| **Serialization** | `serde` & `serde_json` | Universal JSON-RPC message contracts |
| **Frontend Shell** | Quickshell (QML) | Caelestia Shell v2.5+ (QtQuick / Qt 6) |
| **Process Manager** | systemd (User) | `aetheris.service` (Linger-supported autostart) |
| **Agent Backends** | Antigravity, Claude Code, Direct LLM (9router / OpenRouter) | Multi-platform adapter pattern |

---

## 3. Repository Structure

```
~/Projects/Aetheris/
├── daemon/                    # Rust backend daemon workspace
│   ├── Cargo.toml             # Daemon dependencies and build configuration
│   └── src/
│       ├── main.rs            # Entry point & WebSocket server startup
│       ├── config.rs          # Daemon configuration & CLI flags
│       ├── hyprland.rs        # Hyprland socket listener & workspace context resolver
│       ├── transcript.rs      # Kernel inotify watcher for Antigravity transcript.jsonl
│       ├── context.rs         # Fast fuzzy file indexer & 2-tier skill scanner
│       ├── adapters/          # Multi-platform agent normalization
│       │   ├── mod.rs         # AgentAdapter trait definition
│       │   ├── antigravity.rs # Antigravity CLI & transcript adapter
│       │   ├── claude.rs      # Claude Code CLI adapter
│       │   └── direct_api.rs  # Direct SSE streaming API (OpenAI/Anthropic/9router)
│       └── protocol.rs        # Serde models for WebSocket JSON-RPC communication
│
├── qml/                       # Caelestia Quickshell UI modules
│   ├── AgentHudButton.qml     # Dynamic status icon on the top panel bar
│   └── popouts/
│       ├── AgentHudPopout.qml # Glassmorphic overlay window (Chat & Artifacts tabs)
│       └── components/
│           ├── ChatView.qml   # Streaming markdown message list
│           ├── ToolCard.qml   # Interactive MCP / Tool execution card with [Approve]/[Reject]
│           ├── ModeBar.qml    # Platform dropdown, Safe/YOLO mode, and Thinking slider
│           ├── FileDropArea.qml# Native drag-and-drop file target
│           └── QuoteBox.qml   # Quoted snippet preview block
│
├── systemd/
│   └── aetheris.service       # Systemd user service unit definition
│
├── AGENTS.md                  # This file (Engineering conventions & rules)
├── PRODUCT.md                 # Product requirements, user stories, & milestone criteria
├── DESIGN.md                  # Complete architectural specification & WebSocket protocol
├── README.md                  # Project overview
└── .gitignore
```

---

## 4. Development Workflow & Commands

All development commands are executed from the **project root** or `daemon/` directory.

### Rust Daemon Commands

```bash
# Build daemon in debug mode
cargo build --manifest-path daemon/Cargo.toml

# Build daemon optimized for release (<10MB binary)
cargo build --release --manifest-path daemon/Cargo.toml

# Run daemon directly in console
cargo run --manifest-path daemon/Cargo.toml

# Run tests
cargo test --manifest-path daemon/Cargo.toml

# Check code without building binary
cargo check --manifest-path daemon/Cargo.toml

# Format and lint code
cargo fmt --manifest-path daemon/Cargo.toml
cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings
```

### Systemd Service Management

```bash
# Symlink service file into user systemd directory
ln -sf ~/Projects/Aetheris/systemd/aetheris.service ~/.config/systemd/user/aetheris.service

# Reload and enable user service
systemctl --user daemon-reload
systemctl --user enable --now aetheris.service

# View daemon logs
journalctl --user -u aetheris.service -f
```

### Caelestia Shell Testing & Reload

```bash
# Reload Caelestia Shell without logging out
qs -c caelestia kill && caelestia shell -d
```

---

## 5. Critical Architecture Rules

These rules are strictly enforced for all agents contributing code:

1. **Rule 1 — Zero Async Blocking:**
   Never execute blocking filesystem operations (`std::fs`), blocking process calls, or synchronous sleep on tokio threads. Always use `tokio::fs`, `tokio::process::Command`, and `tokio::time::sleep`.
2. **Rule 2 — Memory Footprint Discipline (<15 MB RAM):**
   The daemon must remain lightweight and leak-free. Avoid buffering unbounded file contents into memory. Stream large files, diffs, and transcripts incrementally.
3. **Rule 3 — Event-Driven Hyprland Sync (Zero Polling):**
   Never poll `hyprctl` in an infinite loop. Use the `hyprland` crate's async event listener on `/tmp/hypr/.../.socket2.sock` to react to `activeworkspace` and `activewindow` events instantly.
4. **Rule 4 — Human-in-the-Loop Safety Interception:**
   When running in `Safe Mode`, any tool call that performs state mutations (file modifications, shell commands, web requests) must emit a `tool_call_request` to the frontend and block execution until explicit `tool_approval` is received.
5. **Rule 5 — Idiomatic Rust Error Handling:**
   Never call `.unwrap()` or `.expect()` in non-test production code. Propagate errors via `Result<T, E>` using `thiserror` or custom error enums.
6. **Rule 6 — Strict Caelestia Design Harmony:**
   Never hardcode arbitrary hex colors, fonts, or non-standard border radiuses in QML components. Always bind to `scheme.current`, `Colours.qml`, and `ScreenState.qml`.
7. **Rule 7 — Universal Platform Abstraction:**
   Frontend QML must never contain vendor-specific logic for Antigravity vs Claude Code vs OpenAI. All normalization (mode flags, thinking budgets, skill formatting) belongs inside `daemon/src/adapters/`.
8. **Rule 8 — Zero AI Attribution Standard:**
   Do not add co-authorship comments, AI disclaimers, or "Generated by AI" footers in commit messages or source files. Maintain clean, human-written craftsmanship throughout.
