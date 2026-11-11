# PRODUCT.md — Aetheris Product Requirements

> **Product:** Aetheris  
> **Type:** OS-Level AI Agent HUD  
> **Target OS:** Linux (Hyprland Compositor + Caelestia Shell)  
> **Status:** Approved / In Development

---

## 1. Product Overview & Vision

**Aetheris** transforms the Linux desktop into an ambient, context-aware AI agent workstation. Embedded directly into the top bar of **Hyprland** through **Caelestia Shell**, Aetheris provides developers with an omnipresent HUD (Heads-Up Display) that communicates with multiple AI coding agents (Antigravity, Claude Code, and direct frontier models).

### The Problem It Solves

Modern developers using AI coding agents face significant workflow friction:
1. **Window Juggling & Context Loss:** Terminal agents run in buried windows or tabs. Switching between editor, browser, and agent requires constant window toggling.
2. **Workspace Amnesia:** Standard chat UIs don't know which project or Git branch the user is working on in their current desktop workspace.
3. **Approval Bottlenecks:** Agent CLI sessions require manual terminal monitoring. If an agent pauses to ask for tool execution permission, it sits silently blocked in an obscured terminal.
4. **Poor Media & Code Ergonomics:** Dragging screenshots, attaching files, or quoting specific code blocks into terminal CLIs is cumbersome.

### The Aetheris Solution

Aetheris eliminates this friction by making the agent **ambient, omnipresent, and native**:
- One click or shortcut (`SUPER + Space`) summons the HUD right from the top panel.
- Switching to Workspace 1 (e.g., `~/Projects/MuMu`) automatically loads MuMu's project context, git branch, skills, and chat session.
- Tool approvals pop up directly as interactive UI cards in the HUD with desktop notifications.
- Dragging a file or quoting a snippet into the HUD is effortless and native.

---

## 2. Target Personas & Primary Use Cases

### Primary Persona: Linux Power Developer
- Uses Hyprland tiling window manager for multi-monitor, workspace-driven development.
- Regularly uses AI coding agents (Antigravity CLI, Claude Code) to build, refactor, and test production software.
- Values ultra-low resource usage, instant responsiveness, and keyboard-driven ergonomics.

### Core Use Cases:
1. **Ambient Architecture Advisory:** Asking architectural questions, reviewing API schemas, or planning features while glancing at the top bar HUD without breaking focus on the active editor.
2. **Context-Driven Multitasking:** Developing a backend in Workspace 1, switching to Workspace 2 to tweak Hyprland dotfiles, and having Aetheris seamlessly swap active agent sessions and scoped skills on the fly.
3. **Safe Autonomous Execution:** Running the agent in `Safe Mode` to review every file write and shell command via visual diffs and approval cards before committing.
4. **Instant Bug Reporting & Vision:** Snapping a broken UI element on screen or dropping a crash log file directly into the HUD to prompt the agent for diagnosis.

---

## 3. Functional Requirements

### Epic 1: Workspace Context Awareness
- **FR-1.1:** The system must automatically detect Hyprland workspace switches via Hyprland IPC sockets.
- **FR-1.2:** The system must resolve the active project root directory based on the focused window process in the workspace.
- **FR-1.3:** The system must isolate chat sessions per workspace directory and automatically restore the corresponding session upon workspace entry.
- **FR-1.4:** The system must detect and display the active Git branch and uncommitted status for the workspace.

### Epic 2: Multi-Platform Agent Orchestration
- **FR-2.1:** The system must provide a platform switcher supporting **Antigravity**, **Claude Code**, and **Direct LLM APIs** (via 9router / OpenRouter).
- **FR-2.2:** For Antigravity, the system must stream tokens and tool calls in real-time by watching `transcript.jsonl` using kernel `inotify`.
- **FR-2.3:** For Claude Code, the system must manage CLI sessions and process stdio streams.
- **FR-2.4:** For Direct APIs, the system must stream SSE responses with selectable model backends (Claude 3.7 Sonnet, Gemini 2.0 Flash/Pro, GPT-4o, DeepSeek R1).

### Epic 3: Hierarchical Skill & MCP Discovery
- **FR-3.1:** The system must automatically scan and list User-Scope skills (`~/.gemini/antigravity-cli/builtin/skills`, `~/.gemini/skills`, `~/.agents/skills`).
- **FR-3.2:** The system must automatically discover Workspace-Scope skills (`<workspace>/.agents/skills`).
- **FR-3.3:** The system must detect Model Context Protocol (MCP) servers configured globally or in the active workspace.
- **FR-3.4:** The system must render MCP tool executions as interactive cards showing arguments and formatted results.

