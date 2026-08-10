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
        case "tasks": return "AI task launcher"
        case "overview": return "Window overview"
        case "clipboard": return "Clipboard"
        case "wallpaper": return "Wallpaper"
        case "settings": return "Settings"
        case "session": return "Session"
        default: return "Weyriva"
        }
    }
    readonly property bool showHeader: !["launcher", "tasks"].includes(
        presentationRoute
    )
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
        SpringAnimation { spring: 4; damping: 0.9; epsilon: 0.001 }
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
    ContinuousSurface {
        anchors.fill: parent
        fillColor: Theme.surfaceFrame
        cornerRadius: root.utility ? 24 : Theme.panelRadius
        ContinuousSurface {
            anchors.fill: parent
            anchors.margins: 4
            fillColor: Theme.surface
            cornerRadius: root.utility ? 20 : Theme.panelRadius - 4
        }
    }
    ColumnLayout {
        anchors.fill: parent
        opacity: root.routeFade
        anchors.leftMargin: root.utility ? 20
            : root.presentationRoute === "tasks" ? 12 : 30
        anchors.rightMargin: root.utility ? 20
            : root.presentationRoute === "tasks" ? 12 : 30
        anchors.topMargin: root.utility ? 18
            : root.presentationRoute === "tasks" ? 12 : 28
        anchors.bottomMargin: root.utility ? 18
            : root.presentationRoute === "tasks" ? 12 : 28
        spacing: root.utility ? 10 : 16
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
            AiTaskSurface {
                anchors.fill: parent
                visible: ShellState.route === "tasks"
                active: root.active && visible
            }
            OverviewSurface {
                anchors.fill: parent
                visible: ShellState.route === "overview"
                active: root.active && visible
            }
            ClipboardSurface {
                anchors.fill: parent
                visible: ShellState.route === "clipboard"
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
            SessionSurface {
                anchors.fill: parent
                visible: ShellState.route === "session"
            }
        }
    }
}
