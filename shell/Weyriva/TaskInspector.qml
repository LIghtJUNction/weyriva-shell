pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root
    required property var bridge

    function openCommand(command) {
        if (!root.bridge.validName(root.bridge.selectedAgent)
                || !root.bridge.validName(root.bridge.sessionDraft))
            return
        Quickshell.execDetached([
            "foot", "--title", "CortexFS · " + root.bridge.selectedAgent,
            "--", "ctx", "agent", command, root.bridge.selectedAgent,
            "--session", root.bridge.sessionDraft
        ])
    }

    ContinuousSurface {
        anchors.fill: parent
        fillColor: Theme.inspectorMaterial
        cornerRadius: 20
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 10

        Text {
            text: "RUN CONTEXT"
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1.1
        }
        Text {
            Layout.fillWidth: true
            text: root.bridge.selectedAgent || "No agent selected"
            color: Theme.foreground
            elide: Text.ElideRight
            font.pixelSize: 21
            font.weight: Font.DemiBold
            font.letterSpacing: -0.5
        }
        Text {
            Layout.fillWidth: true
            text: [
                root.bridge.agentDetail.model || "",
                root.bridge.agentDetail.role || "",
                root.bridge.agentDetail.life || ""
            ].filter(value => value.length > 0).join(" · ")
            color: Theme.muted
            elide: Text.ElideRight
            font.pixelSize: 11
        }
        Text {
            Layout.fillWidth: true
            text: root.bridge.agentDetail.cwd || ""
            color: Theme.muted
            elide: Text.ElideMiddle
            font.pixelSize: 10
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 6
            Text {
                Layout.fillWidth: true
                text: "TOOLS"
                color: Theme.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            Text {
                text: String(root.bridge.tools.length)
                color: Theme.foreground
                font.pixelSize: 11
                font.weight: Font.DemiBold
            }
        }
        ListView {
            Layout.fillWidth: true
            Layout.preferredHeight: 176
            clip: true
            spacing: 1
            model: root.bridge.tools
            delegate: Item {
                required property var modelData
                width: ListView.view.width
                height: 28
                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: 5
                    height: 5
                    radius: 3
                    color: Theme.accent
                }
                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 13
                    text: parent.modelData.name
                    color: Theme.foreground
                    elide: Text.ElideRight
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
        }

        Text {
            text: "ACTIVITY"
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1.1
        }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 6
            model: root.bridge.activity
            delegate: Text {
                required property string modelData
                width: ListView.view.width
                text: "•  " + modelData
                color: Theme.muted
                wrapMode: Text.Wrap
                font.pixelSize: 10
                lineHeight: 1.25
            }
        }

        Text {
            Layout.fillWidth: true
            visible: root.bridge.error.length > 0
            text: root.bridge.error
            color: Theme.accentWarm
            wrapMode: Text.Wrap
            font.pixelSize: 10
        }
        Text {
            Layout.fillWidth: true
            visible: root.bridge.inputTokens + root.bridge.outputTokens > 0
            text: root.bridge.inputTokens + " input · "
                + root.bridge.outputTokens + " output tokens"
            color: Theme.muted
            font.pixelSize: 10
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            ActionButton {
                Layout.fillWidth: true
                text: "Chat"
                enabled: root.bridge.selectedAgentReachable
                onClicked: root.openCommand("chat")
            }
            ActionButton {
                Layout.fillWidth: true
                text: "Attach"
                enabled: root.bridge.selectedAgentReachable
                onClicked: root.openCommand("attach")
            }
        }
        ActionButton {
            Layout.fillWidth: true
            text: "Reload session history"
            enabled: root.bridge.selectedSession.length > 0
                && !root.bridge.running
            onClicked: root.bridge.loadHistory()
        }
    }
}
