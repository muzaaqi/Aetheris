import QtQuick
import Caelestia.Config
import qs.components

DropArea {
    id: root

    signal filesDropped(var paths)

    Rectangle {
        anchors.fill: parent
        visible: root.containsDrag
        color: Colours.tPalette.m3primary
        opacity: 0.15
        radius: Tokens.rounding.large
        border.color: Colours.palette.m3primary
        border.width: 2
        z: 100

        StyledText {
            anchors.centerIn: parent
            text: "Drop files or screenshots here to attach"
            color: Colours.palette.m3primary
            font: Tokens.font.title.small
        }
    }

    onDropped: function(drop) {
        if (!drop.hasUrls)
            return;

        const paths = [];
        for (let i = 0; i < drop.urls.length; i++) {
            let u = drop.urls[i].toString();
            if (u.startsWith("file://")) {
                u = u.substring(7);
            }
            if (u.length > 0) {
                paths.push(decodeURIComponent(u));
            }
        }

        if (paths.length > 0) {
            root.filesDropped(paths);
            drop.acceptProposedAction();
        }
    }
}
