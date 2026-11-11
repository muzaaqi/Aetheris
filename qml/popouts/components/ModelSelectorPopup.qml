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

    function toggle() {
        root.isOpen = !root.isOpen;
    }

    function close() {
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

        spacing: Tokens.spacing.extraSmall
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.small
        }

        StyledText {
            text: "Select Model / Platform"
            font: Tokens.font.label.small
            color: Colours.palette.m3onSurfaceVariant
            Layout.leftMargin: Tokens.padding.small
            Layout.topMargin: Tokens.padding.extraSmall
            Layout.bottomMargin: Tokens.padding.extraSmall
        }

        Repeater {
            id: modelRepeater

            model: [
                { id: "antigravity", label: "Antigravity", icon: "smart_toy" },
                { id: "claude", label: "Claude Code", icon: "terminal" },
                { id: "direct", label: "Direct LLM API", icon: "api" }
            ]

            delegate: StyledRect {
                id: platformRect

                required property var modelData
                readonly property bool isSelected: root.currentPlatform === modelData.id

                implicitHeight: 36
                radius: Tokens.rounding.medium
                color: isSelected ? Colours.palette.m3primaryContainer : "transparent"
                Layout.fillWidth: true

                RowLayout {
                    spacing: Tokens.spacing.small
                    anchors {
                        fill: parent
                        leftMargin: Tokens.padding.small
                        rightMargin: Tokens.padding.small
                    }

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
