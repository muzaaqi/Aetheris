# Phase 3 (Part 2 — Caelestia HUD: ToolCard, ModeBar & Diff Viewer) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the frontend safety interception HUD in Caelestia Quickshell (QtQuick/QML), featuring the `ModeBar` execution mode selector (`Safe`, `YOLO`, `Plan-Only`), `DiffViewer` syntax-highlighted diff display, interactive `ToolCard` with `[Approve]` and `[Reject]` actions, and full WebSocket RPC wiring.

**Architecture:**
- `ModeBar.qml`: Material 3 segmented pill selector anchored in the popout header, synchronizing `ExecutionMode` and `ThinkingBudget` with the daemon via `update_config`.
- `DiffViewer.qml`: Collapsible unified diff viewer rendering color-coded additions (`m3tertiaryContainer`), deletions (`m3errorContainer`), and hunk context.
- `ToolCard.qml`: Interactive card for tool executions rendering tool metadata, collapsible diffs, parameter previews, and one-click `[Approve]` / `[Reject]` action buttons.
- `ChatView.qml`: Dynamically switches between chat bubble delegates and `ToolCard` delegates based on message type.
- `AgentHudPopout.qml`: Coordinates WebSocket events (`tool_call_request`, `tool_call_result`, `tool_approval`, `update_config`).

**Tech Stack:** Quickshell / QtQuick (Qt 6), Caelestia Config (`Tokens`, `Colours.palette`, `Colours.tPalette`), WebSocket JSON-RPC.

**Spec:** `PRODUCT.md` (Epic 4: Human-in-the-Loop Safety, Phase 3 Milestones) & `DESIGN.md` (Section 3: Human-in-the-Loop Safety Interception).

## Global Constraints

- 100% adherence to Caelestia Shell design tokens (`Tokens`, `Colours.palette`, `Colours.tPalette`); zero hardcoded hex colors or arbitrary font definitions (AGENTS.md Rule 6).
- All QML files must pass `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts` with 0 warnings/errors.
- All QML files must pass `python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/` with 0 findings.
- Zero binding loops, proper anchor scopes (siblings or parent only), no cross-branch anchoring.
- Clean imperative commits adhering to Conventional Commits with zero AI attribution (AGENTS.md Rule 9 & 10).

## Review Focus

1. **Empty or Truncated Diffs:** If `diff` is empty or truncated, `DiffViewer` must display appropriately without rendering broken empty lines.
2. **Double Approval Prevention:** Once `[Approve]` or `[Reject]` is clicked, buttons must be disabled immediately so repeated clicks don't send duplicate RPCs.
3. **ModeBar Keyboard & Mouse Accessibility:** Clicking each mode tab instantly highlights the active segment and dispatches `update_config` over WebSocket without flicker or binding loops.
4. **ListModel Schema Consistency:** Dynamic properties in `chatModel` (`sender`, `text`, `toolCallId`, `toolName`, `toolArgs`, `requiresApproval`, `status`, `diff`) must provide default values so QtQuick ListModel does not drop properties across mixed delegate types.
5. **Layout Resizing on Diff Expand/Collapse:** Expanding or collapsing a 20-line diff inside `ToolCard` must trigger clean ListView height recalculations without clipping or jumpy scrolls.

---

### Task 1: `ModeBar.qml` Segmented Execution Mode & Thinking Selector

**Files:**
- Create: `qml/popouts/components/ModeBar.qml`

**Interfaces:**
- Produces: `ModeBar.qml`
  - `property string currentMode: "safe"` (`"safe"`, `"yolo"`, `"plan"`)
  - `property string thinkingBudget: "off"` (`"off"`, `"low"`, `"medium"`, `"high"`)
  - `signal modeSelected(string mode)`
  - `signal thinkingSelected(string budget)`

- [ ] **Step 1: Implement `qml/popouts/components/ModeBar.qml`**

Create a compact Material 3 segmented pill container:
- Renders 3 execution mode buttons:
  - 🛡️ `Safe` (`mode === "safe"`)
  - ⚡ `YOLO` (`mode === "yolo"`)
  - 📝 `Plan` (`mode === "plan"`)
