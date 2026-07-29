import QtQuick

Item {
    id: root

    required property var sourceScreen

    implicitWidth: 760
    implicitHeight: 40

    Rectangle {
        anchors.fill: parent
        color: Theme.chrome
        radius: 11
        border.width: 1
        border.color: Theme.separator
    }

    Row {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: 5
        spacing: 2

        ActionButton {
            glyph: "W"
            text: "Applications"
            compact: true
            chrome: true
            selected: ShellState.route === "launcher"
                && ShellState.routeScreen === root.sourceScreen
            onClicked: ShellState.toggleRoute("launcher", root.sourceScreen)
        }

        ActionButton {
            glyph: "○"
            text: "Control center"
            compact: true
            chrome: true
            selected: ShellState.route === "control-center"
                && ShellState.routeScreen === root.sourceScreen
            onClicked: ShellState.toggleRoute(
                "control-center", root.sourceScreen
            )
        }
    }

    ActionButton {
        anchors.centerIn: parent
        text: Qt.formatDateTime(ShellState.now, "ddd  MMM d  hh:mm")
        chrome: true
        selected: ShellState.route === "calendar"
            && ShellState.routeScreen === root.sourceScreen
        onClicked: ShellState.toggleRoute("calendar", root.sourceScreen)
    }

    Row {
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.rightMargin: 5
        spacing: 2

        ActionButton {
            glyph: "•"
            text: "Notifications"
            compact: true
            chrome: true
            selected: ShellState.route === "notifications"
                && ShellState.routeScreen === root.sourceScreen
            onClicked: ShellState.toggleRoute(
                "notifications", root.sourceScreen
            )
        }

        ActionButton {
            glyph: "▧"
            text: "Wallpaper"
            compact: true
            chrome: true
            selected: ShellState.route === "wallpaper"
                && ShellState.routeScreen === root.sourceScreen
            onClicked: ShellState.toggleRoute("wallpaper", root.sourceScreen)
        }

        ActionButton {
            glyph: "···"
            text: "Settings"
            compact: true
            chrome: true
            selected: ShellState.route === "settings"
                && ShellState.routeScreen === root.sourceScreen
            onClicked: ShellState.toggleRoute("settings", root.sourceScreen)
        }

        ActionButton {
            glyph: "□"
            text: "Lock"
            compact: true
            chrome: true
            onClicked: ShellState.requestLock()
        }
    }
}
