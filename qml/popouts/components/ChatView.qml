import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components
import qs.services

Item {
    id: root

    property alias messageModel: messageListView.model
    property alias fileMatches: mentionPopup.matches
    property bool isConnected: true
    property var attachments: []
    property bool mentionQueryActive: false
    property int mentionStartIndex: -1
    property string currentPlatform: "antigravity"
    property string currentMode: "safe"
    readonly property bool hasActivePopup: contextPopup.isOpen || modelPopup.isOpen || modePopup.isOpen || mentionPopup.visible

    signal sendRequested(string text, var attachments)
    signal fileSearchRequested(string query)
    signal toolApprovalRequested(string toolCallId, bool approved)
    signal modeSelected(string mode)
    signal platformSelected(string platform)
    signal filesRequested()
    signal folderRequested()

    onVisibleChanged: {
        if (!root.visible) {
            root.closeAllPopups();
        }
    }

    function closeAllPopups() {
        contextPopup.close();
        modelPopup.close();
        modePopup.close();
        root.mentionQueryActive = false;
    }

    function submitPrompt() {
        if (!root.isConnected)
            return;

        const trimmed = inputField.text.trim();
        if (trimmed.length > 0 || root.attachments.length > 0) {
            root.sendRequested(trimmed, root.attachments.slice());
            inputField.text = "";
            root.attachments = [];
            root.mentionQueryActive = false;
            root.mentionStartIndex = -1;
        }
    }

    function addAttachment(path) {
        if (!path)
            return;
        const list = root.attachments.slice();
        if (!list.includes(path)) {
            list.push(path);
            root.attachments = list;
        }
    }

    function removeAttachment(index) {
        const list = root.attachments.slice();
        list.splice(index, 1);
        root.attachments = list;
    }

    function checkMention() {
        const text = inputField.text;
        const atIndex = text.lastIndexOf("@");
        if (atIndex !== -1) {
            const isAtStartOrWhitespace = atIndex === 0 || text.charAt(atIndex - 1) === " " || text.charAt(atIndex - 1) === "\n" || text.charAt(atIndex - 1) === "\t";
            if (isAtStartOrWhitespace) {
                const candidate = text.substring(atIndex + 1);
                if (candidate.indexOf(" ") === -1 && candidate.indexOf("\n") === -1 && candidate.indexOf("\t") === -1) {
                    root.mentionStartIndex = atIndex;
                    root.mentionQueryActive = true;
                    root.fileSearchRequested(candidate);
                    return;
                }
            }
        }
        root.mentionQueryActive = false;
        root.mentionStartIndex = -1;
    }

    function insertMention(path) {
        if (root.mentionStartIndex >= 0) {
            const text = inputField.text;
            const prefix = text.substring(0, root.mentionStartIndex);
            const newText = prefix + "@" + path + " ";
            inputField.text = newText;
            inputField.cursorPosition = newText.length;
        }
        root.mentionQueryActive = false;
        root.mentionStartIndex = -1;
    }

    function focusInput() {
        inputField.forceActiveFocus();
    }

    FileDropArea {
        anchors.fill: parent
        onFilesDropped: function(paths) {
            for (let i = 0; i < paths.length; i++) {
                root.addAttachment(paths[i]);
            }
        }
    }

    MouseArea {
        id: popupDismisser

        anchors.fill: parent
        z: 90
        visible: contextPopup.isOpen || modelPopup.isOpen || modePopup.isOpen
        onClicked: {
            root.closeAllPopups();
        }
    }

    ColumnLayout {
        id: mainLayout

        spacing: Tokens.spacing.medium

        anchors {
            fill: parent
            margins: Tokens.padding.medium
        }

        // Messages list
        ListView {
            id: messageListView

            clip: true
            spacing: Tokens.spacing.small
            boundsBehavior: Flickable.StopAtBounds

            Layout.fillWidth: true
            Layout.fillHeight: true

            onCountChanged: {
                Qt.callLater(() => {
                    messageListView.positionViewAtEnd();
                });
            }

            onContentHeightChanged: {
                if (atYEnd)
                    positionViewAtEnd();
            }

            delegate: Component {
                id: messageDelegate

                Item {
                    id: delegateRoot

                    required property string sender
                    required property string text
                    property string toolCallId: typeof model !== "undefined" && typeof model.toolCallId !== "undefined" ? model.toolCallId : ""
                    property string toolName: typeof model !== "undefined" && typeof model.toolName !== "undefined" ? model.toolName : ""
                    property string toolArgs: typeof model !== "undefined" && typeof model.toolArgs !== "undefined" ? model.toolArgs : ""
                    property bool requiresApproval: typeof model !== "undefined" && typeof model.requiresApproval !== "undefined" ? model.requiresApproval : true
                    property string status: typeof model !== "undefined" && typeof model.status !== "undefined" ? model.status : "pending"
                    property string diff: typeof model !== "undefined" && typeof model.diff !== "undefined" ? model.diff : ""

                    readonly property bool isTool: sender === "tool"
                    readonly property bool isUser: sender === "user"

                    width: messageListView.width
                    implicitHeight: isTool ? toolCard.implicitHeight + Tokens.spacing.extraSmall
                                           : (bubbleRect.visible ? bubbleRect.implicitHeight + Tokens.spacing.extraSmall : 0)

                    StyledRect {
                        id: bubbleRect

                        visible: !delegateRoot.isTool && (delegateRoot.text.length > 0 || delegateRoot.isUser)
                        width: Math.min(messageText.implicitWidth + Tokens.padding.medium * 2, delegateRoot.width * 0.85)
                        implicitHeight: messageText.implicitHeight + Tokens.padding.medium * 2
                        radius: Tokens.rounding.large
                        color: delegateRoot.isUser ? Colours.palette.m3primaryContainer : Colours.palette.m3surfaceContainerHigh

                        anchors {
                            left: delegateRoot.isUser ? undefined : parent.left
                            right: delegateRoot.isUser ? parent.right : undefined
                        }

                        StyledText {
                            id: messageText

                            text: delegateRoot.text
                            textFormat: delegateRoot.isUser ? Text.PlainText : Text.MarkdownText
                            color: delegateRoot.isUser ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurface
                            wrapMode: Text.Wrap
                            font: Tokens.font.body.medium

                            anchors {
                                fill: parent
                                margins: Tokens.padding.medium
                            }
                        }
                    }

                    ToolCard {
                        id: toolCard

                        visible: delegateRoot.isTool
                        width: delegateRoot.width
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
                }
            }
        }

        // Unified Input Card
        StyledRect {
            id: inputCard

            Layout.fillWidth: true
            implicitHeight: inputCardCol.implicitHeight + Tokens.padding.medium * 2
            radius: Tokens.rounding.extraLarge
            color: Colours.palette.m3surfaceContainer
            border.color: Colours.palette.m3outlineVariant
            border.width: 1

            FileMentionPopup {
                id: mentionPopup

                z: 100
                width: Math.min(360, root.width)
                visible: Boolean(mentionPopup.matches && mentionPopup.matches.length > 0 && root.mentionQueryActive)
                anchors {
                    bottom: parent.top
                    bottomMargin: Tokens.spacing.small
                    left: parent.left
                }

                onFileSelected: function(path) {
                    root.insertMention(path);
                }
                onDismissed: {
                    root.mentionQueryActive = false;
                }
            }

            ContextMenuPopup {
                id: contextPopup

                z: 100
                anchors {
                    bottom: parent.top
                    bottomMargin: Tokens.spacing.small
                    left: parent.left
                    leftMargin: Tokens.padding.medium
                }

                onFilesRequested: function() {
                    root.filesRequested();
                }
                onFolderRequested: function() {
                    root.folderRequested();
                }
                onMentionsRequested: function() {
                    inputField.text = inputField.text + "@";
                    inputField.cursorPosition = inputField.text.length;
                    root.checkMention();
                    root.focusInput();
                }
            }

            ModelSelectorPopup {
                id: modelPopup

                z: 100
                currentPlatform: root.currentPlatform
                anchors {
                    bottom: parent.top
                    bottomMargin: Tokens.spacing.small
                    left: parent.left
                    leftMargin: Tokens.padding.medium + 36
                }

                onPlatformSelected: function(platform) {
                    root.currentPlatform = platform;
                    root.platformSelected(platform);
                    root.focusInput();
                }
            }

            ModeSelectorPopup {
                id: modePopup

                z: 100
                currentMode: root.currentMode
                anchors {
                    bottom: parent.top
                    bottomMargin: Tokens.spacing.small
                    right: parent.right
                    rightMargin: Tokens.padding.medium + 40
                }

                onModeSelected: function(mode) {
                    root.currentMode = mode;
                    root.modeSelected(mode);
                    root.focusInput();
                }
            }

            ColumnLayout {
                id: inputCardCol

                spacing: Tokens.spacing.small
                anchors {
                    left: parent.left
                    right: parent.right
                    top: parent.top
                    margins: Tokens.padding.medium
                }

                // Attachment chips (wrapping flow)
                Flow {
                    id: attachmentBar

                    visible: root.attachments.length > 0
                    spacing: Tokens.spacing.small
                    Layout.fillWidth: true

                    Repeater {
                        id: chipRepeater

                        model: root.attachments

                        delegate: StyledRect {
                            id: chipRect

                            required property string modelData
                            required property int index

                            implicitHeight: 28
                            implicitWidth: chipRow.implicitWidth + Tokens.padding.medium
                            radius: Tokens.rounding.full
                            color: Colours.palette.m3secondaryContainer

                            RowLayout {
                                id: chipRow

                                spacing: Tokens.spacing.extraSmall
                                anchors.centerIn: parent

                                MaterialIcon {
                                    text: "attachment"
                                    color: Colours.palette.m3onSecondaryContainer
                                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                                }

                                StyledText {
                                    text: {
                                        const clean = chipRect.modelData.replace(/\/+$/, "");
                                        return clean.substring(clean.lastIndexOf("/") + 1) || chipRect.modelData;
                                    }
                                    color: Colours.palette.m3onSecondaryContainer
                                    font: Tokens.font.label.small
                                    elide: Text.ElideMiddle
                                    Layout.maximumWidth: 140
                                }

                                MaterialIcon {
                                    text: "close"
                                    color: Colours.palette.m3onSecondaryContainer
                                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()

                                    StateLayer {
                                        anchors.fill: parent
                                        radius: Tokens.rounding.full
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.removeAttachment(chipRect.index)
                                    }
                                }
                            }
                        }
                    }
                }

                // Text Input area (multiline auto-expanding)
                Item {
                    implicitHeight: Math.min(120, Math.max(28, inputField.implicitHeight))
                    Layout.fillWidth: true

                    Flickable {
                        id: inputFlickable

                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        contentWidth: width
                        contentHeight: inputField.implicitHeight
                        anchors {
                            fill: parent
                            leftMargin: Tokens.padding.small
                            rightMargin: Tokens.padding.small
                        }

                        TextEdit {
                            id: inputField

                            width: inputFlickable.width
                            wrapMode: TextEdit.Wrap
                            color: Colours.palette.m3onSurface
                            font: Tokens.font.body.medium
                            selectByMouse: true

                            onTextChanged: {
                                root.checkMention();
                            }

                            onCursorRectangleChanged: {
                                inputFlickable.contentY = Math.max(0, inputField.cursorRectangle.y + inputField.cursorRectangle.height - inputFlickable.height);
                            }

                            Keys.onPressed: function(event) {
                                if (event.key === Qt.Key_Escape) {
                                    if (contextPopup.isOpen || modelPopup.isOpen || modePopup.isOpen) {
                                        root.closeAllPopups();
                                        event.accepted = true;
                                        return;
                                    }
                                    if (mentionPopup.visible) {
                                        root.mentionQueryActive = false;
                                        event.accepted = true;
                                        return;
                                    }
                                }

                                if (mentionPopup.visible) {
                                    if (event.key === Qt.Key_Down) {
                                        mentionPopup.selectNext();
                                        event.accepted = true;
                                        return;
                                    }
                                    if (event.key === Qt.Key_Up) {
                                        mentionPopup.selectPrevious();
                                        event.accepted = true;
                                        return;
                                    }
                                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                                        if (mentionPopup.matches && mentionPopup.matches.length > 0) {
                                            mentionPopup.triggerSelection();
                                            event.accepted = true;
                                            return;
                                        }
                                    }
                                }

                                if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                                    root.submitPrompt();
                                    event.accepted = true;
                                    return;
                                }
                            }

                            StyledText {
                                anchors.fill: parent
                                text: root.isConnected ? "Ask anything, @ to mention, / for actions" : "Connecting to daemon..."
                                color: Colours.palette.m3onSurfaceVariant
                                font: parent.font
                                visible: !inputField.text
                            }
                        }
                    }
                }

                // Bottom Controls Row
                RowLayout {
                    spacing: Tokens.spacing.small
                    Layout.fillWidth: true

                    // "+" Add Context button
                    StyledRect {
                        id: addContextBtn

                        implicitWidth: 28
                        implicitHeight: 28
                        radius: Tokens.rounding.full
                        color: contextPopup.isOpen ? Colours.palette.m3surfaceContainerHighest : "transparent"
                        Accessible.role: Accessible.Button
                        Accessible.name: "Add Context"

                        MaterialIcon {
                            anchors.centerIn: parent
                            text: "add"
                            color: Colours.palette.m3onSurfaceVariant
                            fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                        }

                        StateLayer {
                            anchors.fill: parent
                            radius: Tokens.rounding.full
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                modelPopup.close();
                                modePopup.close();
                                contextPopup.toggle();
                            }
                        }
                    }

                    // Model Selector Button
                    StyledRect {
                        id: modelBtn

                        implicitHeight: 28
                        implicitWidth: modelRow.implicitWidth + Tokens.padding.small * 2
                        radius: Tokens.rounding.full
                        color: modelPopup.isOpen ? Colours.palette.m3surfaceContainerHighest : "transparent"
                        Accessible.role: Accessible.Button
                        Accessible.name: "Select Model"

                        RowLayout {
                            id: modelRow

                            spacing: Tokens.spacing.extraSmall
                            anchors.centerIn: parent

                            MaterialIcon {
                                text: root.currentPlatform === "claude" ? "terminal"
                                    : root.currentPlatform === "direct" ? "api"
                                    : "smart_toy"
                                color: Colours.palette.m3onSurfaceVariant
                                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                            }

                            StyledText {
                                text: root.currentPlatform === "claude" ? "Claude Code"
                                    : root.currentPlatform === "direct" ? "Direct LLM"
                                    : "Antigravity"
                                font: Tokens.font.label.small
                                color: Colours.palette.m3onSurfaceVariant
                            }

                            MaterialIcon {
                                text: "expand_less"
                                color: Colours.palette.m3outline
                                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                            }
                        }

                        StateLayer {
                            anchors.fill: parent
                            radius: Tokens.rounding.full
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                contextPopup.close();
                                modePopup.close();
                                modelPopup.toggle();
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Mode Selector Button
                    StyledRect {
                        id: modeBtn

                        implicitHeight: 28
                        implicitWidth: modeRow.implicitWidth + Tokens.padding.small * 2
                        radius: Tokens.rounding.full
                        color: modePopup.isOpen ? Colours.palette.m3surfaceContainerHighest : "transparent"
                        Accessible.role: Accessible.Button
                        Accessible.name: "Select Execution Mode"

                        RowLayout {
                            id: modeRow

                            spacing: Tokens.spacing.extraSmall
                            anchors.centerIn: parent

                            MaterialIcon {
                                text: root.currentMode === "yolo" ? "bolt"
                                    : root.currentMode === "plan" ? "edit_note"
                                    : "shield"
                                color: root.currentMode === "yolo" ? Colours.palette.m3error
                                     : root.currentMode === "plan" ? Colours.palette.m3tertiary
                                     : Colours.palette.m3primary
                                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                            }

                            StyledText {
                                text: root.currentMode === "yolo" ? "YOLO"
                                    : root.currentMode === "plan" ? "Plan"
                                    : "Safe"
                                font: Tokens.font.label.small
                                color: Colours.palette.m3onSurface
                            }

                            MaterialIcon {
                                text: "expand_less"
                                color: Colours.palette.m3outline
                                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                            }
                        }

                        StateLayer {
                            anchors.fill: parent
                            radius: Tokens.rounding.full
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                contextPopup.close();
                                modelPopup.close();
                                modePopup.toggle();
                            }
                        }
                    }

                    // Circular Send Button
                    StyledRect {
                        id: sendButton

                        readonly property bool canSend: root.isConnected && (inputField.text.trim().length > 0 || root.attachments.length > 0)

                        implicitWidth: 32
                        implicitHeight: 32
                        radius: Tokens.rounding.full
                        color: canSend ? Colours.palette.m3primary : Colours.palette.m3surfaceContainerHighest
                        Accessible.role: Accessible.Button
                        Accessible.name: "Send Message"

                        MaterialIcon {
                            anchors.centerIn: parent
                            text: "arrow_forward"
                            color: sendButton.canSend ? Colours.palette.m3onPrimary : Colours.palette.m3onSurfaceVariant
                            fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                        }

                        StateLayer {
                            anchors.fill: parent
                            radius: Tokens.rounding.full
                            cursorShape: sendButton.canSend ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: root.submitPrompt()
                        }
                    }
                }
            }
        }
    }
}
