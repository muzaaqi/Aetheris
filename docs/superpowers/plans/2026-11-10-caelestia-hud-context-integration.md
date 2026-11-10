# Phase 2 (Part 2) — Caelestia HUD Context Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 2 frontend HUD capabilities in Caelestia Shell: native drag-and-drop file/image attachments, `@file` fuzzy search mention popup with keyboard navigation, and dynamic workspace context header bar (active project path, git branch badge, skills count).

**Architecture:** Create `FileDropArea.qml` and `FileMentionPopup.qml` in `qml/popouts/components/`, extend `ChatView.qml` with attachment chip list and `@` token tracking, and update `AgentHudPopout.qml` to bind to daemon WebSocket `workspace_sync` and `file_results` JSON-RPC messages.

**Tech Stack:** Caelestia Quickshell (QtQuick, Qt 6, QML), Material 3 tokens (`Caelestia.Config`, `Tokens`, `Colours.palette`), `QtQuick.Layouts`, `QtWebSockets`.

**Spec:** [`DESIGN.md`](file:///home/muzaaqi/Projects/Aetheris/DESIGN.md) & [`PRODUCT.md`](file:///home/muzaaqi/Projects/Aetheris/PRODUCT.md) (Epic 1, 3, 5 & Phase 2 Acceptance Criteria).

## Global Constraints

- **Design Harmony (Rule 6):** 100% adherence to Caelestia design system (`Tokens`, `Colours.palette`, `Colours.tPalette`). Never hardcode hex colors or arbitrary font sizes.
- **Platform Abstraction (Rule 7):** Frontend QML communicates strictly via universal JSON-RPC contracts over WebSocket (`ws://127.0.0.1:21420`).
- **QML Quality (Rule 6):** Zero `qmllint` warnings or syntax errors. Omit TypeScript `: void` return annotations.
- **Trunk Hygiene (Rule 10):** All changes developed on short-lived branch `feat/qml-context-integration` with passing quality gates.

## Review Focus

1. **Email addresses or `@` with whitespace:** Typing email addresses (e.g. `user@example.com`) or `@` followed by whitespace must dismiss the mention popup without dispatching spurious file searches. Pinned in Task 3.
2. **Unsupported drag MIME types (non-file drags):** `FileDropArea` must guard with `drop.hasUrls` and filter only local `file://` URLs, ignoring arbitrary internal drags. Pinned in Task 1.
3. **Cursor positioning after `@file` completion:** Inserting a file path must replace the active `@query` token, append a trailing space, and move the text cursor to the end without breaking return submission. Pinned in Task 3.
4. **Rapid workspace switches:** Switching workspace while files or prompts are in progress must update the header path and git branch badge cleanly without undefined reference errors. Pinned in Task 4.
5. **Non-git or zero-skill workspace:** Header must handle null/empty `git_branch` and format `skills_count` with neutral Material 3 styling. Pinned in Task 4.

---

### Task 1: Native File & Image Drag-and-Drop Area with Attachment Chips

**Files:**
- Create: `qml/popouts/components/FileDropArea.qml`
- Modify: `qml/popouts/components/ChatView.qml`

**Interfaces:**
- Produces: `FileDropArea.qml` with signal `filesDropped(var filePaths)`
- Consumes: `Tokens`, `Colours.palette`, `MaterialIcon`
- Extends `ChatView.qml`:
  - `property var attachments: []`
  - `signal sendRequested(string text, var attachments)`
  - Attachment chips row above input field showing icon, basename, and remove button.

- [ ] **Step 1: Create `qml/popouts/components/FileDropArea.qml`**

Implement `DropArea` with visual overlay when `containsDrag` is true. Strip `file://` prefixes from `drop.urls`. Emit `filesDropped(paths)`.

```qml
import QtQuick
import Caelestia.Config
import qs.components

DropArea {
    id: root

    signal filesDropped(var paths)

    Rectangle {
        anchors.fill: parent
        visible: root.containsDrag
        color: Colours.tPalette.m3primary
        opacity: 0.15
        radius: Tokens.rounding.large
        border.color: Colours.palette.m3primary
        border.width: 2
        z: 100

        StyledText {
            anchors.centerIn: parent
            text: "Drop files or screenshots here to attach"
            color: Colours.palette.m3primary
            font: Tokens.font.title.small
        }
    }

    onDropped: function(drop) {
        if (!drop.hasUrls) return;
        const paths = [];
        for (let i = 0; i < drop.urls.length; i++) {
            let u = drop.urls[i].toString();
            if (u.startsWith("file://")) {
                u = u.substring(7);
            }
            if (u.length > 0) {
                paths.push(decodeURIComponent(u));
            }
        }
        if (paths.length > 0) {
            root.filesDropped(paths);
            drop.acceptProposedAction();
        }
    }
}
```

- [ ] **Step 2: Add Attachment Chip Row to `qml/popouts/components/ChatView.qml`**

Add `property var attachments: []` to `ChatView.qml`.
Render a horizontal `ListView` or `RowLayout` of attachment chips when `attachments.length > 0` directly above `inputContainer`.
Update `submitPrompt()` to pass `root.attachments` to `sendRequested(trimmed, root.attachments.slice())` and reset `root.attachments = []`.

- [ ] **Step 3: Run qmllint to verify syntax and tokens**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/FileDropArea.qml qml/popouts/components/ChatView.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 4: Commit**

```bash
git add qml/popouts/components/FileDropArea.qml qml/popouts/components/ChatView.qml
git commit -m "feat(qml): add FileDropArea and attachment chip list to ChatView"
```

---

### Task 2: `@file` Mention Fuzzy Search Popup Component

**Files:**
- Create: `qml/popouts/components/FileMentionPopup.qml`

**Interfaces:**
- Produces: `FileMentionPopup.qml`
  - `property var matches: []`
  - `property int selectedIndex: 0`
  - `signal fileSelected(string path)`
  - `signal dismissed()`
  - `function selectNext()`
  - `function selectPrevious()`
  - `function triggerSelection()`

- [ ] **Step 1: Implement `qml/popouts/components/FileMentionPopup.qml`**

Create a compact floating Material 3 card displaying matching files:
- Styled with `Colours.palette.m3surfaceContainerHighest`, `Tokens.rounding.medium`.
- Shows file icon (`description` or `folder`), matched path string, and highlighted background for `selectedIndex`.
- Maximum height capped to 5 visible items with smooth scroll.
- Clicking an item emits `fileSelected(path)`.

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property var matches: []
    property int selectedIndex: 0
    signal fileSelected(string path)
    signal dismissed()

    implicitWidth: 360
    implicitHeight: Math.min(matchListView.contentHeight + Tokens.padding.small * 2, 220)
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHighest
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    clip: true

    function selectNext() {
        if (root.matches.length > 0) {
            root.selectedIndex = (root.selectedIndex + 1) % root.matches.length;
            matchListView.positionViewAtIndex(root.selectedIndex, ListView.Contain);
        }
    }

    function selectPrevious() {
        if (root.matches.length > 0) {
            root.selectedIndex = (root.selectedIndex - 1 + root.matches.length) % root.matches.length;
            matchListView.positionViewAtIndex(root.selectedIndex, ListView.Contain);
        }
    }

    function triggerSelection() {
        if (root.selectedIndex >= 0 && root.selectedIndex < root.matches.length) {
            root.fileSelected(root.matches[root.selectedIndex].path);
        }
    }

    ListView {
        id: matchListView

        anchors {
            fill: parent
            margins: Tokens.padding.small
        }
        spacing: 2
        model: root.matches
        boundsBehavior: Flickable.StopAtBounds

        delegate: StyledRect {
            id: itemRect

            required property var modelData
            required property int index

            width: matchListView.width
            implicitHeight: 36
            radius: Tokens.rounding.small
            color: index === root.selectedIndex ? Colours.palette.m3primaryContainer : "transparent"

            RowLayout {
                anchors {
                    fill: parent
                    leftMargin: Tokens.padding.small
                    rightMargin: Tokens.padding.small
                }
                spacing: Tokens.spacing.small

                MaterialIcon {
                    text: itemRect.modelData.is_dir ? "folder" : "description"
                    color: itemRect.index === root.selectedIndex ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                }

                StyledText {
                    text: itemRect.modelData.path
                    color: itemRect.index === root.selectedIndex ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurface
                    font: Tokens.font.body.small
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
            }

            StateLayer {
                anchors.fill: parent
                radius: Tokens.rounding.small
                onClicked: {
                    root.selectedIndex = itemRect.index;
                    root.triggerSelection();
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `FileMentionPopup.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/FileMentionPopup.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/FileMentionPopup.qml
git commit -m "feat(qml): add FileMentionPopup component for fuzzy file selection"
```

---

### Task 3: `@file` Mention Tracking and Autocomplete in `ChatView.qml`

**Files:**
- Modify: `qml/popouts/components/ChatView.qml`

**Interfaces:**
- Produces:
  - `signal fileSearchRequested(string query)`
  - `property alias fileMatches: mentionPopup.matches`
- Consumes: `FileMentionPopup.qml`

- [ ] **Step 1: Integrate mention query tracking in `ChatView.qml`**

In `inputField`:
- Listen to `onTextChanged`: Check if current text contains `@`. Find the last `@`. If followed by alphanumeric characters without whitespace, set `mentionQuery = substringAfterLastAt` and emit `root.fileSearchRequested(mentionQuery)`. If followed by whitespace or no `@`, hide popup.
- Intercept navigation keys in `Keys.onPressed`:
  - `Qt.Key_Down` / `Qt.Key_Up`: Forward to `mentionPopup.selectNext()` / `selectPrevious()` when popup is visible.
  - `Qt.Key_Return` / `Qt.Key_Enter`: If popup is visible and has matches, call `mentionPopup.triggerSelection()` and accept event.
  - `Qt.Key_Escape`: Close popup if visible.
- On `mentionPopup.fileSelected(path)`:
  - Replace `@query` with `@path `.
  - Move cursor to end.
  - Hide popup.

- [ ] **Step 2: Position `FileMentionPopup` anchored above `inputContainer`**

Anchor `mentionPopup` to `bottom: inputContainer.top` with `bottomMargin: Tokens.spacing.small`.
Set `visible: mentionPopup.matches.length > 0 && mentionQueryActive`.

- [ ] **Step 3: Run qmllint on `ChatView.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ChatView.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 4: Commit**

```bash
git add qml/popouts/components/ChatView.qml
git commit -m "feat(qml): integrate @file mention tracking and keyboard autocomplete into ChatView"
```

---

### Task 4: Dynamic Workspace Context Header & WebSocket RPC Binding

**Files:**
- Modify: `qml/popouts/AgentHudPopout.qml`

**Interfaces:**
- Consumes:
  - WebSocket `workspace_sync` (`workspace_id`, `path`, `git_branch`, `skills_count`, `mcp_servers`)
  - WebSocket `file_results` (`query`, `matches`)
- Produces:
  - Header bar with project name pill, git branch badge, and skills count chip.
  - WebSocket forwarding of `search_files` requests and `send_message` with `attachments`.

- [ ] **Step 1: Add Context Header Pills to `AgentHudPopout.qml`**

In the Header bar `StyledRect`:
- Display active project directory name (e.g. `📁 Aetheris`).
- When `currentGitBranch` is non-empty, render git branch chip (`git_branch` icon + branch name) in `Colours.palette.m3secondaryContainer`.
- Render skills badge (`bolt` + `X skills`) in `Colours.palette.m3tertiaryContainer`.

- [ ] **Step 2: Connect `workspace_sync` and `file_results` in `ws.onTextMessageReceived`**

```javascript
if (data.type === "workspace_sync") {
    root.currentWorkspacePath = data.payload.path || "";
    root.currentGitBranch = data.payload.git_branch || "";
    root.skillsCount = data.payload.skills_count || 0;
} else if (data.type === "file_results") {
    chatView.fileMatches = data.payload.matches || [];
}
```

- [ ] **Step 3: Forward `search_files` and `attachments`**

- In `chatView.onFileSearchRequested`:
  ```javascript
  ws.sendTextMessage(JSON.stringify({
      type: "search_files",
      payload: { query: query, limit: 8 }
  }));
  ```
- In `dispatchPrompt(text, attachments)`:
  Send payload with `attachments: attachments || []`.

- [ ] **Step 4: Run qmllint on `AgentHudPopout.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/AgentHudPopout.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 5: Commit**

```bash
git add qml/popouts/AgentHudPopout.qml
git commit -m "feat(qml): add workspace context pills, git branch badge, and file search RPC"
```

---

### Task 5: End-to-End HUD Integration & Live Preview Verification

**Files:**
- Modify: `qml/shell.qml`
- Verify: Full QML and Rust quality gates

**Interfaces:**
- Produces: Fully interactive HUD with `@file` mentions, drag-and-drop attachments, and workspace header.

- [ ] **Step 1: Update `qml/shell.qml` to bind active workspace and popout states**

Ensure `shell.qml` hosts `AgentHudButton` and `AgentHudPopout` cleanly for preview.

- [ ] **Step 2: Run full linter and quality test suite**

```bash
qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/AgentHudButton.qml qml/popouts/AgentHudPopout.qml qml/popouts/components/ChatView.qml qml/popouts/components/FileDropArea.qml qml/popouts/components/FileMentionPopup.qml qml/shell.qml
python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/
cargo test --manifest-path daemon/Cargo.toml
```
Expected: All linters and tests exit with code 0.

- [ ] **Step 3: Commit**

```bash
git add qml/shell.qml
git commit -m "chore(qml): verify standalone HUD preview with context pills and mention autocomplete"
```
