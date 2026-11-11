# Phase 3 (HUD Input Redesign: Unified Action Bar, Popups & Inline Selectors) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the Aetheris HUD prompt bar into a unified, modern AI input container matching user-provided reference mockups: integrating the `+` Add Context menu (Files, Folder, @ Mentions), Model selector dropdown, inline Mode selector dropdown (🛡️ Safe, ⚡ YOLO, 📝 Plan), and circular send button directly into the bottom input box, while removing the bulky top ModeBar and retaining drag-and-drop file attachment support.

**Architecture:**
- `ContextMenuPopup.qml`: Floating Material 3 popup menu anchored above the `+` button with options for Files (file picker), Folder (folder picker), and @ Mentions (auto-insert `@`).
- `ModelSelectorPopup.qml`: Floating popup menu anchored above the model button allowing instant switching between backend platforms (`Antigravity`, `Claude Code`, `Direct API`).
- `ModeSelectorPopup.qml`: Compact floating popup menu anchored above the mode button for toggling execution modes (`Safe`, `YOLO`, `Plan`).
- `ChatView.qml`: Consolidates prompt input, drag-and-drop overlay (`FileDropArea`), attachment chips, and the bottom control row (Context `+`, Model pill, Mode pill, Circular Send button) into a single cohesive Material 3 rounded container.
- `AgentHudPopout.qml`: Removes obsolete top `ModeBar` to maximize vertical space for chat history/tool cards, wires new model/mode signals to `update_config` WebSocket RPCs, and connects file selection triggers.

**Tech Stack:** Quickshell / QtQuick (Qt 6), Caelestia Config (`Tokens`, `Colours.palette`, `Colours.tPalette`), `qs.components`, `qs.services`, WebSocket JSON-RPC.

**Spec:** User reference screenshots (`media_1794367643316_aada56c3.png` & `media_1794367682783_bd5d6dd9.png`), `PRODUCT.md` (Phase 3 & Phase 4 Ergonomics), and `DESIGN.md`.

## Global Constraints

- 100% adherence to Caelestia Shell design tokens (`Tokens`, `Colours.palette`, `Colours.tPalette`, `Tokens.font.mono.small`); zero hardcoded hex colors or arbitrary font family strings (AGENTS.md Rule 6).
- All QML files must pass `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts` with 0 warnings/errors.
- All QML files must pass `python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/` with 0 findings.
- Zero binding loops: popup menus must be anchored to parent/container bounds or use explicit placement without circular implicit size bindings.
- All interactive items must use `StateLayer` with `cursorShape: Qt.PointingHandCursor`.
- Audio/mic feature is explicitly omitted from this redesign pass.
- Conventional commits with zero AI attribution (AGENTS.md Rule 9 & 10).

## Review Focus

1. **Popup Dismissal & Mutual Exclusion:** Opening `ContextMenuPopup`, `ModelSelectorPopup`, or `ModeSelectorPopup` must automatically close the other open popups and dismiss cleanly on outside clicks or `Esc`.
2. **Text Area Auto-Sizing with Drag & Drop:** Adding multiple lines of text or dropping 3+ attachment files must cleanly resize the input box without clipping the bottom toolbar or overlapping the chat list view.
3. **Dropdown State Synchronization:** Switching platforms in `ModelSelectorPopup` or modes in `ModeSelectorPopup` must immediately update the button label/icon and dispatch `update_config` to the daemon without desync on reconnect.
4. **`@ Mentions` Trigger via `+` Button:** Clicking `@ Mentions` in `ContextMenuPopup` must programmatically focus the text input, insert `@`, and activate the `FileMentionPopup` without requiring manual keyboard typing.
5. **Send Button State & Interaction:** The circular send button must visually reflect readiness (disabled/faded when input is empty and no attachments exist, active with primary accent when prompt is ready to submit).

---

### Task 1: `ContextMenuPopup.qml` Context Menu Component

