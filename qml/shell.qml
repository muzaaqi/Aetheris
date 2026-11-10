import QtQuick
import Quickshell
import "popouts"

// qmllint disable uncreatable-type
ShellRoot {
    id: root

    AgentHudPopout {
        id: hudPopout
        visible: false
    }

    PanelWindow {
        id: previewBarWindow

        implicitWidth: 300
        implicitHeight: 48
        color: "transparent"

        anchors {
            top: true
            horizontalCenter: true
        }

        AgentHudButton {
            anchors.centerIn: parent
            isConnected: hudPopout.isConnected
            isThinking: hudPopout.isThinking
            popoutOpen: hudPopout.visible

            onToggled: {
                hudPopout.visible = !hudPopout.visible;
            }
        }
    }
}
