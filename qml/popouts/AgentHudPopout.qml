import QtQuick
import QtQuick.Layouts
import QtWebSockets
import Quickshell
import Quickshell.Wayland
import Caelestia.Config
import qs.components
import "components"

// qmllint disable uncreatable-type
PanelWindow {
    id: root

    property alias isConnected: root.wsConnected
    readonly property bool wsConnected: ws.status === WebSocket.Open
    property string currentWorkspacePath: ""

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
        active: root.visible

        onTextMessageReceived: function(message) {
            try {
                const data = JSON.parse(message);
                if (data.type === "token_stream") {
                    const payload = data.payload;
                    if (chatModel.count > 0 && chatModel.get(chatModel.count - 1).sender === "assistant") {
                        const last = chatModel.get(chatModel.count - 1);
                        chatModel.setProperty(chatModel.count - 1, "text", last.text + payload.token);
                    } else {
                        chatModel.append({ sender: "assistant", text: payload.token });
                    }
                } else if (data.type === "workspace_sync") {
                    root.currentWorkspacePath = data.payload.path;
                }
            } catch (e) {
                console.warn("Failed to parse Aetheris message:", e);
            }
        }
    }

    function dispatchPrompt(text) {
        if (!text || ws.status !== WebSocket.Open)
            return;

        chatModel.append({ sender: "user", text: text });
        chatModel.append({ sender: "assistant", text: "" });

        const req = {
            type: "send_message",
            payload: {
                text: text,
                attachments: [],
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
                implicitHeight: 56
                radius: Tokens.rounding.extraLarge
                color: Colours.palette.m3surfaceContainer

                Layout.fillWidth: true

                RowLayout {
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.large
                        rightMargin: Tokens.padding.medium
                    }

                    MaterialIcon {
                        text: "smart_toy"
                        color: root.wsConnected ? Colours.palette.m3primary : Colours.palette.m3outline
                    }

                    ColumnLayout {
                        spacing: 0
                        Layout.fillWidth: true

                        StyledText {
                            text: "Aetheris HUD"
                            font: Tokens.font.title.small
                            color: Colours.palette.m3onSurface
                        }

                        StyledText {
                            text: root.wsConnected ? "ws://127.0.0.1:21420 (Connected)" : "Connecting to daemon..."
                            font: Tokens.font.label.small
                            color: root.wsConnected ? Colours.palette.m3primary : Colours.palette.m3outline
                        }
                    }

                    StyledRect {
                        implicitWidth: 32
                        implicitHeight: 32
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
                        }
                    }
                }
            }

            // Chat View
            ChatView {
                id: chatView

                messageModel: chatModel

                Layout.fillWidth: true
                Layout.fillHeight: true

                onSendRequested: function(text) {
                    root.dispatchPrompt(text);
                }
            }
        }
    }
}
