pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    required property var bridge
    property alias promptText: prompt.text

    function focusPrompt() { prompt.forceActiveFocus() }
    function submit() {
        root.bridge.submit(prompt.text, root.bridge.sessionDraft)
        if (root.bridge.running)
            prompt.text = ""
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: "PRIVATE · DURABLE · CORTEXFS ABI"
                    color: Theme.accentWarm
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: root.bridge.selectedAgent.length > 0
                        ? "Build with " + root.bridge.selectedAgent
                        : "Choose a CortexFS agent"
                    color: Theme.foreground
                    elide: Text.ElideRight
                    font.pixelSize: 28
                    font.weight: Font.DemiBold
                    font.letterSpacing: -0.8
                }
            }
            ContinuousSurface {
                implicitWidth: sessionLabel.implicitWidth + 24
                implicitHeight: 34
                fillColor: Theme.surfaceAlt
                cornerRadius: 17
                Text {
                    id: sessionLabel
                    anchors.centerIn: parent
                    text: root.bridge.sessionStatus.toUpperCase()
                        + " · " + root.bridge.sessionDraft
                    color: Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            ListView {
                id: timeline
                anchors.fill: parent
                clip: true
                spacing: 5
                model: root.bridge.running && root.bridge.response.length > 0
                    ? root.bridge.messages.concat([{
                        role: "assistant",
                        text: root.bridge.response,
                        run: root.bridge.runId
                    }]) : root.bridge.messages
                delegate: TaskMessage { width: ListView.view.width }
                onCountChanged: Qt.callLater(positionViewAtEnd)
            }
            Column {
                anchors.centerIn: parent
                visible: timeline.count === 0
                width: Math.min(parent.width - 80, 460)
                spacing: 12
                ContinuousSurface {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 62
                    height: 62
                    fillColor: Theme.seaGlass
                    cornerRadius: 20
                    Rectangle {
                        anchors.centerIn: parent
                        width: 18
                        height: 18
                        radius: 9
                        color: Theme.foreground
                    }
                    Rectangle {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: 11
                        anchors.bottomMargin: 11
                        width: 9
                        height: 9
                        radius: 5
                        color: Theme.clay
                    }
                }
                Text {
                    width: parent.width
                    text: "A focused place for consequential work."
                    color: Theme.foreground
                    horizontalAlignment: Text.AlignHCenter
                    font.pixelSize: 21
                    font.weight: Font.DemiBold
                    font.letterSpacing: -0.5
                }
                Text {
                    width: parent.width
                    text: "Continue a durable session or launch a new task with any live agent."
                    color: Theme.muted
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    font.pixelSize: 12
                    lineHeight: 1.3
                }
            }
        }

        ContinuousSurface {
            Layout.fillWidth: true
            visible: root.bridge.pendingApproval !== null
            implicitHeight: approvalLayout.implicitHeight + 26
            fillColor: Theme.accentWarm
            cornerRadius: 20
            ContinuousSurface {
                anchors.fill: parent
                anchors.margins: 3
                fillColor: Theme.surfaceAlt
                cornerRadius: 17
            }
            RowLayout {
                id: approvalLayout
                anchors.fill: parent
                anchors.margins: 13
                spacing: 10
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: "TOOL APPROVAL"
                        color: Theme.accentWarm
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 1
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.bridge.pendingApproval
                            ? root.bridge.pendingApproval.name + " · "
                                + JSON.stringify(
                                    root.bridge.pendingApproval.args
                                ) : ""
                        color: Theme.foreground
                        elide: Text.ElideRight
                        font.pixelSize: 12
                    }
                }
                ActionButton {
                    text: "Deny"
                    onClicked: root.bridge.approve("deny")
                }
                ActionButton {
                    text: "Allow once"
                    emphasized: true
                    onClicked: root.bridge.approve("allow_once")
                }
            }
        }

        ContinuousSurface {
            Layout.fillWidth: true
            implicitHeight: 126
            fillColor: prompt.activeFocus ? Theme.focusRing : Theme.surfaceFrame
            cornerRadius: 27
            ContinuousSurface {
                anchors.fill: parent
                anchors.margins: prompt.activeFocus ? 2 : 3
                fillColor: Theme.surfaceRaised
                cornerRadius: prompt.activeFocus ? 25 : 24
            }
            TextArea {
                id: prompt
                anchors.left: parent.left
                anchors.right: actions.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: 17
                placeholderText: "Describe the outcome, constraints, and evidence you expect…"
                color: Theme.foreground
                placeholderTextColor: Theme.muted
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                enabled: root.bridge.selectedAgentReachable
                    && !root.bridge.running
                background: null
                font.pixelSize: 14
                Keys.onPressed: event => {
                    if ((event.modifiers & Qt.ControlModifier)
                            && (event.key === Qt.Key_Return
                                || event.key === Qt.Key_Enter)) {
                        root.submit()
                        event.accepted = true
                    }
                }
            }
            Column {
                id: actions
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                spacing: 7
                ActionButton {
                    glyph: "↗"
                    text: root.bridge.running ? "Running" : "Launch task"
                    emphasized: true
                    enabled: root.bridge.selectedAgentReachable
                        && !root.bridge.running
                    onClicked: root.submit()
                }
                ActionButton {
                    visible: root.bridge.running
                    text: "Cancel"
                    danger: true
                    onClicked: root.bridge.cancel()
                }
            }
        }
    }
}
