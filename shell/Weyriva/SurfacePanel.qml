import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property var notificationServer
    required property string presentation
    required property string presentationRoute
    required property bool active
    required property real sourceOffsetX
    required property int sourceOrigin

    readonly property bool utility: presentation === "utility"
    readonly property string title: {
        switch (presentationRoute) {
        case "control-center": return "Controls"
        case "calendar": return "Calendar"
        case "notifications": return "Notifications"
        case "wallpaper": return "Wallpaper"
        case "settings": return "Settings"
        default: return "Weyriva"
        }
    }
    readonly property bool showHeader: presentationRoute !== "launcher"
    property real routeFade: 1

    opacity: active ? 1 : 0
    scale: active || ShellState.reducedMotion ? 1 : (utility ? 1 : 0.985)
    transformOrigin: sourceOrigin
    focus: active
    Keys.onEscapePressed: ShellState.closeRoute()

    Behavior on x {
        enabled: root.utility && root.active && !ShellState.reducedMotion
        SmoothedAnimation {
            duration: Theme.motionPanel
            velocity: 520
        }
    }

    Behavior on height {
        enabled: root.utility && root.active && !ShellState.reducedMotion
        SmoothedAnimation {
            duration: Theme.motionPanel
            velocity: 520
        }
    }

    transform: Translate {
        id: slide

        x: root.active || ShellState.reducedMotion ? 0 : root.sourceOffsetX
        y: root.active || ShellState.reducedMotion ? 0
            : (root.utility ? -5 : 12)

        Behavior on x {
            enabled: !ShellState.reducedMotion
            SmoothedAnimation {
                duration: Theme.motionPanel
                velocity: 260
            }
        }

        Behavior on y {
            enabled: !ShellState.reducedMotion
            SmoothedAnimation {
                duration: Theme.motionPanel
                velocity: 260
            }
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: ShellState.reducedMotion ? 90 : 155
            easing.type: Easing.OutCubic
        }
    }

    Behavior on scale {
        enabled: !ShellState.reducedMotion
        SmoothedAnimation {
            duration: Theme.motionPanel
            velocity: 2.5
        }
    }

    onActiveChanged: {
        if (active)
            forceActiveFocus()
    }

    onPresentationRouteChanged: {
        if (utility && active && ShellState.reducedMotion)
            routeCrossFade.restart()
    }

    SequentialAnimation {
        id: routeCrossFade

        NumberAnimation {
            target: root
            property: "routeFade"
            to: 0.35
            duration: 45
            easing.type: Easing.OutCubic
        }
        NumberAnimation {
            target: root
            property: "routeFade"
            to: 1
            duration: 45
            easing.type: Easing.InCubic
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.surface
        radius: root.utility ? 14 : 18
        border.width: 1
        border.color: Theme.separator
    }

    ColumnLayout {
        anchors.fill: parent
        opacity: root.routeFade
        anchors.leftMargin: root.utility ? 18 : 22
        anchors.rightMargin: root.utility ? 18 : 22
        anchors.topMargin: root.utility ? 16 : 20
        anchors.bottomMargin: root.utility ? 16 : 20
        spacing: root.utility ? 10 : 14

        SurfaceHeader {
            Layout.fillWidth: true
            visible: root.showHeader
            title: root.title
            utility: root.utility
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            LauncherSurface {
                anchors.fill: parent
                visible: ShellState.route === "launcher"
                active: root.active && visible
            }

            ControlCenterSurface {
                anchors.fill: parent
                visible: ShellState.route === "control-center"
            }

            CalendarSurface {
                anchors.fill: parent
                visible: ShellState.route === "calendar"
            }

            NotificationsSurface {
                anchors.fill: parent
                visible: ShellState.route === "notifications"
                notificationServer: root.notificationServer
            }

            WallpaperSurface {
                anchors.fill: parent
                visible: ShellState.route === "wallpaper"
            }

            SettingsSurface {
                anchors.fill: parent
                visible: ShellState.route === "settings"
            }
        }
    }
}
