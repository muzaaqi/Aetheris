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

    function toggle() {
        root.isOpen = !root.isOpen;
    }

    function close() {
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

        spacing: Tokens.spacing.extraSmall
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.small
        }

        StyledText {
            text: "Execution Mode"
            font: Tokens.font.label.small
            color: Colours.palette.m3onSurfaceVariant
            Layout.leftMargin: Tokens.padding.small
            Layout.topMargin: Tokens.padding.extraSmall
            Layout.bottomMargin: Tokens.padding.extraSmall
        }

        Repeater {
            id: modeRepeater

            model: [
                { id: "safe", label: "Safe Mode", icon: "shield" },
                { id: "yolo", label: "YOLO Mode", icon: "bolt" },
                { id: "plan", label: "Plan-Only Mode", icon: "edit_note" }
            ]

            delegate: StyledRect {
                id: modeRect

                required property var modelData
                readonly property bool isSelected: root.currentMode === modelData.id

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
