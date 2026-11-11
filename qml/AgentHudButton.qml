import QtQuick
import Caelestia.Config
import qs.components
import qs.services

Item {
    id: root

    property bool isConnected: false
    property bool isThinking: false
    property bool popoutOpen: false

    signal toggled()

    implicitWidth: Tokens.sizes.bar.innerWidth
    implicitHeight: Tokens.sizes.bar.innerWidth

    StateLayer {
        anchors.fill: parent
        radius: Tokens.rounding.full
        onClicked: root.toggled()
    }

    MaterialIcon {
        id: mainIcon

        anchors.centerIn: parent
        text: "smart_toy"
        color: {
            if (!root.isConnected)
                return Colours.palette.m3outline;
            if (root.isThinking)
                return Colours.palette.m3tertiary;
            return Colours.palette.m3primary;
        }
        fontStyle: Tokens.font.icon.builders.small.weight(Font.Bold).build()
    }

    Rectangle {
        id: statusDot

        width: 6
        height: 6
        radius: Tokens.rounding.full

        color: {
            if (!root.isConnected)
                return Colours.palette.m3error;
            if (root.isThinking)
                return Colours.palette.m3tertiary;
            return Colours.palette.m3primary;
        }

        anchors {
            right: parent.right
            bottom: parent.bottom
            margins: 2
        }
    }
}
