import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

Item {
    id: root

    property alias messageModel: messageListView.model
    signal sendRequested(string text)

    function submitPrompt() {
        const trimmed = inputField.text.trim();
        if (trimmed.length > 0) {
            root.sendRequested(trimmed);
            inputField.text = "";
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

            delegate: Component {
                id: messageDelegate

                Item {
                    id: delegateRoot

                    required property string sender
                    required property string text

                    readonly property bool isUser: sender === "user"

                    width: messageListView.width
                    implicitHeight: bubbleRect.implicitHeight + Tokens.spacing.extraSmall

                    StyledRect {
                        id: bubbleRect

                        width: Math.min(messageText.implicitWidth + Tokens.padding.large * 2, delegateRoot.width * 0.85)
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
                            color: delegateRoot.isUser ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurface
                            wrapMode: Text.Wrap
                            font: Tokens.font.body.medium

                            anchors {
                                fill: parent
                                margins: Tokens.padding.medium
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

                    Keys.onReturnPressed: function(event) {
                        if (!(event.modifiers & Qt.ShiftModifier)) {
                            root.submitPrompt();
                            event.accepted = true;
                        }
                    }

                    StyledText {
                        anchors.fill: parent
                        text: "Ask Aetheris or dispatch task..."
                        color: Colours.palette.m3onSurfaceVariant
                        font: parent.font
                        visible: !inputField.text && !inputField.activeFocus
                    }
                }

                StyledRect {
                    id: sendButton

                    implicitWidth: 36
                    implicitHeight: 36
                    radius: Tokens.rounding.full
                    color: inputField.text.trim().length > 0 ? Colours.palette.m3primary : Colours.palette.m3surfaceContainerHighest

                    StateLayer {
                        anchors.fill: parent
                        radius: Tokens.rounding.full
                        cursorShape: inputField.text.trim().length > 0 ? Qt.PointingHandCursor : Qt.ArrowCursor
                        onClicked: root.submitPrompt()
                    }

                    MaterialIcon {
                        anchors.centerIn: parent
                        text: "arrow_upward"
                        color: inputField.text.trim().length > 0 ? Colours.palette.m3onPrimary : Colours.palette.m3onSurfaceVariant
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }
                }
            }
        }
    }
}
