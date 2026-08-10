import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property var sourceScreen

    implicitWidth: 1000
    implicitHeight: 54

    function routeIs(name) {
        return ShellState.route === name
            && ShellState.routeScreenName === root.sourceScreen.name
    }

    RowLayout {
        anchors.fill: parent
        spacing: 10

        Item {
            Layout.preferredWidth: 226
            Layout.fillHeight: true
            ContinuousSurface {
                anchors.fill: parent
                fillColor: Theme.surfaceFrame
                cornerRadius: 27
                ContinuousSurface {
                    anchors.fill: parent
                    anchors.margins: 3
                    fillColor: Theme.chrome
                    cornerRadius: 24
                }
            }
            RowLayout {
                anchors.fill: parent
                anchors.margins: 7
                spacing: 4
                ActionButton {
                    text: "Search"
                    chrome: true
                    selected: root.routeIs("launcher")
                    onClicked: ShellState.toggleRoute(
                        "launcher", root.sourceScreen
                    )
                }
                ActionButton {
                    glyph: "✦"
                    text: "New task"
                    chrome: true
                    emphasized: !root.routeIs("tasks")
                    selected: root.routeIs("tasks")
                    onClicked: ShellState.toggleRoute("tasks", root.sourceScreen)
                }
            }
        }

        Item {
            Layout.preferredWidth: 264
            Layout.fillHeight: true
            ContinuousSurface {
                anchors.fill: parent
                fillColor: Theme.surfaceFrame
                cornerRadius: 27
                ContinuousSurface {
                    anchors.fill: parent
                    anchors.margins: 3
                    fillColor: Theme.chrome
                    cornerRadius: 24
                }
            }
            RowLayout {
                anchors.fill: parent
                anchors.margins: 7
                spacing: 4
                WorkspaceRail {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    sourceScreen: root.sourceScreen
                }
                ActionButton {
                    glyph: "▦"
                    text: "Window overview"
                    compact: true
                    chrome: true
                    selected: root.routeIs("overview")
                    onClicked: ShellState.toggleRoute(
                        "overview", root.sourceScreen
                    )
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            ContinuousSurface {
                anchors.fill: parent
                fillColor: Theme.surfaceFrame
                cornerRadius: 27
                ContinuousSurface {
                    anchors.fill: parent
                    anchors.margins: 3
                    fillColor: Theme.chrome
                    cornerRadius: 24
                }
            }
            RowLayout {
                anchors.fill: parent
                anchors.margins: 7
                spacing: 3
                ActionButton {
                    Layout.fillWidth: true
                    text: Qt.formatDateTime(
                        ShellState.now, "ddd  MMM d  hh:mm"
                    )
                    chrome: true
                    selected: root.routeIs("calendar")
                    onClicked: ShellState.toggleRoute(
                        "calendar", root.sourceScreen
                    )
                }
                DndChip {}
                ActionButton {
                    glyph: ShellState.doNotDisturb ? "–" : "•"
                    text: "Notifications"
                    compact: true
                    chrome: true
                    selected: root.routeIs("notifications")
                    onClicked: ShellState.toggleRoute(
                        "notifications", root.sourceScreen
                    )
                }
                ActionButton {
                    glyph: "○"
                    text: "Control center"
                    compact: true
                    chrome: true
                    selected: root.routeIs("control-center")
                    onClicked: ShellState.toggleRoute(
                        "control-center", root.sourceScreen
                    )
                }
                ActionButton {
                    glyph: "↗"
                    text: "Session actions"
                    compact: true
                    chrome: true
                    selected: root.routeIs("session")
                    onClicked: ShellState.toggleRoute(
                        "session", root.sourceScreen
                    )
                }
            }
        }
    }
}
