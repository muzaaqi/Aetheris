#!/usr/bin/env bash
# Standalone preview runner for Aetheris HUD
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CAELESTIA_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/caelestia"
PREVIEW_DIR="/tmp/aetheris-preview"

if [ ! -d "$CAELESTIA_DIR" ]; then
    echo "Error: Caelestia Shell configuration not found at $CAELESTIA_DIR" >&2
    exit 1
fi

mkdir -p "$PREVIEW_DIR"

# Link Caelestia shell modules/services so qs.* imports resolve cleanly
ln -sfn "$CAELESTIA_DIR/components" "$PREVIEW_DIR/components"
ln -sfn "$CAELESTIA_DIR/services" "$PREVIEW_DIR/services"
ln -sfn "$CAELESTIA_DIR/utils" "$PREVIEW_DIR/utils"
ln -sfn "$CAELESTIA_DIR/assets" "$PREVIEW_DIR/assets"
ln -sfn "$PROJECT_DIR/qml" "$PREVIEW_DIR/aetheris"

# Create preview shell root
cat << 'EOF' > "$PREVIEW_DIR/shell.qml"
import QtQuick
import Quickshell
import Caelestia.Config
import qs.components
import qs.services
import "aetheris"
import "aetheris/popouts"

// qmllint disable uncreatable-type
ShellRoot {
    id: root

    AgentHudPopout {
        id: hudPopout
        visible: false
    }

    PanelWindow {
        id: previewBarWindow

        implicitWidth: 320
        implicitHeight: 48
        color: "transparent"

        anchors {
            top: true
        }

        StyledRect {
            anchors.centerIn: parent
            implicitWidth: 48
            implicitHeight: 40
            radius: Tokens.rounding.full
            color: Colours.palette.m3surfaceContainerHigh
            border.color: Colours.palette.m3outlineVariant
            border.width: 1

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
}
EOF

echo "✨ Starting Aetheris Standalone Preview..."
echo "👉 Click the floating status pill at the top-center of your screen to toggle the HUD."
echo "🛑 Press Ctrl+C in this terminal to close the preview."
echo ""

quickshell -p "$PREVIEW_DIR"
