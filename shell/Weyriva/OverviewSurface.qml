pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io

Item {
    id: root

    required property bool active
    property var windows: []
    property string error: ""
    property bool loading: false
    property string focusingId: ""

    function responseResult(text) {
        const envelope = JSON.parse(text)
        if (envelope.error)
            throw new Error(envelope.error.message || "Window query failed")
        if (!Array.isArray(envelope.result))
            throw new Error("Window query returned an invalid result")
        return envelope.result
    }

    function refresh() {
        if (queryProcess.running)
            return
        loading = true
        error = ""
        queryProcess.command = [
            "weyriva", "ipc", "call", "weyriva.niri.windows"
        ]
        queryProcess.running = true
    }

    function focusWindow(window) {
        const id = Number(window.id)
        if (!Number.isInteger(id) || id < 0 || focusProcess.running)
            return
        focusingId = String(id)
        error = ""
        focusProcess.command = [
            "niri", "msg", "action", "focus-window", "--id", String(id)
        ]
        focusProcess.running = true
    }

    function activateSurface() {
        refresh()
        Qt.callLater(function() { windowList.forceActiveFocus() })
    }

    onActiveChanged: {
        if (active && visible)
            activateSurface()
    }
    onVisibleChanged: {
        if (visible && active)
            activateSurface()
    }

    Process {
        id: queryProcess
        stdout: StdioCollector { id: queryOutput }
        stderr: StdioCollector { id: queryError }
        onRunningChanged: {
            if (running)
                return
            root.loading = false
            if (queryOutput.text.trim().length === 0) {
                root.windows = []
                root.error = queryError.text.trim() || "Window service unavailable"
                return
            }
            try {
                root.windows = root.responseResult(queryOutput.text).sort(
                    (left, right) => Number(right.is_focused)
                        - Number(left.is_focused)
                        || Number(left.workspace_id || 0)
                            - Number(right.workspace_id || 0)
                )
                root.error = ""
            } catch (requestError) {
                root.windows = []
                root.error = requestError.message
            }
        }
    }

    Process {
        id: focusProcess
        stderr: StdioCollector { id: focusError }
        onRunningChanged: {
            if (running)
                return
            root.focusingId = ""
            if (focusError.text.trim().length === 0) {
                ShellState.closeRoute()
                return
            }
            root.error = focusError.text.trim() || "Could not focus that window"
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: root.loading ? "READING NIRI…"
                    : root.windows.length + " OPEN WINDOWS"
                color: Theme.muted
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
            ActionButton {
                glyph: "↻"
                text: "Refresh windows"
                compact: true
                enabled: !root.loading
                onClicked: root.refresh()
            }
        }

        ListView {
            id: windowList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: count > 0
            clip: true
            spacing: 4
            model: root.windows
            currentIndex: count > 0 ? 0 : -1
            Keys.onDownPressed: currentIndex = Math.min(currentIndex + 1, count - 1)
            Keys.onUpPressed: currentIndex = Math.max(currentIndex - 1, 0)
            Keys.onReturnPressed: {
                if (currentItem)
                    root.focusWindow(currentItem.modelData)
            }
            Keys.onEnterPressed: {
                if (currentItem)
                    root.focusWindow(currentItem.modelData)
            }
            Keys.onEscapePressed: ShellState.closeRoute()

            delegate: Button {
                id: row
                required property var modelData
                width: ListView.view.width
                implicitHeight: 62
                enabled: root.focusingId.length === 0
                onClicked: root.focusWindow(modelData)

                contentItem: RowLayout {
                    spacing: 12
                    Rectangle {
                        Layout.preferredWidth: 34
                        Layout.preferredHeight: 34
                        radius: 17
                        color: row.modelData.is_focused
                            ? Theme.selection : Theme.surfaceAlt
                        Text {
                            anchors.centerIn: parent
                            text: String(row.modelData.workspace_id || "·")
                            color: Theme.foreground
                            font.pixelSize: 12
                            font.weight: Font.Bold
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.title || "Untitled window"
                            color: Theme.foreground
                            elide: Text.ElideRight
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            text: root.focusingId === String(row.modelData.id)
                                ? "FOCUSING…" : row.modelData.app_id || "Application"
                            color: Theme.muted
                            elide: Text.ElideRight
                            font.pixelSize: 10
                            font.letterSpacing: 0.5
                        }
                    }
                    Text {
                        text: row.modelData.is_focused ? "CURRENT" : "↗"
                        color: row.modelData.is_focused
                            ? Theme.accent : Theme.muted
                        font.pixelSize: 10
                        font.weight: Font.Bold
                    }
                }

                background: Rectangle {
                    color: row.down ? Theme.pressed
                        : ListView.isCurrentItem ? Theme.hover
                        : row.hovered ? Theme.hover : "transparent"
                    radius: Theme.radiusSmall
                    border.width: row.activeFocus ? 2 : 1
                    border.color: row.activeFocus
                        ? Theme.focusRing : Theme.separator
                }
            }
        }

        Column {
            Layout.alignment: Qt.AlignCenter
            visible: windowList.count === 0
            spacing: 8
            BrandMark {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 92
                height: 68
                quiet: true
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.loading ? "Looking for windows…"
                    : root.error.length > 0 ? "Overview unavailable"
                    : "No open windows"
                color: Theme.foreground
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: root.error.length > 0
                width: 360
                text: root.error
                color: Theme.muted
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                font.pixelSize: 11
            }
        }
    }
}