**Files:**
- Create: `qml/popouts/components/ContextMenuPopup.qml`

**Interfaces:**
- Produces: `ContextMenuPopup.qml`
  - `property bool isOpen: false`
  - `signal filesRequested()`
  - `signal folderRequested()`
  - `signal mentionsRequested()`
  - `signal dismissed()`
  - `function toggle(): void`
  - `function open(): void`
  - `function close(): void`

- [ ] **Step 1: Implement `qml/popouts/components/ContextMenuPopup.qml`**

Create the floating context menu:
- Styled with `Colours.palette.m3surfaceContainerHigh`, `Tokens.rounding.large`, border `Colours.palette.m3outlineVariant`.
- Header: "Add Context" in `Tokens.font.label.small`, `Colours.palette.m3onSurfaceVariant`.
- Menu items:
  - 📎 **Files**: icon `attach_file`, label `Files`, emits `filesRequested()` and closes.
  - 📁 **Folder**: icon `folder_open`, label `Folder`, emits `folderRequested()` and closes.
  - ＠ **Mentions**: icon `alternate_email`, label `Mentions`, emits `mentionsRequested()` and closes.
- Each item uses `StateLayer` with hover highlight `Colours.palette.m3surfaceContainerHighest` and `cursorShape: Qt.PointingHandCursor`.

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components
import qs.services

