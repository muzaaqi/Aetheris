# Design & Architecture Specification: Aetheris — OS-Level AI Agent HUD

> **Project Name:** Aetheris  
> **Status:** Approved / Ready for Build  
> **Target Platform:** Linux (Hyprland + Caelestia Quickshell)  
> **Core Technologies:** Rust (Backend Daemon) + QML / Quickshell (Frontend Bar & HUD)  
> **Agent Integrations:** Antigravity (Google DeepMind), Claude Code (Anthropic), Direct LLM API (via 9router / OpenRouter)

---

## 1. Executive Summary & Product Vision

**Aetheris** is an operating-system-level AI agent conversational and orchestration interface embedded directly into the top bar of the Hyprland desktop environment.

The name **Aetheris** is derived from *Aether* (the pure, radiant upper air breathed by the gods in Greek mythology), harmonizing naturally with the celestial naming convention of *Caelestia* and the radiance of *Hyprland* (Hyperion).

Unlike detached web chat interfaces or standalone terminal windows, Aetheris acts as a **system-level, context-aware agent control center**:
- Visually and seamlessly integrated into the Caelestia Shell bar (Glassmorphic, Material 3, dynamic theming matching `scheme.current`).
- Automatically tracks the active Hyprland workspace: switching to a project workspace immediately shifts context, repository files, active session, and scoped skills.
- Serves as a unified visual HUD for existing CLI agent engines (Antigravity CLI, Claude Code), streaming transcripts in real-time, providing human-in-the-loop safety approvals, and rendering code diff previews.

---

