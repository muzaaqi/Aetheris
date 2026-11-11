import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property string toolCallId: ""
    property string toolName: ""
    property string toolArgs: ""
    property bool requiresApproval: true
    property string status: "pending"
    property string diffText: ""

    signal approvalDecided(string toolCallId, bool approved)

    readonly property bool isPending: root.status === "pending" && root.requiresApproval

    implicitWidth: parent ? parent.width : 420
    implicitHeight: mainCol.implicitHeight + Tokens.padding.medium * 2
    radius: Tokens.rounding.large
    color: Colours.palette.m3surfaceContainerHigh
    border.color: isPending ? Colours.palette.m3primary : Colours.palette.m3outlineVariant
    border.width: isPending ? 2 : 1

    ColumnLayout {
        id: mainCol

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Tokens.padding.medium
        }
        spacing: Tokens.spacing.small

        // Tool Title & Status Row
        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spacing.small

            MaterialIcon {
                text: {
                    const n = root.toolName.toLowerCase();
                    if (n.includes("command") || n.includes("bash") || n.includes("terminal")) return "terminal";
                    if (n.includes("file") || n.includes("write") || n.includes("edit")) return "edit_note";
                    return "extension";
                }
                color: root.isPending ? Colours.palette.m3primary : Colours.palette.m3onSurfaceVariant
                fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
            }

            StyledText {
                text: root.toolName
                font: Tokens.font.title.small
                color: Colours.palette.m3onSurface
            }

            Item { Layout.fillWidth: true }

            // Status Badge
            StyledRect {
                implicitHeight: 24
                implicitWidth: statusRow.implicitWidth + Tokens.padding.small * 2
                radius: Tokens.rounding.full
                color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3primaryContainer
                     : root.status === "rejected" ? Colours.palette.m3errorContainer
                     : Colours.palette.m3secondaryContainer

                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: root.status === "approved" || root.status === "completed" ? "check_circle"
                            : root.status === "rejected" ? "cancel"
                            : "hourglass_top"
                        color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3onPrimaryContainer
                             : root.status === "rejected" ? Colours.palette.m3onErrorContainer
                             : Colours.palette.m3onSecondaryContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                    }

                    StyledText {
                        text: root.status === "approved" ? "Approved"
                            : root.status === "rejected" ? "Rejected"
                            : root.status === "completed" ? "Executed"
                            : "Pending Approval"
                        color: root.status === "approved" || root.status === "completed" ? Colours.palette.m3onPrimaryContainer
                             : root.status === "rejected" ? Colours.palette.m3onErrorContainer
                             : Colours.palette.m3onSecondaryContainer
                        font: Tokens.font.label.small
                    }
                }
            }
        }

        // Arguments snippet
        StyledRect {
            visible: Boolean(root.toolArgs)
            Layout.fillWidth: true
            implicitHeight: Math.min(argsTextItem.implicitHeight + Tokens.padding.small * 2, 80)
            radius: Tokens.rounding.small
            color: Colours.palette.m3surfaceContainerLowest
            clip: true

            StyledText {
                id: argsTextItem
                anchors {
                    fill: parent
                    margins: Tokens.padding.small
                }
                text: root.toolArgs
                font: Tokens.font.body.small
                font.family: "monospace"
                color: Colours.palette.m3onSurfaceVariant
                wrapMode: Text.WrapAnywhere
            }
        }

        // Embedded Diff Viewer
        DiffViewer {
            visible: Boolean(root.diffText)
            Layout.fillWidth: true
            diffText: root.diffText
            fileLabel: root.toolName
        }

        // Action Buttons Row (only if pending approval)
        RowLayout {
            visible: root.isPending
            Layout.fillWidth: true
            spacing: Tokens.spacing.small

            Item { Layout.fillWidth: true }

            // Reject Button
            StyledRect {
                id: rejectBtn
                implicitHeight: 32
                implicitWidth: rejectRow.implicitWidth + Tokens.padding.medium
                radius: Tokens.rounding.full
                color: Colours.palette.m3errorContainer

                RowLayout {
                    id: rejectRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: "close"
                        color: Colours.palette.m3onErrorContainer
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }

                    StyledText {
                        text: "Reject"
                        color: Colours.palette.m3onErrorContainer
                        font: Tokens.font.label.small
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.full
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.status = "rejected";
                        root.approvalDecided(root.toolCallId, false);
                    }
                }
            }

            // Approve Button
            StyledRect {
                id: approveBtn
                implicitHeight: 32
                implicitWidth: approveRow.implicitWidth + Tokens.padding.medium
                radius: Tokens.rounding.full
                color: Colours.palette.m3primary

                RowLayout {
                    id: approveRow
                    anchors.centerIn: parent
                    spacing: Tokens.spacing.extraSmall

                    MaterialIcon {
                        text: "check"
                        color: Colours.palette.m3onPrimary
                        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
                    }

                    StyledText {
                        text: "Approve"
                        color: Colours.palette.m3onPrimary
                        font: Tokens.font.label.small
                    }
                }

                StateLayer {
                    anchors.fill: parent
                    radius: Tokens.rounding.full
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.status = "approved";
                        root.approvalDecided(root.toolCallId, true);
                    }
                }
            }
        }
    }
}
