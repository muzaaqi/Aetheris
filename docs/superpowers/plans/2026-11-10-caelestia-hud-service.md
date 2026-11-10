# Caelestia HUD & Service (Phase 1 Part 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the frontend Caelestia Quickshell QML HUD (top bar status button and glassmorphic popout window with WebSocket communication) and the systemd user service unit to complete the Phase 1 end-to-end communication MVP.

**Architecture:** A systemd user unit manages the lifecycle of `aetheris-daemon`. The frontend is composed of `AgentHudButton.qml` (a Caelestia panel button showing real-time daemon status) and `AgentHudPopout.qml` (a `PanelWindow` with `WlrLayershell` overlay) containing `ChatView.qml`. Communication with the daemon at `ws://127.0.0.1:21420` is handled via QtWebSockets, exchanging JSON-RPC contracts (`send_message` and `token_stream` echo).

**Tech Stack:** Caelestia Shell (Quickshell 0.3.2, Qt 6.12, QtQuick, QtWebSockets, Material 3 design system), systemd user service.

**Spec:** [`DESIGN.md`](file:///home/muzaaqi/Projects/Aetheris/DESIGN.md) (Sections 2, 3, 4) and [`PRODUCT.md`](file:///home/muzaaqi/Projects/Aetheris/PRODUCT.md) (Phase 1 Acceptance Criteria).

## Global Constraints

- 100% adherence to Caelestia Shell design system (`Colours.palette`, `Tokens`, `MaterialIcon`).
- Zero hardcoded hex colors, arbitrary fonts, or non-standard border radiuses (`AGENTS.md` Rule 6).
- Follow all QML rules from `.agents/skills/qt-qml/`: no version numbers on imports (Qt 6), no circular dependencies, declarative property bindings, minimal delegates.
- All QML files must pass `qmllint` with exit code 0.
- Systemd service unit must restart on failure (`Restart=on-failure`) with clean shutdown timeout.
- Short-lived feature branch `feat/qml-hud-scaffold` off clean `main`.
- Conventional commits in imperative English (`AGENTS.md` Rule 9).

## Review Focus

1. **WebSocket Reconnection & Offline Resilience** — when the daemon is stopped, restarted, or not yet running, the QML WebSocket must transition to a disconnected state (🔴) and reconnect cleanly on demand without freezing the UI thread or flooding logs.
2. **Keyboard Focus & Escape Dismissal** — when the HUD popout opens, it must acquire keyboard focus on demand (`WlrKeyboardFocus.OnDemand`), and pressing `Escape` or clicking outside must dismiss the overlay cleanly.
3. **Empty Message & Whitespace Submission** — submitting an empty string or pure whitespace must be ignored, preventing empty JSON-RPC messages from being dispatched to the daemon.
4. **Token Stream Accumulation** — as incremental `token_stream` frames arrive (`done: false` followed by `done: true`), tokens must append reactively to the active assistant bubble without flicker or binding loops.
5. **Systemd Service Environment & Path** — `aetheris.service` must resolve `%h/Projects/Aetheris/daemon/target/release/aetheris-daemon` and handle terminal signals (`SIGINT`/`SIGTERM`) within the 5-second timeout.

---

### Task 1: Systemd User Service Unit

**Files:**
- Create: `systemd/aetheris.service`

**Interfaces:**
- Consumes: `daemon/target/release/aetheris-daemon` binary
- Produces: `systemd/aetheris.service` unit file

- [ ] **Step 1: Write the service file in `systemd/aetheris.service`**

```ini
[Unit]
Description=Aetheris AI Agent Orchestration HUD Daemon
Documentation=https://github.com/muzaaqi/Aetheris
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=%h/Projects/Aetheris/daemon/target/release/aetheris-daemon
Restart=on-failure
RestartSec=3s
KillMode=mixed
TimeoutStopSec=5s
Environment=RUST_LOG=info

[Install]
WantedBy=graphical-session.target
```

- [ ] **Step 2: Validate service file syntax**

Run: `systemd-analyze verify systemd/aetheris.service 2>&1`
Expected: Clean output or standard user-unit dependency notice without syntax errors.

- [ ] **Step 3: Commit**

```bash
git add systemd/aetheris.service
git commit -m "feat(systemd): add aetheris user service unit"
```

---

### Task 2: Top Bar Status Button (`AgentHudButton.qml`)

**Files:**
- Create: `qml/AgentHudButton.qml`

**Interfaces:**
- Consumes: Caelestia components (`Colours.palette`, `Tokens`, `MaterialIcon`, `StateLayer`)
- Produces:
  - `AgentHudButton` component with properties:
    - `property bool isConnected: false`
    - `property bool isThinking: false`
    - `property bool popoutOpen: false`
    - `signal toggled()`

- [ ] **Step 1: Implement `qml/AgentHudButton.qml`**

Design:
- Compact size matching `Tokens.sizes.bar.innerWidth` or standard Caelestia icon buttons.
- StateLayer for hover ripple and pointer cursor (`cursorShape: Qt.PointingHandCursor`).
- `MaterialIcon` displaying `"smart_toy"` (or `"neurology"`).
- Dynamic tinting:
  - Connected & Idle: `Colours.palette.m3primary`
  - Thinking: `Colours.palette.m3tertiary` (pulsing or accent)
  - Disconnected / Error: `Colours.palette.m3outline`
- Small status dot indicator in corner (green = connected, yellow = thinking, red = offline).
- Click emits `toggled()`.

- [ ] **Step 2: Validate with `qmllint`**

Run: `qmllint -I ~/.config/quickshell/caelestia qml/AgentHudButton.qml 2>&1`
Expected: Exit 0.

- [ ] **Step 3: Commit**

```bash
git add qml/AgentHudButton.qml
git commit -m "feat(qml): add AgentHudButton top bar component"
```

---

### Task 3: Chat View Component (`qml/popouts/components/ChatView.qml`)

**Files:**
- Create: `qml/popouts/components/ChatView.qml`

**Interfaces:**
- Consumes:
  - Message model: `ListModel { ListElement { sender: "user" | "assistant", text: "..." } }`
  - `signal sendRequested(string text)`
- Produces:
  - Message scroll area (Flickable / ListView)
  - Styled user bubble (`Colours.palette.m3primaryContainer`) and assistant bubble (`Colours.palette.m3surfaceContainerHigh`)
  - Input bar with `TextInput` / `TextFieldBase`, send button, and keyboard Enter shortcut.

- [ ] **Step 1: Implement `qml/popouts/components/ChatView.qml`**

Design:
- `ListView` with message delegate (user on right, assistant on left).
- Input row at bottom:
  - Text input field with placeholder `"Ask Aetheris or dispatch task..."`
  - Send icon button (`MaterialIcon { text: "arrow_upward" }`)
  - Submitting via Enter (without Shift) triggers `sendRequested(text.trim())` if not empty.
- Clears input on submit.

- [ ] **Step 2: Validate with `qmllint`**

Run: `qmllint -I ~/.config/quickshell/caelestia qml/popouts/components/ChatView.qml 2>&1`
Expected: Exit 0.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ChatView.qml
git commit -m "feat(qml): add ChatView streaming message component"
```

---

### Task 4: Glassmorphic Popout Overlay Window (`qml/popouts/AgentHudPopout.qml`)

**Files:**
- Create: `qml/popouts/AgentHudPopout.qml`

**Interfaces:**
- Consumes:
  - `ChatView.qml` from `components/`
  - `QtWebSockets` (`WebSocket { url: "ws://127.0.0.1:21420" }`)
  - Quickshell `PanelWindow` with `WlrLayershell`
- Produces:
  - Full overlay HUD window with open/close toggle and WebSocket client.

- [ ] **Step 1: Implement `qml/popouts/AgentHudPopout.qml`**

Design:
- `PanelWindow` with `WlrLayershell`:
  - `WlrLayershell.layer: WlrLayer.Overlay`
  - `WlrLayershell.keyboardFocus: root.visible ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None`
- Sizing: width ~480px, height ~640px, anchored top/center below panel bar.
- Background: `StyledRect` or rounded rectangle (`radius: Tokens.rounding.large`) with Material 3 glassmorphic surface (`Colours.tPalette.m3surfaceContainerHigh`).
- Header bar:
  - Title: "Aetheris HUD"
  - Connection status badge: `🟢 ws://127.0.0.1:21420`
  - Close button (`MaterialIcon { text: "close" }`)
- Embedded `WebSocket`:
  - `url: "ws://127.0.0.1:21420"`
  - `active: root.visible` (auto-connects when opened)
  - `onTextMessageReceived`: parses JSON-RPC (`token_stream`, `workspace_sync`), appends streamed token to last assistant message.
- `onSendRequested(text)`:
  - Appends user message to model.
  - Sends `ClientMessage::SendMessage` JSON:
    `JSON.stringify({ type: "send_message", payload: { text: text, attachments: [], quote: null } })`
  - Appends new assistant placeholder to model.
- Keyboard shortcut: `Shortcut { sequence: "Escape"; onActivated: root.visible = false }`.

- [ ] **Step 2: Validate with `qmllint`**

Run: `qmllint -I ~/.config/quickshell/caelestia qml/popouts/AgentHudPopout.qml 2>&1`
Expected: Exit 0.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/AgentHudPopout.qml
git commit -m "feat(qml): add AgentHudPopout glassmorphic overlay window with WebSocket client"
```

---

### Task 5: Standalone Preview Harness & End-to-End Verification

**Files:**
- Create: `qml/shell.qml` (Standalone test harness allowing `quickshell -p qml/` execution)

**Interfaces:**
- Consumes:
  - `AgentHudButton.qml`
  - `popouts/AgentHudPopout.qml`
  - `aetheris-daemon` on `ws://127.0.0.1:21420`
- Produces:
  - Complete standalone testable QML shell.

- [ ] **Step 1: Implement `qml/shell.qml`**

Design:
- `ShellRoot` containing a preview bar with `AgentHudButton` and the `AgentHudPopout` overlay.
- Clicking the button toggles `popout.visible`.

- [ ] **Step 2: Lint the entire `qml/` directory**

Run:
```bash
qmllint -I ~/.config/quickshell/caelestia qml/AgentHudButton.qml qml/popouts/AgentHudPopout.qml qml/popouts/components/ChatView.qml qml/shell.qml
python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/
```
Expected: Exit 0, zero warnings.

- [ ] **Step 3: End-to-End WebSocket Test**
1. Start daemon release binary: `daemon/target/release/aetheris-daemon &`
2. Test WebSocket connection from standalone QML or test script.
3. Send "Hello Aetheris" prompt, verify streamed echo response received.
4. Kill daemon.

- [ ] **Step 4: Commit**

```bash
git add qml/shell.qml
git commit -m "feat(qml): add standalone preview harness and verify end-to-end communication"
```
