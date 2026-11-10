import QtQuick
import QtQuick.Layouts
import QtWebSockets
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import Caelestia.Config
import qs.components
import "components"

// qmllint disable uncreatable-type
PanelWindow {
    id: root

    property alias isConnected: root.wsConnected
    readonly property bool wsConnected: ws.status === WebSocket.Open
    property bool isThinking: false
    property string currentWorkspacePath: ""
    property string currentGitBranch: ""
    property int skillsCount: 0

    implicitWidth: 480
    implicitHeight: 640
    color: "transparent"

    WlrLayershell.namespace: "caelestia-aetheris-hud"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: root.visible ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None

    anchors {
        top: true
    }

    margins {
        top: 64
    }

    Shortcut {
        sequence: "Escape"
        onActivated: root.visible = false
    }

    HyprlandFocusGrab {
        active: root.visible
        windows: [root]
        onCleared: root.visible = false
    }

    onVisibleChanged: {
        if (root.visible) {
            chatView.focusInput();
        }
    }

    ListModel {
        id: chatModel

        ListElement {
            sender: "assistant"
            text: "Hello! I am Aetheris, your OS-level AI agent. How can I assist you in this workspace?"
        }
    }

    WebSocket {
        id: ws

        url: "ws://127.0.0.1:21420"
        active: true

        onStatusChanged: {
            if (ws.status !== WebSocket.Open) {
                root.isThinking = false;
            }
        }

        onTextMessageReceived: function(message) {
            try {
                const data = JSON.parse(message);
                if (data.type === "token_stream") {
                    const payload = data.payload;
                    if (payload.done === true) {
                        root.isThinking = false;
                    }
                    if (payload.token) {
                        if (chatModel.count > 0 && chatModel.get(chatModel.count - 1).sender === "assistant") {
                            const last = chatModel.get(chatModel.count - 1);
                            chatModel.setProperty(chatModel.count - 1, "text", last.text + payload.token);
                        } else {
                            chatModel.append({ sender: "assistant", text: payload.token });
                        }
                    }
                } else if (data.type === "workspace_sync") {
                    root.currentWorkspacePath = data.payload.path || "";
                    root.currentGitBranch = data.payload.git_branch || "";
                    root.skillsCount = data.payload.skills_count || 0;
                } else if (data.type === "file_results") {
                    chatView.fileMatches = data.payload.matches || [];
                }
            } catch (e) {
                console.warn("Failed to parse Aetheris message:", e);
            }
        }
    }

    Timer {
        id: reconnectTimer

        interval: 2000
        repeat: true
        running: ws.status !== WebSocket.Open

        onTriggered: {
            ws.active = false;
            ws.active = true;
        }
    }

    function dispatchPrompt(text, attachments) {
        if ((!text && (!attachments || attachments.length === 0)) || ws.status !== WebSocket.Open)
            return;

        root.isThinking = true;
        chatModel.append({ sender: "user", text: text });
        chatModel.append({ sender: "assistant", text: "" });

        const req = {
            type: "send_message",
            payload: {
                text: text,
                attachments: attachments || [],
                quote: null
            }
        };
        ws.sendTextMessage(JSON.stringify(req));
    }

    StyledRect {
        id: surfaceRect

        radius: Tokens.rounding.extraLarge
        color: Colours.tPalette.m3surfaceContainerHigh

        anchors.fill: parent

        ColumnLayout {
            spacing: 0
            anchors.fill: parent

            // Header bar
            StyledRect {
                implicitHeight: 68
                radius: Tokens.rounding.extraLarge
                color: Colours.palette.m3surfaceContainer

                Layout.fillWidth: true

                ColumnLayout {
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.large
                        rightMargin: Tokens.padding.medium
                        topMargin: Tokens.padding.small
                        bottomMargin: Tokens.padding.small
                    }
                    spacing: Tokens.spacing.extraSmall

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.spacing.small

                        MaterialIcon {
                            text: "smart_toy"
                            color: root.wsConnected ? Colours.palette.m3primary : Colours.palette.m3outline
                            fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                        }

                        StyledText {
                            text: "Aetheris HUD"
                            font: Tokens.font.title.small
                            color: Colours.palette.m3onSurface
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        StyledRect {
                            implicitWidth: 28
                            implicitHeight: 28
                            radius: Tokens.rounding.full
                            color: "transparent"

                            StateLayer {
                                anchors.fill: parent
                                radius: Tokens.rounding.full
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.visible = false
                            }

                            MaterialIcon {
                                anchors.centerIn: parent
                                text: "close"
                                color: Colours.palette.m3onSurfaceVariant
                                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                            }
                        }
                    }

                    // Context badges row
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.spacing.small

                        // Active Project Pill
                        StyledRect {
                            visible: Boolean(root.currentWorkspacePath)
                            implicitHeight: 22
                            implicitWidth: projRow.implicitWidth + Tokens.padding.medium
                            radius: Tokens.rounding.full
                            color: Colours.palette.m3surfaceContainerHigh

                            RowLayout {
                                id: projRow
                                anchors.centerIn: parent
                                spacing: Tokens.spacing.extraSmall

                                MaterialIcon {
                                    text: "folder"
                                    color: Colours.palette.m3onSurfaceVariant
                                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                                }

                                StyledText {
                                    text: {
                                        if (!root.currentWorkspacePath)
                                            return "";
                                        const clean = root.currentWorkspacePath.replace(/\/+$/, "");
                                        return clean.substring(clean.lastIndexOf("/") + 1) || root.currentWorkspacePath;
                                    }
                                    color: Colours.palette.m3onSurfaceVariant
                                    font: Tokens.font.label.small
                                    elide: Text.ElideMiddle
                                    Layout.maximumWidth: 120
                                }
                            }
                        }

                        // Git Branch Badge
                        StyledRect {
                            visible: Boolean(root.currentGitBranch)
                            implicitHeight: 22
                            implicitWidth: branchRow.implicitWidth + Tokens.padding.medium
                            radius: Tokens.rounding.full
                            color: Colours.palette.m3secondaryContainer

                            RowLayout {
                                id: branchRow
                                anchors.centerIn: parent
                                spacing: Tokens.spacing.extraSmall

                                MaterialIcon {
                                    text: "fork_right"
                                    color: Colours.palette.m3onSecondaryContainer
                                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                                }

                                StyledText {
                                    text: root.currentGitBranch
                                    color: Colours.palette.m3onSecondaryContainer
                                    font: Tokens.font.label.small
                                    elide: Text.ElideMiddle
                                    Layout.maximumWidth: 110
                                }
                            }
                        }

                        // Skills Count Badge
                        StyledRect {
                            visible: root.skillsCount > 0
                            implicitHeight: 22
                            implicitWidth: skillsRow.implicitWidth + Tokens.padding.medium
                            radius: Tokens.rounding.full
                            color: Colours.palette.m3tertiaryContainer

                            RowLayout {
                                id: skillsRow
                                anchors.centerIn: parent
                                spacing: Tokens.spacing.extraSmall

                                MaterialIcon {
                                    text: "bolt"
                                    color: Colours.palette.m3onTertiaryContainer
                                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                                }

                                StyledText {
                                    text: root.skillsCount + (root.skillsCount === 1 ? " skill" : " skills")
                                    color: Colours.palette.m3onTertiaryContainer
                                    font: Tokens.font.label.small
                                }
                            }
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        // Connection status
                        StyledText {
                            visible: !root.wsConnected
                            text: "Connecting..."
                            font: Tokens.font.label.small
                            color: Colours.palette.m3outline
                        }
                    }
                }
            }

            // Chat View
            ChatView {
                id: chatView

                messageModel: chatModel
                isConnected: root.wsConnected

                Layout.fillWidth: true
                Layout.fillHeight: true

                onSendRequested: function(text, attachments) {
                    root.dispatchPrompt(text, attachments);
                }

                onFileSearchRequested: function(query) {
                    if (ws.status === WebSocket.Open) {
                        ws.sendTextMessage(JSON.stringify({
                            type: "search_files",
                            payload: {
                                query: query,
                                limit: 8
                            }
                        }));
                    }
                }
            }
        }
    }
}