StyledRect {
    id: root

    property bool isOpen: false

    signal filesRequested()
    signal folderRequested()
    signal mentionsRequested()
    signal dismissed()

    function toggle(): void {
        root.isOpen = !root.isOpen;
    }

    function open(): void {
        root.isOpen = true;
    }

    function close(): void {
        root.isOpen = false;
        root.dismissed();
    }

    visible: root.isOpen
    implicitWidth: 180
    implicitHeight: menuCol.implicitHeight + Tokens.padding.small * 2
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHigh
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    z: 100

    ColumnLayout {
        id: menuCol

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.small
        }
        spacing: Tokens.spacing.extraSmall

        StyledText {
            Layout.leftMargin: Tokens.padding.small
            Layout.topMargin: Tokens.padding.extraSmall
            Layout.bottomMargin: Tokens.padding.extraSmall
            text: "Add Context"
            font: Tokens.font.label.small
            color: Colours.palette.m3onSurfaceVariant
        }

        Repeater {
            model: [
                { id: "files", label: "Files", icon: "attach_file" },
                { id: "folder", label: "Folder", icon: "folder_open" },
                { id: "mentions", label: "Mentions", icon: "alternate_email" }
            ]

            delegate: StyledRect {
                id: itemRect

                required property var modelData

                Layout.fillWidth: true
                implicitHeight: 32
                radius: Tokens.rounding.medium
                color: "transparent"

                RowLayout {
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.small
                        rightMargin: Tokens.padding.small
                    }
                    spacing: Tokens.spacing.small

                    MaterialIcon {
                        text: itemRect.modelData.icon
                        color: Colours.palette.m3onSurfaceVariant
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                    }

                    StyledText {
                        text: itemRect.modelData.label
                        font: Tokens.font.label.medium
                        color: Colours.palette.m3onSurface
                        Layout.fillWidth: true
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.medium
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.close();
                        if (itemRect.modelData.id === "files") {
                            root.filesRequested();
                        } else if (itemRect.modelData.id === "folder") {
                            root.folderRequested();
                        } else if (itemRect.modelData.id === "mentions") {
                            root.mentionsRequested();
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `ContextMenuPopup.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ContextMenuPopup.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ContextMenuPopup.qml
git commit -m "feat(qml): add ContextMenuPopup component for Add Context action"
```

---

### Task 2: `ModelSelectorPopup.qml` Backend Platform/Model Dropdown Component

**Files:**
- Create: `qml/popouts/components/ModelSelectorPopup.qml`

**Interfaces:**
- Produces: `ModelSelectorPopup.qml`
  - `property bool isOpen: false`
  - `property string currentPlatform: "antigravity"`
  - `signal platformSelected(string platform)`
  - `signal dismissed()`
  - `function toggle(): void`
  - `function close(): void`

- [ ] **Step 1: Implement `qml/popouts/components/ModelSelectorPopup.qml`**

Create the model/platform selector dropdown:
- Supports switching between backend agents:
  - `antigravity` ("Antigravity", icon: `smart_toy`, description: "System & IDE agent")
  - `claude` ("Claude Code", icon: `terminal`, description: "Anthropic agentic CLI")
  - `direct` ("Direct API", icon: `api`, description: "Direct SSE streaming")
- Active model highlighted with `m3primaryContainer` and `check` icon.
- Sized with `implicitWidth: 220`.

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components
import qs.services

StyledRect {
    id: root

    property bool isOpen: false
    property string currentPlatform: "antigravity"

    signal platformSelected(string platform)
    signal dismissed()

    function toggle(): void {
        root.isOpen = !root.isOpen;
    }

    function close(): void {
        root.isOpen = false;
        root.dismissed();
    }

    visible: root.isOpen
    implicitWidth: 220
    implicitHeight: modelCol.implicitHeight + Tokens.padding.small * 2
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHigh
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    z: 100

    ColumnLayout {
        id: modelCol

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.small
        }
        spacing: Tokens.spacing.extraSmall

        StyledText {
            Layout.leftMargin: Tokens.padding.small
            Layout.topMargin: Tokens.padding.extraSmall
            Layout.bottomMargin: Tokens.padding.extraSmall
            text: "Select Model / Platform"
            font: Tokens.font.label.small
            color: Colours.palette.m3onSurfaceVariant
        }

        Repeater {
            model: [
                { id: "antigravity", label: "Antigravity", icon: "smart_toy" },
                { id: "claude", label: "Claude Code", icon: "terminal" },
                { id: "direct", label: "Direct LLM API", icon: "api" }
            ]

            delegate: StyledRect {
                id: platformRect

                required property var modelData

                readonly property bool isSelected: root.currentPlatform === modelData.id

                Layout.fillWidth: true
                implicitHeight: 36
                radius: Tokens.rounding.medium
                color: isSelected ? Colours.palette.m3primaryContainer : "transparent"

                RowLayout {
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.small
                        rightMargin: Tokens.padding.small
                    }
                    spacing: Tokens.spacing.small

                    MaterialIcon {
                        text: platformRect.modelData.icon
                        color: platformRect.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                    }

                    StyledText {
                        text: platformRect.modelData.label
                        font: Tokens.font.label.medium
                        color: platformRect.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurface
                        Layout.fillWidth: true
                    }

                    MaterialIcon {
                        visible: platformRect.isSelected
                        text: "check"
                        color: Colours.palette.m3onPrimaryContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.medium
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.currentPlatform = platformRect.modelData.id;
                        root.platformSelected(platformRect.modelData.id);
                        root.close();
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `ModelSelectorPopup.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ModelSelectorPopup.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ModelSelectorPopup.qml
git commit -m "feat(qml): add ModelSelectorPopup component for agent backend selection"
```

---

### Task 3: `ModeSelectorPopup.qml` Execution Mode Dropdown Component

**Files:**
- Create: `qml/popouts/components/ModeSelectorPopup.qml`

**Interfaces:**
- Produces: `ModeSelectorPopup.qml`
  - `property bool isOpen: false`
  - `property string currentMode: "safe"`
  - `signal modeSelected(string mode)`
  - `signal dismissed()`
  - `function toggle(): void`
  - `function close(): void`

- [ ] **Step 1: Implement `qml/popouts/components/ModeSelectorPopup.qml`**

Create the compact execution mode selector popup:
- Displays 3 modes:
  - 🛡️ `Safe` ("Approval required for commands & edits")
  - ⚡ `YOLO` ("Auto-approve all mutating tools")
  - 📝 `Plan` ("Reject mutations, plan only")
- Includes current active checkmark.

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components
import qs.services

StyledRect {
    id: root

    property bool isOpen: false
    property string currentMode: "safe"

    signal modeSelected(string mode)
    signal dismissed()

    function toggle(): void {
        root.isOpen = !root.isOpen;
    }

    function close(): void {
        root.isOpen = false;
        root.dismissed();
    }

    visible: root.isOpen
    implicitWidth: 200
    implicitHeight: modeCol.implicitHeight + Tokens.padding.small * 2
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHigh
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    z: 100

    ColumnLayout {
        id: modeCol

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.small
        }
        spacing: Tokens.spacing.extraSmall

        StyledText {
            Layout.leftMargin: Tokens.padding.small
            Layout.topMargin: Tokens.padding.extraSmall
            Layout.bottomMargin: Tokens.padding.extraSmall
            text: "Execution Mode"
            font: Tokens.font.label.small
            color: Colours.palette.m3onSurfaceVariant
        }

        Repeater {
            model: [
                { id: "safe", label: "Safe Mode", icon: "shield" },
                { id: "yolo", label: "YOLO Mode", icon: "bolt" },
                { id: "plan", label: "Plan-Only Mode", icon: "edit_note" }
            ]

            delegate: StyledRect {
                id: modeRect

                required property var modelData

                readonly property bool isSelected: root.currentMode === modelData.id

                Layout.fillWidth: true
                implicitHeight: 36
                radius: Tokens.rounding.medium
                color: isSelected ? Colours.palette.m3primaryContainer : "transparent"

                RowLayout {
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.small
                        rightMargin: Tokens.padding.small
                    }
                    spacing: Tokens.spacing.small

                    MaterialIcon {
                        text: modeRect.modelData.icon
                        color: modeRect.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                    }

                    StyledText {
                        text: modeRect.modelData.label
                        font: Tokens.font.label.medium
                        color: modeRect.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurface
                        Layout.fillWidth: true
                    }

                    MaterialIcon {
                        visible: modeRect.isSelected
                        text: "check"
                        color: Colours.palette.m3onPrimaryContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.medium
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.currentMode = modeRect.modelData.id;
                        root.modeSelected(modeRect.modelData.id);
                        root.close();
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `ModeSelectorPopup.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ModeSelectorPopup.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ModeSelectorPopup.qml
git commit -m "feat(qml): add ModeSelectorPopup component for execution mode selection"
```

---

### Task 4: `ChatView.qml` Unified Input Bar Redesign

**Files:**
- Modify: `qml/popouts/components/ChatView.qml`

**Interfaces:**
- Produces:
  - `property string currentPlatform: "antigravity"`
  - `property string currentMode: "safe"`
  - `signal modeSelected(string mode)`
  - `signal platformSelected(string platform)`
  - `signal filesRequested()`
  - `signal folderRequested()`
- Consumes:
  - `ContextMenuPopup.qml`
  - `ModelSelectorPopup.qml`
  - `ModeSelectorPopup.qml`

- [ ] **Step 1: Redesign bottom input container in `ChatView.qml`**

1. Replace old separate text container with unified rounded pill container:
   - Outer `StyledRect` (`radius: Tokens.rounding.extraLarge`, background `Colours.palette.m3surfaceContainerHigh`, border `Colours.palette.m3outlineVariant`).
   - Drag & Drop: Wrap with `FileDropArea` so dropping files highlights the input container and appends attachments.
   - Top: `attachmentBar` (if `root.attachments.length > 0`).
   - Middle: Text input with placeholder `"Ask anything, @ to mention, / for actions"` and auto-expansion (`font: Tokens.font.body.medium`).
   - Bottom Toolbar Row:
     - Left:
       - `+` button (`add` icon, opens `ContextMenuPopup`).
       - Model button (Icon + Label e.g. "Antigravity" + `expand_less` chevron, opens `ModelSelectorPopup`).
     - Spacer `Item { Layout.fillWidth: true }`.
     - Right:
       - Mode button (Icon e.g. `shield` / `bolt` / `edit_note` + Label + `expand_less` chevron, opens `ModeSelectorPopup`).
       - Circular Send button (36x36 circular button with `arrow_forward` icon, `Colours.palette.m3primary`).
2. Anchor `ContextMenuPopup`, `ModelSelectorPopup`, and `ModeSelectorPopup` above the input container with `bottom: inputCard.top`, `bottomMargin: Tokens.spacing.small`.
3. Mutual exclusion: opening one popup closes the other two.
4. Clicking `@ Mentions` inserts `@` into the text box and calls `checkMention()`.

- [ ] **Step 2: Run qmllint on `ChatView.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ChatView.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ChatView.qml
git commit -m "feat(qml): redesign ChatView bottom input bar with integrated toolbar and popups"
```

---

### Task 5: `AgentHudPopout.qml` Integration & Top ModeBar Removal

**Files:**
- Modify: `qml/popouts/AgentHudPopout.qml`

**Interfaces:**
- Consumes: `ChatView`'s `modeSelected`, `platformSelected`, `filesRequested`.
- Produces: `dispatchConfigUpdate(mode, thinking, platform)` over WebSocket.

- [ ] **Step 1: Remove `ModeBar` from top and wire new `ChatView` signals**

1. Remove `ModeBar` from between the context header and `ChatView` in `AgentHudPopout.qml`.
2. Update `dispatchConfigUpdate` to accept `platform`:
   ```javascript
   function dispatchConfigUpdate(mode, thinking, platform) {
       if (ws.status === WebSocket.Open) {
           ws.sendTextMessage(JSON.stringify({
               type: "update_config",
               payload: {
                   platform: platform || chatView.currentPlatform,
                   mode: mode || chatView.currentMode,
                   thinking: thinking || "off"
               }
           }));
       }
   }
   ```
3. Wire `chatView`:
   - `onModeSelected: function(mode) { root.dispatchConfigUpdate(mode, "off", chatView.currentPlatform); }`
   - `onPlatformSelected: function(plat) { root.dispatchConfigUpdate(chatView.currentMode, "off", plat); }`
   - `onFilesRequested: function() { /* invoke file selection / dialog */ }`
4. On WebSocket reconnect, sync `chatView.currentMode` and `chatView.currentPlatform`.

- [ ] **Step 2: Run qmllint on `AgentHudPopout.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/AgentHudPopout.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/AgentHudPopout.qml
git commit -m "feat(qml): remove top ModeBar and connect unified input bar controls to WebSocket RPCs"
```

---

### Task 6: Preview Verification & End-to-End Quality Gates

**Files:**
- Modify: `scripts/preview.sh` (ensure preview loads new components cleanly)
- Verify: Full QML and Rust workspace

- [ ] **Step 1: Verify standalone preview execution**

Run: `timeout 5 ./scripts/preview.sh`
Expected: Launches cleanly with `INFO: Configuration Loaded` and 0 errors.

- [ ] **Step 2: Run complete project quality checks**

```bash
qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/AgentHudButton.qml qml/popouts/AgentHudPopout.qml qml/popouts/components/ChatView.qml qml/popouts/components/FileDropArea.qml qml/popouts/components/FileMentionPopup.qml qml/popouts/components/ContextMenuPopup.qml qml/popouts/components/ModelSelectorPopup.qml qml/popouts/components/ModeSelectorPopup.qml qml/popouts/components/DiffViewer.qml qml/popouts/components/ToolCard.qml qml/shell.qml
python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/
cargo test --manifest-path daemon/Cargo.toml
cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings
cargo fmt --check --manifest-path daemon/Cargo.toml
```
Expected: All linters and tests exit with code 0.

- [ ] **Step 3: Commit any preview script adjustments**

```bash
git add scripts/preview.sh
git commit -m "chore(qml): verify redesigned input bar in standalone preview"
```
