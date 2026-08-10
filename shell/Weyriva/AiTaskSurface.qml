import QtQuick
import QtQuick.Layouts

Item {
    id: root
    required property bool active

    function activateSurface() {
        bridge.refreshAgents()
        if (ShellState.taskDraft.length > 0) {
            workspace.promptText = ShellState.taskDraft
            ShellState.taskDraft = ""
        }
        Qt.callLater(workspace.focusPrompt)
    }

    onActiveChanged: {
        if (active && visible)
            activateSurface()
    }
    onVisibleChanged: {
        if (visible && active)
            activateSurface()
    }

    CortexBridge {
        id: bridge
        onCompleted: status => {
            if (status === "ok")
                ShellState.showToast("AI task completed", "info")
        }
    }

    Shortcut {
        sequence: "Ctrl+N"
        enabled: root.active && !bridge.running
        onActivated: {
            bridge.newSession()
            workspace.focusPrompt()
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 10

        TaskNavigation {
            Layout.preferredWidth: 250
            Layout.fillHeight: true
            bridge: bridge
        }
        TaskWorkspace {
            id: workspace
            Layout.fillWidth: true
            Layout.fillHeight: true
            bridge: bridge
        }
        TaskInspector {
            Layout.preferredWidth: 252
            Layout.fillHeight: true
            visible: root.width >= 1120
            bridge: bridge
        }
    }
}
