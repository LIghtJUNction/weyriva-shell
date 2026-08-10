//@ pragma StateDir $BASE/weyriva
//@ pragma DataDir $BASE/weyriva
//@ pragma CacheDir $BASE/weyriva
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Services.Pam
import Quickshell.Wayland
import qs.Weyriva
ShellRoot {
    id: root
    OsdController { id: osdController }
    StatusOverlayHost {}
    function defaultRouteScreen() {
        return Quickshell.screens.length > 0 ? Quickshell.screens[0] : null
    }
    Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: ShellState.now = new Date()
    }
    Connections {
        target: ShellState
        function onRequestLock() {
            ShellState.closeRoute()
            sessionLock.locked = true
        }
    }
    NotificationServer {
        id: notifications
        bodySupported: true
        actionsSupported: true
        persistenceSupported: true
        keepOnReload: true
        onNotification: notification => {
            notification.tracked = true
            if (!ShellState.doNotDisturb)
                ShellState.openRoute("notifications", root.defaultRouteScreen())
        }
    }
    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                required property var modelData
                screen: modelData
                visible: ShellState.barVisible
                aboveWindows: true
                anchors { top: true; left: true; right: true }
                implicitHeight: 74
                exclusiveZone: 74
                WlrLayershell.layer: WlrLayer.Top
                mask: Region { item: topBar }
                color: "transparent"
                TopBar {
                    id: topBar
                    anchors.top: parent.top
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.topMargin: 10
                    width: Math.min(parent.width - 28, 1000)
                    sourceScreen: modelData
                }
            }
        }
    }
    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                id: utilityHost
                required property var modelData
                readonly property real barWidth: Math.min(width - 28, 1000)
                readonly property real barLeft: (width - barWidth) / 2
                readonly property real barRight: barLeft + barWidth
                property bool active: [
                    "control-center", "calendar", "notifications"
                ].includes(ShellState.route)
                    && ShellState.routeScreenName === modelData.name
                readonly property real controlX: boundedX(barLeft)
                readonly property real calendarX: boundedX(
                    width / 2 - utilitySurface.width / 2
                )
                readonly property real notificationsX: boundedX(
                    barRight - utilitySurface.width
                )
                readonly property real routeX:
                    ShellState.presentationRoute === "control-center" ? controlX
                    : ShellState.presentationRoute === "calendar" ? calendarX
                    : notificationsX
                function boundedX(value) {
                    return Math.max(10, Math.min(
                        value, width - utilitySurface.width - 10
                    ))
                }
                screen: modelData
                anchors { top: true; left: true; right: true; bottom: true }
                visible: true
                aboveWindows: true
                focusable: active
                exclusionMode: ExclusionMode.Ignore
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.keyboardFocus: active
                    ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
                mask: Region { item: active ? utilitySurface : null }
                color: "transparent"
                SurfacePanel {
                    id: utilitySurface
                    anchors.top: parent.top
                    anchors.topMargin: 78
                    x: utilityHost.routeX
                    width: Math.min(410, parent.width - 24)
                    height: ShellState.presentationRoute === "control-center" ? 310
                        : ShellState.presentationRoute === "calendar" ? 470 : 440
                    notificationServer: notifications
                    presentation: "utility"
                    presentationRoute: ShellState.presentationRoute
                    active: utilityHost.active
                    sourceOffsetX: ShellState.presentationRoute === "control-center" ? -14
                        : ShellState.presentationRoute === "calendar" ? 0 : 14
                    sourceOrigin: ShellState.presentationRoute === "control-center"
                        ? Item.TopLeft : ShellState.presentationRoute === "calendar"
                            ? Item.Top : Item.TopRight
                }
            }
        }
    }
    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                id: centeredHost
                required property var modelData
                property bool active: [
                    "launcher", "tasks", "overview", "clipboard",
                    "wallpaper", "settings", "session"
                ].includes(ShellState.route)
                    && ShellState.routeScreenName === modelData.name
                screen: modelData
                anchors { top: true; left: true; right: true; bottom: true }
                visible: true
                aboveWindows: true
                focusable: active
                exclusionMode: ExclusionMode.Ignore
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.keyboardFocus: active
                    ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
                mask: Region { item: active ? centeredSurface : null }
                color: "transparent"
                Rectangle {
                    anchors.fill: parent
                    color: Theme.scrim
                    opacity: centeredHost.active ? 1 : 0
                    visible: opacity > 0
                    Behavior on opacity {
                        NumberAnimation {
                            duration: ShellState.reducedMotion ? 90 : 170
                            easing.type: Easing.OutCubic
                        }
                    }
                }
                SurfacePanel {
                    id: centeredSurface
                    anchors.centerIn: parent
                    width: Math.min(
                        parent.width - 80,
                        ShellState.route === "tasks" ? 1340
                            : ["overview", "clipboard"].includes(ShellState.route)
                                ? 820 : ShellState.route === "session" ? 680 : 760
                    )
                    height: Math.min(
                        parent.height - 136,
                        ShellState.route === "tasks" ? 840
                            : ShellState.route === "launcher" ? 620
                                : ShellState.route === "session" ? 600 : 560
                    )
                    notificationServer: notifications
                    presentation: "centered"
                    presentationRoute: ShellState.route
                    active: centeredHost.active
                    sourceOffsetX: ["launcher", "tasks"].includes(
                        ShellState.route
                    ) ? -10 : 0
                    sourceOrigin: ["launcher", "tasks"].includes(
                        ShellState.route
                    ) ? Item.TopLeft : Item.Top
                }
            }
        }
    }
    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                required property var modelData
                screen: modelData
                anchors { top: true; left: true; right: true; bottom: true }
                aboveWindows: false
                exclusionMode: ExclusionMode.Ignore
                WlrLayershell.layer: WlrLayer.Background
                WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
                mask: Region { item: null }
                color: Theme.background
                Image {
                    anchors.fill: parent
                    source: ShellState.wallpaper
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }
            }
        }
    }
    WlSessionLock {
        id: sessionLock
        locked: false
        WlSessionLockSurface {
            color: Theme.background
            LockSurface {
                id: lockView
                anchors.fill: parent
                authContext: auth
            }
        }
    }
    PamContext {
        id: auth
        property string pendingResponse: ""
        config: "login"
        onPamMessage: {
            if (responseRequired) {
                respond(pendingResponse)
                pendingResponse = ""
            }
        }
        onCompleted: result => {
            pendingResponse = ""
            lockView.clearPassword()
            if (result === PamResult.Success) {
                sessionLock.locked = false
                ShellState.closeRoute()
            } else {
                lockView.focusPassword()
            }
        }
    }
    IpcHandler {
        target: "weyriva"
        function route(name: string): void {
            ShellState.toggleRoute(name, root.defaultRouteScreen())
        }
        function osd(kind: string, direction: string): void {
            osdController.adjust(kind, direction)
        }
        function lock(): void { ShellState.requestLock() }
        function clearNotifications(): void {
            const values = notifications.trackedNotifications.values
            for (let index = values.length - 1; index >= 0; --index)
                values[index].dismiss()
        }
        function toggleDnd(): void {
            ShellState.doNotDisturb = !ShellState.doNotDisturb
        }
        function setDnd(enabled: bool): void {
            ShellState.doNotDisturb = enabled
        }
        function toggleBar(): void {
            ShellState.barVisible = !ShellState.barVisible
        }
        function reload(): void { Quickshell.reload(false) }
        function status(): string {
            const visibleRoute = ShellState.route === ""
                ? "desktop" : ShellState.route
            const appearance = ShellState.dark ? "dark" : "light"
            const lockState = sessionLock.secure ? "secure" : "ready"
            return "route=" + visibleRoute
                + ";theme=" + appearance
                + ";lock=" + lockState
        }
    }
}