### Epic 4: Human-in-the-Loop Safety & Execution Modes
- **FR-4.1:** The system must provide a universal mode switcher:
  - `Safe Mode`: All mutations require user approval via `[Approve]` / `[Reject]` UI buttons.
  - `Auto / YOLO Mode`: Automated execution (`--mode=accept-edits` / `--dangerously-skip-permissions`).
  - `Plan-Only Mode`: Read-only architectural planning without file write permissions.
- **FR-4.2:** The system must provide an inline collapsible code diff viewer (green/red additions and deletions) before code modifications are saved.

### Epic 5: UX Ergonomics & Quoting Engine
- **FR-5.1:** The system must allow highlighting any text snippet in agent responses to display a `[Quote]` button that inserts the text as a Markdown blockquote (`> ...`) into the input box.
- **FR-5.2:** Every code block in agent messages must feature a `[Quote]` button that attaches file path and line number metadata (`> [src/index.ts:15-30]`).
- **FR-5.3:** Typing `@` in the input field must trigger an instant fuzzy file search popup scoped to the active project repository.
- **FR-5.4:** The input box must support native drag-and-drop file and image attachments.
- **FR-5.5:** The system must provide a universal Reasoning / Thinking slider (`Off`, `Low`, `Medium`, `High`) mapped dynamically to the active model's reasoning budget.

### Epic 6: System Integration & Status HUD
- **FR-6.1:** The top panel bar must feature a dynamic status icon indicating agent state (🟢 Idle, 🟡 Thinking/Running, 🔴 Needs Approval, 🔵 Background Task).
- **FR-6.2:** The system must send desktop notifications via `notify-send` when background tasks complete or when user approval is needed.
- **FR-6.3:** The HUD overlay must support a split-tab view: `[Chat & Tools]` and `[Artifacts & Plans]` for reviewing structured plans and Mermaid diagrams.

---

## 4. Non-Functional Requirements

- **Performance & Footprint:**
  - The Rust daemon must consume **< 15 MB RAM** at steady state.
  - CPU usage when idle must remain **< 0.1%**.
  - WebSocket round-trip message dispatch latency must remain **< 5 ms**.
- **Visual Fidelity:**
  - 100% adherence to Caelestia Shell's Material 3 design system, colors (`scheme.current`), blur, and corner rounding.
  - Responsive layout scaling across 1080p, 1440p, and 4K displays.
- **Stability & Reliability:**
  - Daemon must automatically reconnect to Hyprland socket if the compositor restarts.
  - Background daemon must run as a systemd user service with zero-downtime crash recovery.

---

## 5. Phased Milestones & Acceptance Criteria

### Phase 1: Core Foundation & Communication MVP
- [x] Rust daemon boots and listens on `ws://127.0.0.1:21420`.
- [x] Hyprland workspace listener tracks active workspace and logs directory changes.
- [x] Caelestia top bar button renders and toggles the basic dropdown QML chat window.
- [x] Basic prompt submission sends text to daemon and receives streamed echo tokens.

### Phase 2: Context Awareness & Multi-Platform
- [x] Live Antigravity transcript watcher streams active assistant messages to the HUD.
- [x] Workspace path auto-switches active session when moving between Hyprland workspaces.
- [x] `@file` mention fuzzy search popover works with project files.
- [x] File and image drag-and-drop into QML target functional.
- [x] 2-tier skill scanner indexes global and workspace skills.

### Phase 3: Safety Guardrails & Tool Execution
- [x] Inline Tool Cards render for tool calls (`run_command`, `write_file`, MCP).
- [x] Interactive `[Approve]` and `[Reject]` buttons intercept tool execution in Safe Mode.
- [x] Universal Mode Switcher (`Safe`, `YOLO`, `Plan-Only`) functional.
- [x] Collapsible inline diff viewer renders code modifications.

### Phase 4: Ergonomics & Thinking Controls
- [ ] Selection text quoting and smart code-block quoting functional.
- [ ] Universal Thinking slider maps to Claude 3.7 / o3-mini / Gemini budgets.
- [ ] Top bar icon animates and displays dynamic status colors (🟢, 🟡, 🔴, 🔵).
- [ ] Desktop notifications fire on completion and approval requests.

### Phase 5: Polish, Quota & Vision
- [ ] Token usage counter and quota tracking displayed in header.
- [ ] Camera button triggers screen snip and attaches image to prompt.
- [ ] Artifacts split-tab renders Markdown plans and Mermaid diagrams.
