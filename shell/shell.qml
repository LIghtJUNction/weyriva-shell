//@ pragma StateDir $BASE/weyriva
//@ pragma DataDir $BASE/weyriva
//@ pragma CacheDir $BASE/weyriva
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Services.Pam
import Quickshell.Wayland
import qs.Weyriva

ShellRoot {
    id: root

    Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: ShellState.now = new Date()
    }

    Connections {
        target: ShellState
        function onRequestLock() {
            ShellState.route = ""
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
                ShellState.route = "notifications"
        }
    }

    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                required property var modelData
                screen: modelData
                visible: ShellState.barVisible
                anchors { top: true; left: true; right: true }
                implicitHeight: 64
                exclusiveZone: 64
                mask: Region { item: barSurface }
                color: "transparent"

                Rectangle {
                    id: barSurface
                    anchors.top: parent.top
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.topMargin: 8
                    width: Math.min(parent.width - 24, 900)
                    height: 48
                    color: Theme.chrome
                    radius: 24
                    border.width: 3
                    border.color: Theme.ink

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 5

                        ActionButton {
                            glyph: "W"
                            text: "Applications"
                            compact: true
                            chrome: true
                            selected: ShellState.route === "launcher"
                            onClicked: ShellState.toggleRoute("launcher")
                        }
                        ActionButton {
                            glyph: "◉"
                            text: "Control center"
                            compact: true
                            chrome: true
                            selected: ShellState.route === "control-center"
                            onClicked: ShellState.toggleRoute("control-center")
                        }
                        Item { Layout.fillWidth: true }
                        ActionButton {
                            text: Qt.formatDateTime(ShellState.now, "ddd  MMM d  hh:mm")
                            chrome: true
                            selected: ShellState.route === "calendar"
                            onClicked: ShellState.toggleRoute("calendar")
                        }
                        ActionButton {
                            glyph: "●"
                            text: "Notifications"
                            compact: true
                            chrome: true
                            selected: ShellState.route === "notifications"
                            onClicked: ShellState.toggleRoute("notifications")
                        }
                        ActionButton {
                            glyph: "✦"
                            text: "Wallpaper"
                            compact: true
                            chrome: true
                            selected: ShellState.route === "wallpaper"
                            onClicked: ShellState.toggleRoute("wallpaper")
                        }
                        ActionButton {
                            glyph: "⚙"
                            text: "Settings"
                            compact: true
                            chrome: true
                            selected: ShellState.route === "settings"
                            onClicked: ShellState.toggleRoute("settings")
                        }
                        ActionButton {
                            glyph: "▣"
                            text: "Lock"
                            compact: true
                            chrome: true
                            onClicked: ShellState.requestLock()
                        }
                    }
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
                property bool active: [
                    "control-center",
                    "calendar",
                    "notifications"
                ].includes(ShellState.route)

                screen: modelData
                anchors { top: true; left: true; right: true; bottom: true }
                visible: true
                focusable: active
                exclusionMode: ExclusionMode.Ignore
                WlrLayershell.keyboardFocus: active
                    ? WlrKeyboardFocus.OnDemand
                    : WlrKeyboardFocus.None
                mask: Region { item: active ? utilitySurface : null }
                color: "transparent"

                SurfacePanel {
                    id: utilitySurface
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.topMargin: 72
                    anchors.rightMargin: 12
                    width: 430
                    height: ShellState.route === "control-center" ? 350 : 520
                    notificationServer: notifications
                    presentation: "utility"
                    active: utilityHost.active
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
                    "launcher",
                    "wallpaper",
                    "settings"
                ].includes(ShellState.route)

                screen: modelData
                anchors { top: true; left: true; right: true; bottom: true }
                visible: true
                focusable: active
                exclusionMode: ExclusionMode.Ignore
                WlrLayershell.keyboardFocus: active
                    ? WlrKeyboardFocus.OnDemand
                    : WlrKeyboardFocus.None
                mask: Region { item: active ? centeredSurface : null }
                color: "transparent"

                SurfacePanel {
                    id: centeredSurface
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 72, 760)
                    height: ShellState.route === "launcher"
                        ? Math.min(parent.height - 120, 620)
                        : Math.min(parent.height - 140, 540)
                    notificationServer: notifications
                    presentation: "centered"
                    active: centeredHost.active
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
            color: Theme.carrier

            Item {
                anchors.fill: parent

                BrandMark {
                    visible: parent.width > 860
                    width: 310
                    height: 238
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: Math.max(70, parent.width * 0.10)
                }

                Item {
                    id: lockCard
                    anchors.centerIn: parent
                    anchors.horizontalCenterOffset: parent.width > 860
                        ? Math.min(250, parent.width * 0.18) : 0
                    width: Math.min(480, parent.width - 56)
                    height: 370

                    Canvas {
                        anchors.fill: parent
                        onWidthChanged: requestPaint()
                        onHeightChanged: requestPaint()
                        onPaint: {
                            const context = getContext("2d")
                            context.clearRect(0, 0, width, height)
                            context.fillStyle = Theme.ivory
                            context.strokeStyle = Theme.ink
                            context.lineWidth = 6
                            context.lineJoin = "round"
                            context.beginPath()
                            context.moveTo(width * 0.08, height * 0.05)
                            context.bezierCurveTo(
                                width * 0.30, -2,
                                width * 0.82, height * 0.01,
                                width * 0.95, height * 0.12
                            )
                            context.bezierCurveTo(
                                width + 2, height * 0.40,
                                width * 0.98, height * 0.78,
                                width * 0.90, height * 0.95
                            )
                            context.bezierCurveTo(
                                width * 0.62, height + 2,
                                width * 0.19, height * 0.98,
                                width * 0.06, height * 0.87
                            )
                            context.bezierCurveTo(
                                -2, height * 0.57,
                                width * 0.01, height * 0.22,
                                width * 0.08, height * 0.05
                            )
                            context.closePath()
                            context.fill()
                            context.stroke()
                        }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 46
                        anchors.rightMargin: 46
                        anchors.topMargin: 42
                        anchors.bottomMargin: 40
                        spacing: 16

                        Text {
                            text: "WELCOME BACK"
                            color: Theme.muted
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                            font.letterSpacing: 1.2
                        }
                        Text {
                            text: "Weyriva"
                            color: Theme.ink
                            font.pixelSize: 39
                            font.bold: true
                            font.letterSpacing: -0.9
                        }
                        Text {
                            text: "Your desktop is covered and waiting."
                            color: Theme.muted
                            font.pixelSize: 14
                        }
                        Item { Layout.preferredHeight: 5 }
                        TextField {
                            id: password
                            Layout.fillWidth: true
                            implicitHeight: 52
                            echoMode: TextInput.Password
                            placeholderText: "Password"
                            color: Theme.ink
                            placeholderTextColor: Theme.muted
                            leftPadding: 18
                            rightPadding: 18
                            enabled: !auth.active
                            Component.onCompleted: forceActiveFocus()
                            background: Rectangle {
                                color: Theme.paper
                                radius: 17
                                border.width: password.activeFocus ? 3 : 1
                                border.color: Theme.ink
                            }
                            onAccepted: unlock()
                            function unlock() {
                                if (auth.active)
                                    return
                                auth.pendingResponse = text
                                auth.start()
                            }
                        }
                        ActionButton {
                            Layout.fillWidth: true
                            text: auth.active ? "Authenticating…" : "Unlock"
                            enabled: !auth.active
                            onClicked: password.unlock()
                        }
                        Text {
                            Layout.fillWidth: true
                            text: auth.message
                            color: auth.messageIsError ? Theme.clay : Theme.muted
                            wrapMode: Text.Wrap
                            font.pixelSize: 12
                        }
                    }
                }
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
            password.text = ""
            if (result === PamResult.Success) {
                sessionLock.locked = false
                ShellState.route = ""
            }
        }
    }

    IpcHandler {
        target: "weyriva"
        function route(name: string): void { ShellState.toggleRoute(name) }
        function lock(): void { ShellState.requestLock() }
        function clearNotifications(): void {
            const values = notifications.trackedNotifications.values
            for (let index = values.length - 1; index >= 0; --index)
                values[index].dismiss()
        }
        function toggleDnd(): void {
            ShellState.doNotDisturb = !ShellState.doNotDisturb
        }
        function setDnd(enabled: bool): void { ShellState.doNotDisturb = enabled }
        function toggleBar(): void { ShellState.barVisible = !ShellState.barVisible }
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
