import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

Item {
    id: root

    property alias messageModel: messageListView.model
    property alias fileMatches: mentionPopup.matches
    property bool isConnected: true
    property var attachments: []
    property bool mentionQueryActive: false
    property int mentionStartIndex: -1

    signal sendRequested(string text, var attachments)
    signal fileSearchRequested(string query)
    signal toolApprovalRequested(string toolCallId, bool approved)

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

    FileMentionPopup {
        id: mentionPopup

        z: 10
        anchors {
            bottom: mainLayout.bottom
            bottomMargin: inputContainer.height + Tokens.spacing.small
            left: mainLayout.left
        }
        width: Math.min(360, mainLayout.width)
        visible: Boolean(mentionPopup.matches && mentionPopup.matches.length > 0 && root.mentionQueryActive)

        onFileSelected: function(path) {
            root.insertMention(path);
        }
        onDismissed: {
            root.mentionQueryActive = false;
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

        // Attachment chips
        RowLayout {
            id: attachmentBar

            visible: root.attachments.length > 0
            spacing: Tokens.spacing.small
            Layout.fillWidth: true

            Repeater {
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
                        anchors.centerIn: parent
                        spacing: Tokens.spacing.extraSmall

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
                            Layout.maximumWidth: 120
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

        // Input bar
        StyledRect {
            id: inputContainer

            implicitHeight: 48
            radius: Tokens.rounding.full
            color: Colours.palette.m3surfaceContainer

            Layout.fillWidth: true

            RowLayout {
                spacing: Tokens.spacing.small

                anchors {
                    fill: parent
                    leftMargin: Tokens.padding.large
                    rightMargin: Tokens.padding.small
                }

                TextInput {
                    id: inputField

                    color: Colours.palette.m3onSurface
                    font: Tokens.font.body.medium
                    clip: true

                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter

                    onTextChanged: {
                        root.checkMention();
                    }

                    Keys.onPressed: function(event) {
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
                            if (event.key === Qt.Key_Escape) {
                                root.mentionQueryActive = false;
                                event.accepted = true;
                                return;
                            }
                        }

                        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                            root.submitPrompt();
                            event.accepted = true;
                        }
                    }

                    StyledText {
                        anchors.fill: parent
                        text: root.isConnected ? "Ask Aetheris or dispatch task..." : "Connecting to daemon..."
                        color: Colours.palette.m3onSurfaceVariant
                        font: parent.font
                        visible: !inputField.text
                    }
                }

                StyledRect {
                    id: sendButton

                    readonly property bool canSend: root.isConnected && (inputField.text.trim().length > 0 || root.attachments.length > 0)

                    implicitWidth: 36
                    implicitHeight: 36
                    radius: Tokens.rounding.full
                    color: canSend ? Colours.palette.m3primary : Colours.palette.m3surfaceContainerHighest

                    StateLayer {
                        anchors.fill: parent
                        radius: Tokens.rounding.full
                        cursorShape: sendButton.canSend ? Qt.PointingHandCursor : Qt.ArrowCursor
                        onClicked: root.submitPrompt()
                    }

                    MaterialIcon {
                        anchors.centerIn: parent
                        text: "arrow_upward"
                        color: sendButton.canSend ? Colours.palette.m3onPrimary : Colours.palette.m3onSurfaceVariant
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }
                }
            }
        }
    }
}
