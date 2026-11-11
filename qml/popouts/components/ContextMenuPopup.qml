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

    function toggle() {
        root.isOpen = !root.isOpen;
    }

    function open() {
        root.isOpen = true;
    }

    function close() {
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
