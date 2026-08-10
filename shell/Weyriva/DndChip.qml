import QtQuick
import QtQuick.Controls

Button {
    id: control

    implicitWidth: 48
    implicitHeight: 36
    hoverEnabled: true
    scale: down && !ShellState.reducedMotion ? 0.965 : 1
    Accessible.name: ShellState.doNotDisturb
        ? "Do not disturb is on" : "Do not disturb is off"
    ToolTip.visible: hovered
    ToolTip.text: ShellState.doNotDisturb
        ? "Allow notification popups" : "Mute notification popups"
    ToolTip.delay: 500

    Behavior on scale {
        enabled: !control.down && !ShellState.reducedMotion
        SpringAnimation { spring: 5; damping: 0.9; epsilon: 0.001 }
    }

    onClicked: {
        ShellState.doNotDisturb = !ShellState.doNotDisturb
        ShellState.showToast(
            ShellState.doNotDisturb
                ? "Do not disturb enabled" : "Notifications restored",
            ShellState.doNotDisturb ? "attention" : "info"
        )
    }

    contentItem: Row {
        anchors.centerIn: parent
        spacing: 4

        Text {
            text: ShellState.doNotDisturb ? "–" : "•"
            color: ShellState.doNotDisturb
                ? Theme.accentWarm : Theme.foreground
            font.pixelSize: 14
            font.weight: Font.Bold
        }

        Text {
            text: "DND"
            color: Theme.foreground
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.4
        }
    }

    background: Item {
        ContinuousSurface {
            anchors.fill: parent
            fillColor: control.activeFocus ? Theme.focusRing : "transparent"
            cornerRadius: height / 2
        }
        ContinuousSurface {
            anchors.fill: parent
            anchors.margins: control.activeFocus ? 2 : 0
            fillColor: control.down ? Theme.pressed
                : ShellState.doNotDisturb ? Theme.selection
                : control.hovered ? Theme.hover : "transparent"
            cornerRadius: height / 2
        }
    }
}
