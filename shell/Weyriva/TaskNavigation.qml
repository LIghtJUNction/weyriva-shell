pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    required property var bridge

    ContinuousSurface {
        anchors.fill: parent
        fillColor: Theme.navigationMaterial
        cornerRadius: 20
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 9
            Rectangle {
                implicitWidth: 10
                implicitHeight: 10
                radius: 5
                color: root.bridge.loading ? Theme.accentWarm
                    : root.bridge.available ? Theme.accent : Theme.muted
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text {
                    text: "CortexFS"
                    color: Theme.foreground
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    font.letterSpacing: -0.2
                }
                Text {
                    text: root.bridge.loading ? "Discovering runtime"
                        : root.bridge.agents.length + " agents available"
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
            ActionButton {
                glyph: "↻"
                text: "Refresh CortexFS"
                compact: true
                enabled: !root.bridge.loading && !root.bridge.running
                onClicked: root.bridge.refreshAgents()
            }
        }

        Text {
            text: "AGENTS"
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1.1
        }

        ListView {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.max(150, root.height * 0.36)
            clip: true
            spacing: 3
            model: root.bridge.agents
            delegate: Button {
                id: agentButton
                required property var modelData
                width: ListView.view.width
                height: 48
                leftPadding: 11 + modelData.depth * 13
                rightPadding: 10
                hoverEnabled: true
                enabled: !root.bridge.running
                scale: down && !ShellState.reducedMotion ? 0.975 : 1
                onClicked: root.bridge.selectAgent(modelData.name)
                Behavior on scale {
                    enabled: !agentButton.down && !ShellState.reducedMotion
                    SpringAnimation {
                        spring: 5
                        damping: 0.9
                        epsilon: 0.001
                    }
                }
                contentItem: RowLayout {
                    spacing: 8
                    Rectangle {
                        implicitWidth: 7
                        implicitHeight: 7
                        radius: 4
                        color: agentButton.modelData.reachable
                            ? Theme.accent : Theme.muted
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Text {
                            Layout.fillWidth: true
                            text: agentButton.modelData.name
                            color: root.bridge.selectedAgent
                                    === agentButton.modelData.name
                                ? Theme.onSelection : Theme.foreground
                            elide: Text.ElideRight
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            text: agentButton.modelData.model.length > 0
                                ? agentButton.modelData.model
                                : agentButton.modelData.status
                            color: root.bridge.selectedAgent
                                    === agentButton.modelData.name
                                ? Theme.onSelection : Theme.muted
                            opacity: 0.78
                            elide: Text.ElideRight
                            font.pixelSize: 10
                        }
                    }
                }
                background: ContinuousSurface {
                    fillColor: root.bridge.selectedAgent
                            === agentButton.modelData.name
                        ? Theme.selection
                        : agentButton.hovered ? Theme.hover : "transparent"
                    cornerRadius: 15
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: "SESSIONS · " + root.bridge.sessions.length
                color: Theme.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            ActionButton {
                glyph: "+"
                text: "New session"
                compact: true
                enabled: root.bridge.selectedAgent.length > 0
                    && !root.bridge.running
                onClicked: root.bridge.newSession()
            }
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 2
            model: root.bridge.sessions
            delegate: Button {
                id: sessionButton
                required property string modelData
                width: ListView.view.width
                height: 38
                leftPadding: 11
                rightPadding: 10
                hoverEnabled: true
                enabled: !root.bridge.running
                onClicked: root.bridge.selectSession(modelData)
                contentItem: Text {
                    text: sessionButton.modelData
                    color: root.bridge.selectedSession
                            === sessionButton.modelData
                        ? Theme.onSelection : Theme.foreground
                    elide: Text.ElideMiddle
                    verticalAlignment: Text.AlignVCenter
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }
                background: ContinuousSurface {
                    fillColor: root.bridge.selectedSession
                            === sessionButton.modelData
                        ? Theme.selection
                        : sessionButton.hovered ? Theme.hover : "transparent"
                    cornerRadius: 13
                }
            }
            Text {
                anchors.centerIn: parent
                visible: parent.count === 0
                    && !root.bridge.loading
                    && root.bridge.selectedAgent.length > 0
                width: parent.width - 24
                text: "No indexed sessions yet"
                color: Theme.muted
                horizontalAlignment: Text.AlignHCenter
                font.pixelSize: 11
            }
        }
    }
}
