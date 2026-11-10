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
