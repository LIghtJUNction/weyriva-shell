pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import Quickshell.Io

Item {
    id: root

    required property var sourceScreen
    property var workspaces: []
    implicitWidth: 194
    implicitHeight: 36

    function refresh() {
        if (queryProcess.running)
            return
        queryProcess.command = [
            "weyriva", "ipc", "call", "weyriva.niri.workspaces"
        ]
        queryProcess.running = true
    }

    function activate(workspace) {
        const reference = workspace.name || String(workspace.idx)
        if (reference.length === 0 || focusProcess.running)
            return
        focusProcess.command = [
            "niri", "msg", "action", "focus-workspace", reference
        ]
        focusProcess.running = true
    }

    Component.onCompleted: refresh()

    Timer {
        interval: 2000
        repeat: true
        running: root.visible
        onTriggered: root.refresh()
    }

    Process {
        id: queryProcess
        stdout: StdioCollector { id: queryOutput }
        onRunningChanged: {
            if (running || queryOutput.text.trim().length === 0)
                return
            try {
                const envelope = JSON.parse(queryOutput.text)
                if (!Array.isArray(envelope.result))
                    return
                const screenName = root.sourceScreen ? root.sourceScreen.name : ""
                let values = envelope.result.filter(workspace =>
                    !screenName || workspace.output === screenName
                )
                if (values.length === 0)
                    values = envelope.result
                values.sort((left, right) => Number(left.idx) - Number(right.idx))
                root.workspaces = values
            } catch (error) {
                root.workspaces = []
            }
        }
    }

    Process {
        id: focusProcess
        onRunningChanged: {
            if (!running)
                root.refresh()
        }
    }

    Flickable {
        anchors.fill: parent
        contentWidth: workspaceRow.width
        contentHeight: height
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Row {
            id: workspaceRow
            height: parent.height
            spacing: 1
            Repeater {
                model: root.workspaces
                delegate: Button {
                    id: workspaceButton
                    required property var modelData
                    readonly property string label: modelData.name
                        || String(modelData.idx)
                    width: workspaceLabel.implicitWidth + 22
                    height: parent.height
                    hoverEnabled: true
                    scale: down && !ShellState.reducedMotion ? 0.965 : 1
                    Accessible.name: "Workspace " + label
                    onClicked: root.activate(modelData)
                    Behavior on scale {
                        enabled: !workspaceButton.down
                            && !ShellState.reducedMotion
                        SpringAnimation {
                            spring: 5
                            damping: 0.9
                            epsilon: 0.001
                        }
                    }
                    contentItem: Text {
                        id: workspaceLabel
                        text: workspaceButton.label
                        color: Boolean(workspaceButton.modelData.is_focused
                                || workspaceButton.modelData.is_active)
                            ? Theme.onSelection : Theme.chromeText
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                    background: ContinuousSurface {
                        fillColor: Boolean(workspaceButton.modelData.is_focused
                                || workspaceButton.modelData.is_active)
                            ? Theme.selection
                            : workspaceButton.hovered ? Theme.hover : "transparent"
                        cornerRadius: height / 2
                    }
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.workspaces.length === 0
                text: "NO WORKSPACES"
                color: Theme.chromeMuted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }
    }
}
