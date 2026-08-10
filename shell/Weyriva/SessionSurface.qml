import QtQuick
import QtQuick.Layouts
import Quickshell

ColumnLayout {
    id: root

    property string pendingAction: ""
    spacing: 12

    function actionCommand(action) {
        switch (action) {
        case "logout":
            return ["niri", "msg", "action", "quit"]
        case "reboot":
            return ["systemctl", "reboot"]
        case "poweroff":
            return ["systemctl", "poweroff"]
        default:
            return []
        }
    }

    function requestAction(action) {
        if (action === "lock") {
            pendingAction = ""
            ShellState.requestLock()
            return
        }
        if (pendingAction !== action) {
            pendingAction = action
            confirmTimer.restart()
            ShellState.showToast("Press again to confirm " + action, "attention")
            return
        }
        const command = actionCommand(action)
        if (command.length === 0)
            return
        pendingAction = ""
        ShellState.closeRoute()
        Quickshell.execDetached(command)
    }

    Timer {
        id: confirmTimer
        interval: 7000
        repeat: false
        onTriggered: root.pendingAction = ""
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 116
        color: Theme.surfaceAlt
        radius: Theme.radius
        border.width: 1
        border.color: Theme.separator

        RowLayout {
            anchors.fill: parent
            anchors.margins: 18
            spacing: 18
            BrandMark {
                Layout.preferredWidth: 104
                Layout.preferredHeight: 78
                quiet: true
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text {
                    text: "LEAVE THE WORKSPACE"
                    color: Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Text {
                    text: "Choose a deliberate ending."
                    color: Theme.foreground
                    font.pixelSize: 20
                    font.weight: Font.DemiBold
                    font.letterSpacing: -0.35
                }
                Text {
                    text: "Power actions require a second press."
                    color: Theme.muted
                    font.pixelSize: 11
                }
            }
        }
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: "□"
        text: "Lock"
        subtitle: "Keep applications running and cover the session"
        value: "Now"
        onClicked: root.requestAction("lock")
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: "↙"
        text: "Log out"
        subtitle: "End this Niri session"
        value: root.pendingAction === "logout" ? "Confirm" : ""
        danger: root.pendingAction === "logout"
        onClicked: root.requestAction("logout")
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: "↻"
        text: "Restart"
        subtitle: "Restart the computer"
        value: root.pendingAction === "reboot" ? "Confirm" : ""
        danger: root.pendingAction === "reboot"
        onClicked: root.requestAction("reboot")
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: "○"
        text: "Power off"
        subtitle: "Shut the computer down"
        value: root.pendingAction === "poweroff" ? "Confirm" : ""
        danger: root.pendingAction === "poweroff"
        onClicked: root.requestAction("poweroff")
    }

    Item { Layout.fillHeight: true }
}
