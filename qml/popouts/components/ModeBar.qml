import QtQuick
import QtQuick.Layouts
import Caelestia.Config
import qs.components

StyledRect {
    id: root

    property string currentMode: "safe"
    property string thinkingBudget: "off"

    signal modeSelected(string mode)
    signal thinkingSelected(string budget)

    implicitHeight: 38
    radius: Tokens.rounding.full
    color: Colours.palette.m3surfaceContainer

    RowLayout {
        anchors {
            fill: parent
            leftMargin: Tokens.padding.small
            rightMargin: Tokens.padding.small
        }
        spacing: Tokens.spacing.extraSmall

        // Mode Segmented Pills
        RowLayout {
            spacing: Tokens.spacing.extraSmall

            Repeater {
                model: [
                    { id: "safe", label: "Safe", icon: "shield" },
                    { id: "yolo", label: "YOLO", icon: "bolt" },
                    { id: "plan", label: "Plan", icon: "edit_note" }
                ]

                delegate: StyledRect {
                    id: modeBtn

                    required property var modelData

                    readonly property bool isSelected: root.currentMode === modelData.id

                    implicitHeight: 28
                    implicitWidth: modeRow.implicitWidth + Tokens.padding.medium
                    radius: Tokens.rounding.full
                    color: isSelected ? Colours.palette.m3primaryContainer : "transparent"

                    RowLayout {
                        id: modeRow
                        anchors.centerIn: parent
                        spacing: Tokens.spacing.extraSmall

                        MaterialIcon {
                            text: modeBtn.modelData.icon
                            color: modeBtn.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                            fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                        }

                        StyledText {
                            text: modeBtn.modelData.label
                            color: modeBtn.isSelected ? Colours.palette.m3onPrimaryContainer : Colours.palette.m3onSurfaceVariant
                            font: Tokens.font.label.small
                        }
                    }

                    StateLayer {
                        anchors.fill: parent
                        radius: Tokens.rounding.full
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.currentMode = modeBtn.modelData.id;
                            root.modeSelected(modeBtn.modelData.id);
                        }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
        }

        // Thinking Budget Chip
        StyledRect {
            id: thinkingChip

            implicitHeight: 28
            implicitWidth: thinkingRow.implicitWidth + Tokens.padding.medium
            radius: Tokens.rounding.full
            color: root.thinkingBudget !== "off" ? Colours.palette.m3tertiaryContainer : "transparent"

            RowLayout {
                id: thinkingRow
                anchors.centerIn: parent
                spacing: Tokens.spacing.extraSmall

                MaterialIcon {
                    text: "psychology"
                    color: root.thinkingBudget !== "off" ? Colours.palette.m3onTertiaryContainer : Colours.palette.m3outline
                    fontStyle: Tokens.font.icon.builders.small.weight(Font.Normal).build()
                }

                StyledText {
                    text: root.thinkingBudget === "off" ? "Think: Off" : "Think: " + root.thinkingBudget.toUpperCase()
                    color: root.thinkingBudget !== "off" ? Colours.palette.m3onTertiaryContainer : Colours.palette.m3outline
                    font: Tokens.font.label.small
                }
            }

            StateLayer {
                anchors.fill: parent
                radius: Tokens.rounding.full
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    const next = root.thinkingBudget === "off" ? "low"
                        : root.thinkingBudget === "low" ? "medium"
                        : root.thinkingBudget === "medium" ? "high" : "off";
                    root.thinkingBudget = next;
                    root.thinkingSelected(next);
                }
            }
        }
    }
}