## 2. System Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                      HYPRLAND DESKTOP ENVIRONMENT                      │
├────────────────────────────────────────────────────────────────────────┤
│  Top Bar (Caelestia QML)                                               │
│  [ Workspace Pager ]    [ Aetheris Icon 🟢 ]    [ Clock & Status ]     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Click / Shortcut (SUPER + Space)
┌───────────────────────────────────▼────────────────────────────────────┐
│                  CAELESTIA QML HUD OVERLAY (FRONTEND)                  │
│  - Mode Bar: [Antigravity ▼] [⚡ YOLO / 🛡️ Safe] [Thinking: Med ▼]    │
│  - Tabs: [ 💬 Chat & Tools ]  [ 📄 Artifacts & Plans ]                 │
│  - Chat View: Streaming Markdown, Tool Cards, Collapsible Diffs        │
│  - Input Box: Native DropArea (drag files), @file fuzzy picker, Quote  │
└───────────────────────────────────▲────────────────────────────────────┘
                                    │ Local WebSocket (ws://127.0.0.1:21420)
┌───────────────────────────────────▼────────────────────────────────────┐
│                    AETHERIS-DAEMON (RUST BACKEND)                      │
│  Memory Footprint: ~8 MB RAM | Zero-GC | tokio async runtime           │
│                                                                        │
│  ┌───────────────────────┐  ┌──────────────────────────────────────┐  │
│  │   Hyprland Monitor    │  │          Transcript Watcher          │  │
│  │   (hyprland-rs IPC)   │  │       (notify inotify engine)        │  │
│  │ - Workspace detection │  │ - Watch Antigravity transcript.jsonl │  │
│  │ - Auto-detect active  │  │ - Live token & tool call streaming   │  │
│  │   working directory   │  │                                      │  │
│  └───────────────────────┘  └──────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    Platform Normalizer Layer                     │  │
│  │  - Antigravity Adapter (CLI wrapper / transcript pipe)           │  │
│  │  - Claude Code Adapter (CLI child process / stdio JSON)          │  │
│  │  - Direct LLM / 9router Adapter (Streaming SSE API)             │  │
│  │  - Universal Mode & Thinking Effort Translator                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Context & Tool Discovery                     │  │
│  │  - Skill Scanner (~/.gemini, ~/.agents, <workspace>/.agents)     │  │
│  │  - MCP Discovery (Neon, Context7, git, project-scoped)           │  │
│  │  - Fast Fuzzy File Indexer (ignore & walkdir crates)             │  │
│  │  - Quota & Token Usage Tracker                                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Detailed Feature Specifications

### A. Workspace & Context Binding
- **Automatic Detection:** The daemon subscribes to the Hyprland IPC socket events (`activeworkspace` and `activewindow`).
- **Path Resolution:** Resolves the active project working directory based on the focused window (e.g., WezTerm terminal inside `~/Projects/MuMu` or Zed editor inside `~/.config/hypr`).
- **Session Isolation:** Each project path maintains its own distinct chat session. Switching from Workspace 1 to Workspace 2 automatically hot-swaps the chat history, active git branch, and context.

### B. Universal Multi-Platform Adapter
Provides a seamless unified experience across distinct agent backends:
1. **Antigravity Adapter:**
   - Watches `<appDataDir>/brain/<id>/.system_generated/logs/transcript.jsonl` using kernel `inotify`.
   - Dispatches user prompts via Antigravity subprocess or session socket.
2. **Claude Code Adapter:**
   - Manages Claude Code processes (`claude --dangerously-skip-permissions` or standard interactive mode).
3. **Direct API Adapter:**
   - Directly connects to frontier and open-source models via local proxy **9router** (`http://127.0.0.1:20128`) or cloud provider APIs.

### C. Mode & Thinking Effort Normalization
- **Mode Slider:**
  - `Safe Mode`: Intercepts tool execution and displays an inline `[Approve]` / `[Reject]` prompt for the user.
  - `Auto / YOLO Mode`: Enables autonomous tool execution (`--mode=accept-edits` for Antigravity, `--dangerously-skip-permissions` for Claude Code).
  - `Plan-Only Mode`: Restricts the agent to read-only codebase surveying, generating architectural artifacts without modifying files.
- **Thinking Effort Slider:**
  - Universal UI slider: `Off` | `Low` | `Medium` | `High`.
  - Automatically mapped to model-native budget tokens (e.g., `budget_tokens` on Claude 3.7 Sonnet, `reasoning_effort` on OpenAI o3-mini, `thinkingBudget` on Gemini 2.0 Flash Thinking).
  - Automatically dimmed/disabled when the active model does not support variable reasoning effort.

### D. Skill & MCP Discovery
- **User-Scope Skills:** Scans and indexes global skills from `~/.gemini/antigravity-cli/builtin/skills`, `~/.gemini/skills`, and `~/.agents/skills`.
- **Workspace-Scope Skills:** Discovers project-specific skills from `<workspace>/.agents/skills`.
- **MCP Visualizer:** Intercepts Model Context Protocol (MCP) tool calls (e.g., Neon SQL queries, Context7 lookups) and renders interactive tool cards displaying tool arguments, execution progress, and collapsible output payloads.

### E. File Interaction & Quote Engine
- **File & Directory Mentions (`@file` & `@dir`):** Typing `@` triggers an instant fuzzy search popover scoped to the active git repository (powered by Rust's `ignore` and `walkdir` crates).
- **Drag-and-Drop:** Native QML `DropArea` enables dragging files or screenshots directly from file managers into the chat input.
- **Selection Quoting:** Highlighting any snippet in the agent's response displays a floating `[Quote]` button (or `Alt + Q`) to insert the snippet as a Markdown quote block (`> ...`).
- **Smart Code Block Quoting:** Every code block features a `[Quote]` button that preserves source file path and line number ranges (e.g., `> [src/index.ts:40-55]`).

### F. Top Bar Status & Notifications
- Dynamic top bar icon states:
  - 🟢 **Idle / Ready**
  - 🟡 **Thinking / Executing Command**
  - 🔴 **Action Required (Needs Tool Approval)**
  - 🔵 **Background Task / Subagent Active**
- Desktop notifications via `notify-send` when long background tasks finish or when the agent requests permissions.

---

## 4. Communication Protocol (WebSocket JSON-RPC)

Frontend (QML) and Backend (Rust) communicate over a local WebSocket (`ws://127.0.0.1:21420`):

### Client $\rightarrow$ Daemon
```json
// Send message with quote
{
  "type": "send_message",
  "payload": {
    "text": "Please audit this query performance",
    "attachments": ["/path/to/screenshot.png"],
    "quote": { "source": "src/db.ts:12-20", "text": "const res = await db.query..." }
  }
}

// Approve or reject tool call
{
  "type": "tool_approval",
  "payload": { "tool_call_id": "call_12345", "approved": true }
}

// Update configuration
{
  "type": "update_config",
  "payload": { "platform": "antigravity", "mode": "safe", "thinking": "medium" }
}
```

### Daemon $\rightarrow$ Client
```json
// Token streaming
{
  "type": "token_stream",
  "payload": { "token": "Here is ", "done": false }
}

// Tool execution approval request
{
  "type": "tool_call_request",
  "payload": {
    "tool_call_id": "call_12345",
    "tool_name": "run_command",
    "args": { "CommandLine": "bun test tests/api" },
    "requires_approval": true
  }
}

// Workspace state synchronization
{
  "type": "workspace_sync",
  "payload": {
    "workspace_id": 1,
    "path": "/home/muzaaqi/Projects/MuMu",
    "git_branch": "main",
    "skills_count": 14,
    "mcp_servers": ["neon", "cloudflare", "context7"]
  }
}
```

---

## 5. Implementation Roadmap

| Phase | Milestone | Deliverables |
|---|---|---|
| **Phase 1 (MVP)** | Rust Daemon Core + QML Top Bar | Local WebSocket server, Hyprland workspace listener, Caelestia top bar button, dropdown chat overlay, basic token streaming. |
| **Phase 2** | Project Context & Multi-Platform | `@file` mentions, native drag-and-drop file attachment, 2-tier skill scanner, Antigravity live transcript watcher. |
| **Phase 3** | Execution Safety & Guardrails | Tool call visualizer cards, inline approval buttons, mode switcher (Safe / YOLO / Plan), MCP discovery, inline diff viewer. |
| **Phase 4** | UX Ergonomics & Polish | Selection quote & smart code block quote, universal thinking slider, dynamic status bar icon, desktop notifications. |
| **Phase 5** | Observability & Vision | Token/cost quota tracker, screen snip-to-prompt camera button, artifacts & Mermaid diagram split-view. |

---

## 6. Project Structure

```
~/Projects/Aetheris/
├── daemon/                    # Rust backend daemon
│   ├── Cargo.toml             # Dependencies (tokio, hyprland, notify, ignore, serde)
│   └── src/
│       ├── main.rs            # Entry point & WebSocket server
│       ├── hyprland.rs        # Hyprland IPC listener & workspace tracker
│       ├── transcript.rs      # Kernel inotify watcher for Antigravity transcripts
│       ├── context.rs         # Fast fuzzy file indexer & skill scanner
│       ├── adapters/          # Multi-platform adapters
│       │   ├── mod.rs
│       │   ├── antigravity.rs
│       │   ├── claude.rs
│       │   └── direct_api.rs
│       └── protocol.rs        # JSON-RPC serde data models
├── qml/                       # Caelestia Quickshell components
│   ├── AgentHudButton.qml     # Top bar status icon
│   └── popouts/
│       ├── AgentHudPopout.qml # Glassmorphic dropdown HUD overlay
│       └── components/
│           ├── ChatView.qml
│           ├── ToolCard.qml
│           ├── ModeBar.qml
│           └── FileDropArea.qml
├── docs/
│   └── DESIGN.md              # Full architectural specification
├── systemd/
│   └── aetheris.service       # Systemd user service unit
├── .gitignore
└── README.md                  # Project overview
```
