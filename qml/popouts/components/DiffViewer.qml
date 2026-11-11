import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components
import qs.services

StyledRect {
    id: root

    property string diffText: ""
    property string fileLabel: "Diff"
    property bool expanded: true

    readonly property var allDiffLines: {
        if (!root.diffText) return [];
        return root.diffText.replace(/\r?\n+$/, "").split(/\r?\n/);
    }
    readonly property int maxDisplayLines: 100
    readonly property bool isTruncated: allDiffLines.length > maxDisplayLines
    readonly property var diffLines: isTruncated ? allDiffLines.slice(0, maxDisplayLines) : allDiffLines

    readonly property int addedCount: {
        let count = 0;
        for (let i = 0; i < allDiffLines.length; i++) {
            if (allDiffLines[i].startsWith("+") && !allDiffLines[i].startsWith("+++")) count++;
        }
        return count;
    }

    readonly property int deletedCount: {
        let count = 0;
        for (let i = 0; i < allDiffLines.length; i++) {
            if (allDiffLines[i].startsWith("-") && !allDiffLines[i].startsWith("---")) count++;
        }
        return count;
    }

    implicitWidth: 380
    implicitHeight: mainCol.implicitHeight
    radius: Tokens.rounding.medium
    color: Colours.palette.m3surfaceContainerLowest
    border.color: Colours.palette.m3outlineVariant
    border.width: 1
    clip: true

    ColumnLayout {
        id: mainCol

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
        }
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
                        font: Tokens.font.mono.small
                        elide: Text.ElideRight
                        color: lineRect.isAddition ? Colours.palette.m3onTertiaryContainer
                             : lineRect.isDeletion ? Colours.palette.m3onErrorContainer
                             : lineRect.isHunkHeader ? Colours.palette.m3primary
                             : Colours.palette.m3onSurfaceVariant
                    }
                }
            }

            // Truncation Indicator
            StyledRect {
                visible: root.isTruncated
                Layout.fillWidth: true
                implicitHeight: 24
                color: Colours.palette.m3surfaceContainerHigh

                StyledText {
                    anchors.centerIn: parent
                    text: "... [" + (root.allDiffLines.length - root.maxDisplayLines) + " more lines collapsed]"
                    font: Tokens.font.label.small
                    color: Colours.palette.m3outline
                }
            }
        }
    }
}