- Active button highlighted with `Colours.palette.m3primaryContainer` and `Colours.palette.m3onPrimaryContainer`.
- Inactive buttons styled with `Colours.palette.m3surfaceContainerHigh` and `Colours.palette.m3onSurfaceVariant`.
- Clicking a mode updates `currentMode` and emits `modeSelected(mode)`.
- Includes a thinking budget pill toggling `off` -> `low` -> `medium` -> `high` with brain icon (`psychology`).

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property string currentMode: "safe"
    property string thinkingBudget: "off"

    signal modeSelected(string mode)
    signal thinkingSelected(string budget)

    implicitHeight: 38
    radius: Tokens.rounding.full
    color: Colours.palette.m3surfaceContainer

    RowLayout {
        anchors {
            fill: parent
            leftMargin: Tokens.padding.small
            rightMargin: Tokens.padding.small
        }
        spacing: Tokens.spacing.extraSmall

        // Mode Segmented Pills
        RowLayout {
            spacing: Tokens.spacing.extraSmall

            Repeater {
                model: [
                    { id: "safe", label: "Safe", icon: "shield" },
                    { id: "yolo", label: "YOLO", icon: "bolt" },
                    { id: "plan", label: "Plan", icon: "edit_note" }
                ]

                delegate: StyledRect {
                    id: modeBtn

                    required property var modelData

                    readonly property bool isSelected: root.currentMode === modelData.id

                    implicitHeight: 28
                    implicitWidth: modeRow.implicitWidth + Tokens.padding.medium
                    radius: Tokens.rounding.full
                    color: isSelected ? Colours.palette.m3primaryContainer : "transparent"

                    RowLayout {
                        id: modeRow
                        anchors.centerIn: parent
                        spacing: Tokens.spacing.extraSmall

                        MaterialIcon {
                            text: modeBtn.modelData.icon
                            color: modeBtn.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                            fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                        }

                        StyledText {
                            text: modeBtn.modelData.label
                            color: modeBtn.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                            font: Tokens.font.label.small
                        }
                    }

                    StateLayer {
                        anchors.fill: parent
                        radius: Tokens.rounding.full
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.currentMode = modeBtn.modelData.id;
                            root.modeSelected(modeBtn.modelData.id);
                        }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
        }

        // Thinking Budget Chip
        StyledRect {
            id: thinkingChip

            implicitHeight: 28
            implicitWidth: thinkingRow.implicitWidth + Tokens.padding.medium
            radius: Tokens.rounding.full
            color: root.thinkingBudget !== "off" ? Colours.palette.m3tertiaryContainer : "transparent"

            RowLayout {
                id: thinkingRow
                anchors.centerIn: parent
                spacing: Tokens.spacing.extraSmall

                MaterialIcon {
                    text: "psychology"
                    color: root.thinkingBudget !== "off" ? Colours.palette.m3onTertiaryContainer : Colours.palette.m3outline
                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                }

                StyledText {
                    text: root.thinkingBudget === "off" ? "Think: Off" : "Think: " + root.thinkingBudget.toUpperCase()
                    color: root.thinkingBudget !== "off" ? Colours.palette.m3onTertiaryContainer : Colours.palette.m3outline
                    font: Tokens.font.label.small
                }
            }

            StateLayer {
                anchors.fill: parent
                radius: Tokens.rounding.full
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    const next = root.thinkingBudget === "off" ? "low"
                        : root.thinkingBudget === "low" ? "medium"
                        : root.thinkingBudget === "medium" ? "high" : "off";
                    root.thinkingBudget = next;
                    root.thinkingSelected(next);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `ModeBar.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ModeBar.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ModeBar.qml
git commit -m "feat(qml): add ModeBar execution mode and thinking budget selector"
```

---

### Task 2: `DiffViewer.qml` Inline Code Diff Component

**Files:**
- Create: `qml/popouts/components/DiffViewer.qml`

**Interfaces:**
- Produces: `DiffViewer.qml`
  - `property string diffText: ""`
  - `property string fileLabel: ""`
  - `property bool expanded: true`

- [ ] **Step 1: Implement `qml/popouts/components/DiffViewer.qml`**

Create a collapsible unified diff view:
- Header bar:
  - Expand/collapse toggle chevron (`expand_more` / `expand_less`).
  - File icon + `fileLabel`.
  - Added lines badge (+N in green tint) and deleted lines badge (-M in error tint).
- Body:
  - Renders line-by-line using a `Repeater` or `ColumnLayout`.
  - Lines starting with `+` colored in `Colours.palette.m3tertiaryContainer` with `Colours.palette.m3onTertiaryContainer` text.
  - Lines starting with `-` colored in `Colours.palette.m3errorContainer` with `Colours.palette.m3onErrorContainer` text.
  - Context lines (` ` or `@@`) rendered in `Colours.palette.m3onSurfaceVariant`.
  - Monospace font (`Tokens.font.body.small`).

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property string diffText: ""
    property string fileLabel: "Diff"
    property bool expanded: true

    readonly property var diffLines: root.diffText ? root.diffText.split("\n") : []
    readonly property int addedCount: {
        let count = 0;
        for (let i = 0; i < diffLines.length; i++) {
            if (diffLines[i].startsWith("+") && !diffLines[i].startsWith("+++")) count++;
        }
        return count;
    }
    readonly property int deletedCount: {
        let count = 0;
        for (let i = 0; i < diffLines.length; i++) {
            if (diffLines[i].startsWith("-") && !diffLines[i].startsWith("---")) count++;
        }
        return count;
    }

    implicitWidth: parent ? parent.width : 380
    implicitHeight: expanded ? headerRect.implicitHeight + linesColumn.implicitHeight + Tokens.padding.small : headerRect.implicitHeight
    radius: Tokens.rounding.medium
    color: Colours.palette.m3surfaceContainerLowest
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    clip: true

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Diff Header
        StyledRect {
            id: headerRect

            Layout.fillWidth: true
            implicitHeight: 32
            color: Colours.palette.m3surfaceContainerHigh
            radius: root.expanded ? 0 : Tokens.rounding.medium

            RowLayout {
                anchors {
                    fill: parent
                    leftMargin: Tokens.padding.small
                    rightMargin: Tokens.padding.small
                }
                spacing: Tokens.spacing.extraSmall

                MaterialIcon {
                    text: root.expanded ? "expand_more" : "chevron_right"
                    color: Colours.palette.m3onSurfaceVariant
                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                }

                MaterialIcon {
                    text: "difference"
                    color: Colours.palette.m3primary
                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                }

                StyledText {
                    text: root.fileLabel
                    color: Colours.palette.m3onSurface
                    font: Tokens.font.label.small
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }

                // Additions badge
                StyledText {
                    visible: root.addedCount > 0
                    text: "+" + root.addedCount
                    color: Colours.palette.m3tertiary
                    font: Tokens.font.label.small
                }

                // Deletions badge
                StyledText {
                    visible: root.deletedCount > 0
                    text: "-" + root.deletedCount
                    color: Colours.palette.m3error
                    font: Tokens.font.label.small
                }
            }

            StateLayer {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.expanded = !root.expanded
            }
        }

        // Diff Lines Content
        ColumnLayout {
            id: linesColumn

            visible: root.expanded
            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: root.diffLines

                delegate: StyledRect {
                    id: lineRect

                    required property string modelData
                    required property int index

                    readonly property bool isAddition: modelData.startsWith("+") && !modelData.startsWith("+++")
                    readonly property bool isDeletion: modelData.startsWith("-") && !modelData.startsWith("---")
                    readonly property bool isHunkHeader: modelData.startsWith("@@")

                    Layout.fillWidth: true
                    implicitHeight: 20
                    color: isAddition ? Colours.palette.m3tertiaryContainer
                         : isDeletion ? Colours.palette.m3errorContainer
                         : "transparent"

                    StyledText {
                        anchors {
                            fill: parent
                            leftMargin: Tokens.padding.small
                            rightMargin: Tokens.padding.small
                        }
                        text: lineRect.modelData
                        font: Tokens.font.body.small
                        font.family: "monospace"
                        elide: Text.ElideRight
                        color: lineRect.isAddition ? Colours.palette.m3onTertiaryContainer
                             : lineRect.isDeletion ? Colours.palette.m3onErrorContainer
                             : lineRect.isHunkHeader ? Colours.palette.m3primary
                             : Colours.palette.m3onSurfaceVariant
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `DiffViewer.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/DiffViewer.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/DiffViewer.qml
git commit -m "feat(qml): add DiffViewer component with syntax coloring and collapsible hunks"
```

---

### Task 3: `ToolCard.qml` Interactive Tool Execution Card

**Files:**
- Create: `qml/popouts/components/ToolCard.qml`

**Interfaces:**
- Produces: `ToolCard.qml`
  - `property string toolCallId: ""`
  - `property string toolName: ""`
  - `property string toolArgs: ""`
  - `property bool requiresApproval: true`
  - `property string status: "pending"` (`"pending"`, `"approved"`, `"rejected"`, `"completed"`)
  - `property string diffText: ""`
  - `signal approvalDecided(string toolCallId, bool approved)`

- [ ] **Step 1: Implement `qml/popouts/components/ToolCard.qml`**

Create the interactive ToolCard component:
- Card surface styled with `Colours.palette.m3surfaceContainerHigh`, `Tokens.rounding.large`.
- Tool Header:
  - Dynamic tool icon (`terminal` for bash/command, `edit_note` for write/edit, `extension` for MCP).
  - Tool name in `Tokens.font.title.small`.
  - Status pill (`Pending Approval`, `Approved`, `Rejected`, `Executed`).
- Arguments summary:
  - Formatted command or file target in code block format.
- Diff viewer:
  - If `diffText` is non-empty, renders embedded `DiffViewer`.
- Action Buttons (rendered when `status === "pending"` and `requiresApproval`):
  - `[Reject]` button with `cancel` icon, `Colours.palette.m3errorContainer`.
  - `[Approve]` button with `check` icon, `Colours.palette.m3primaryContainer`.
  - Clicking either sets `status` immediately to prevent double submissions and emits `approvalDecided(toolCallId, approved)`.

```qml
import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property string toolCallId: ""
    property string toolName: ""
    property string toolArgs: ""
    property bool requiresApproval: true
    property string status: "pending"
    property string diffText: ""

    signal approvalDecided(string toolCallId, bool approved)

    readonly property bool isPending: root.status === "pending" && root.requiresApproval

    implicitWidth: parent ? parent.width : 420
    implicitHeight: mainCol.implicitHeight + Tokens.padding.medium * 2
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHigh
    border.color: isPending ? Colours.palette.m3primary : Colours.palette.m3outlineVariant
    border.width: isPending ? 2 : 1

    ColumnLayout {
        id: mainCol

        anchors {
            fill: parent
            margins: Tokens.padding.medium
        }
        spacing: Tokens.spacing.small

        // Tool Title & Status Row
        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spacing.small

            MaterialIcon {
                text: {
                    const n = root.toolName.toLowerCase();
                    if (n.includes("command") || n.includes("bash") || n.includes("terminal")) return "terminal";
                    if (n.includes("file") || n.includes("write") || n.includes("edit")) return "edit_note";
                    return "extension";
                }
                color: root.isPending ? Colours.palette.m3primary : Colours.palette.m3onSurfaceVariant
                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
            }

            StyledText {
                text: root.toolName
                font: Tokens.font.title.small
                color: Colours.palette.m3onSurface
            }

            Item { Layout.fillWidth: true }

            // Status Badge
            StyledRect {
                implicitHeight: 24
                implicitWidth: statusRow.implicitWidth + Tokens.padding.small * 2
                radius: Tokens.rounding.full
                color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3primaryContainer
                     : root.status === "rejected" ? Colours.palette.m3errorContainer
                     : Colours.palette.m3secondaryContainer

                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: root.status === "approved" || root.status === "completed" ? "check_circle"
                            : root.status === "rejected" ? "cancel"
                            : "hourglass_top"
                        color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3onPrimaryContainer
                             : root.status === "rejected" ? Colours.palette.m3onErrorContainer
                             : Colours.palette.m3onSecondaryContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                    }

                    StyledText {
                        text: root.status === "approved" ? "Approved"
                            : root.status === "rejected" ? "Rejected"
                            : root.status === "completed" ? "Executed"
                            : "Pending Approval"
                        color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3onPrimaryContainer
                             : root.status === "rejected" ? Colours.palette.m3onErrorContainer
                             : Colours.palette.m3onSecondaryContainer
                        font: Tokens.font.label.small
                    }
                }
            }
        }

        // Arguments snippet
        StyledRect {
            visible: Boolean(root.toolArgs)
            Layout.fillWidth: true
            implicitHeight: Math.min(argsTextItem.implicitHeight + Tokens.padding.small * 2, 80)
            radius: Tokens.rounding.small
            color: Colours.palette.m3surfaceContainerLowest
            clip: true

            StyledText {
                id: argsTextItem
                anchors {
                    fill: parent
                    margins: Tokens.padding.small
                }
                text: root.toolArgs
                font: Tokens.font.body.small
                font.family: "monospace"
                color: Colours.palette.m3onSurfaceVariant
                wrapMode: Text.WrapAnywhere
            }
        }

        // Embedded Diff Viewer
        DiffViewer {
            visible: Boolean(root.diffText)
            Layout.fillWidth: true
            diffText: root.diffText
            fileLabel: root.toolName
        }

        // Action Buttons Row (only if pending approval)
        RowLayout {
            visible: root.isPending
            Layout.fillWidth: true
            spacing: Tokens.spacing.small

            Item { Layout.fillWidth: true }

            // Reject Button
            StyledRect {
                id: rejectBtn
                implicitHeight: 32
                implicitWidth: rejectRow.implicitWidth + Tokens.padding.medium
                radius: Tokens.rounding.full
                color: Colours.palette.m3errorContainer

                RowLayout {
                    id: rejectRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: "close"
                        color: Colours.palette.m3onErrorContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }

                    StyledText {
                        text: "Reject"
                        color: Colours.palette.m3onErrorContainer
                        font: Tokens.font.label.small
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.full
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.status = "rejected";
                        root.approvalDecided(root.toolCallId, false);
                    }
                }
            }

            // Approve Button
            StyledRect {
                id: approveBtn
                implicitHeight: 32
                implicitWidth: approveRow.implicitWidth + Tokens.padding.medium
                radius: Tokens.rounding.full
                color: Colours.palette.m3primary

                RowLayout {
                    id: approveRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: "check"
                        color: Colours.palette.m3onPrimary
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }

                    StyledText {
                        text: "Approve"
                        color: Colours.palette.m3onPrimary
                        font: Tokens.font.label.small
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.full
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.status = "approved";
                        root.approvalDecided(root.toolCallId, true);
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run qmllint on `ToolCard.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ToolCard.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ToolCard.qml
git commit -m "feat(qml): add interactive ToolCard with parameter preview and approval actions"
```

---

### Task 4: `ChatView.qml` Integration with ToolCard Delegate

**Files:**
- Modify: `qml/popouts/components/ChatView.qml`

**Interfaces:**
- Produces:
  - `signal toolApprovalRequested(string toolCallId, bool approved)`
- Consumes: `ToolCard.qml`

- [ ] **Step 1: Update `messageDelegate` in `ChatView.qml`**

- In `delegateRoot`:
  Add optional properties with safe defaults:
  ```qml
  required property string sender
  required property string text
  property string toolCallId: ""
  property string toolName: ""
  property string toolArgs: ""
  property bool requiresApproval: true
  property string status: "pending"
  property string diff: ""
  ```
- Conditional layout:
  If `sender === "tool"`:
  Render `ToolCard`:
  ```qml
  ToolCard {
      width: messageListView.width
      toolCallId: delegateRoot.toolCallId
      toolName: delegateRoot.toolName
      toolArgs: delegateRoot.toolArgs
      requiresApproval: delegateRoot.requiresApproval
      status: delegateRoot.status
      diffText: delegateRoot.diff

      onApprovalDecided: function(id, approved) {
          root.toolApprovalRequested(id, approved);
      }
  }
  ```
  If `sender !== "tool"`:
  Render standard chat bubble `StyledRect`.

- [ ] **Step 2: Run qmllint on `ChatView.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/components/ChatView.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 3: Commit**

```bash
git add qml/popouts/components/ChatView.qml
git commit -m "feat(qml): integrate ToolCard delegates and approval signal in ChatView"
```

---

### Task 5: `AgentHudPopout.qml` Full Safety Integration & WebSocket RPCs

**Files:**
- Modify: `qml/popouts/AgentHudPopout.qml`

**Interfaces:**
- Consumes:
  - `ServerMessage::ToolCallRequest` (`tool_call_id`, `tool_name`, `args`, `requires_approval`, `diff`)
  - `ServerMessage::ToolCallResult` (`tool_call_id`, `success`, `output`)
- Produces:
  - `ClientMessage::UpdateConfig` on mode or thinking changes
  - `ClientMessage::ToolApproval` on tool approval decisions

- [ ] **Step 1: Embed `ModeBar.qml` in `AgentHudPopout.qml`**

- Place `ModeBar` between the context header and the `ChatView`:
  ```qml
  ModeBar {
      id: modeBar
      Layout.fillWidth: true
      Layout.leftMargin: Tokens.padding.medium
      Layout.rightMargin: Tokens.padding.medium
      Layout.topMargin: Tokens.spacing.extraSmall
      Layout.bottomMargin: Tokens.spacing.extraSmall

      onModeSelected: function(mode) {
          root.dispatchConfigUpdate(mode, modeBar.thinkingBudget);
      }
      onThinkingSelected: function(budget) {
          root.dispatchConfigUpdate(modeBar.currentMode, budget);
      }
  }
  ```

- [ ] **Step 2: Wire WebSocket messages for Tool Calls and Mode Updates**

- In `ws.onTextMessageReceived`:
  - `data.type === "tool_call_request"`:
    ```javascript
    chatModel.append({
        sender: "tool",
        text: "",
        toolCallId: data.payload.tool_call_id || "",
        toolName: data.payload.tool_name || "",
        toolArgs: JSON.stringify(data.payload.args || {}),
        requiresApproval: data.payload.requires_approval === true,
        status: data.payload.requires_approval ? "pending" : "approved",
        diff: data.payload.diff || ""
    });
    ```
  - `data.type === "tool_call_result"`:
    ```javascript
    for (let i = 0; i < chatModel.count; i++) {
        const item = chatModel.get(i);
        if (item.toolCallId === data.payload.tool_call_id) {
            chatModel.setProperty(i, "status", data.payload.success ? "completed" : "rejected");
            break;
        }
    }
    ```
- In `dispatchConfigUpdate(mode, thinking)`:
  ```javascript
  if (ws.status === WebSocket.Open) {
      ws.sendTextMessage(JSON.stringify({
          type: "update_config",
          payload: {
              platform: "antigravity",
              mode: mode,
              thinking: thinking
          }
      }));
  }
  ```
- In `chatView.onToolApprovalRequested(id, approved)`:
  ```javascript
  if (ws.status === WebSocket.Open) {
      ws.sendTextMessage(JSON.stringify({
          type: "tool_approval",
          payload: {
              tool_call_id: id,
              approved: approved
          }
      }));
  }
  ```

- [ ] **Step 3: Run qmllint on `AgentHudPopout.qml`**

Run: `qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/popouts/AgentHudPopout.qml`
Expected: PASS with 0 warnings/errors.

- [ ] **Step 4: Commit**

```bash
git add qml/popouts/AgentHudPopout.qml
git commit -m "feat(qml): integrate ModeBar and wire tool call requests and approvals in HUD popout"
```

---

### Task 6: Standalone Preview & End-to-End Quality Gates

**Files:**
- Modify: `qml/shell.qml` (add mock tool items for preview)
- Verify: Entire `qml/` and `daemon/` workspaces

- [ ] **Step 1: Update `qml/shell.qml` for preview testing**

Verify mock tool card rendering and mode selection in `shell.qml`.

- [ ] **Step 2: Run complete quality checks**

```bash
qmllint -I ~/.config/quickshell/caelestia -I qml -I qml/popouts qml/AgentHudButton.qml qml/popouts/AgentHudPopout.qml qml/popouts/components/ChatView.qml qml/popouts/components/FileDropArea.qml qml/popouts/components/FileMentionPopup.qml qml/popouts/components/ModeBar.qml qml/popouts/components/DiffViewer.qml qml/popouts/components/ToolCard.qml qml/shell.qml
python3 .agents/skills/qt-qml-review/references/lint-scripts/qt_qml_lint.py --dir qml/
cargo test --manifest-path daemon/Cargo.toml
cargo clippy --manifest-path daemon/Cargo.toml -- -D warnings
cargo fmt --check --manifest-path daemon/Cargo.toml
```
Expected: All linters and tests exit with code 0.

- [ ] **Step 3: Commit**

```bash
git add qml/shell.qml
git commit -m "chore(qml): verify standalone HUD preview with ModeBar and ToolCard components"
```
