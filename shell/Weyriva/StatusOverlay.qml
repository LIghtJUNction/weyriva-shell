import QtQuick
import QtQuick.Layouts

Item {
    id: root

    property bool osdVisible: false
    property bool toastVisible: false

    Connections {
        target: ShellState
        function onOsdRevisionChanged() {
            root.osdVisible = true
            root.toastVisible = false
            osdTimer.restart()
        }
        function onToastRequested() {
            root.toastVisible = true
            root.osdVisible = false
            toastTimer.restart()
        }
    }

    Timer {
        id: osdTimer
        interval: 1400
        repeat: false
        onTriggered: root.osdVisible = false
    }

    Timer {
        id: toastTimer
        interval: 2200
        repeat: false
        onTriggered: root.toastVisible = false
    }

    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 72
        width: root.toastVisible ? Math.min(460, toastText.implicitWidth + 48) : 300
        height: root.toastVisible ? 52 : 78
        visible: root.osdVisible || root.toastVisible || opacity > 0
        opacity: root.osdVisible || root.toastVisible ? 1 : 0
        color: Theme.surface
        radius: Theme.radius
        border.width: 1
        border.color: ShellState.toastTone === "attention" && root.toastVisible
            ? Theme.accentWarm : Theme.separator

        Behavior on opacity {
            NumberAnimation {
                duration: ShellState.reducedMotion ? 0 : 130
                easing.type: Easing.OutCubic
            }
        }

        Text {
            id: toastText
            anchors.centerIn: parent
            visible: root.toastVisible
            text: ShellState.toastMessage
            color: Theme.foreground
            font.pixelSize: 13
            font.weight: Font.DemiBold
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            visible: root.osdVisible
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                Text {
                    text: ShellState.osdKind === "volume"
                        ? ShellState.osdMuted ? "×" : "◖" : "☀"
                    color: ShellState.osdMuted
                        ? Theme.accentWarm : Theme.foreground
                    font.pixelSize: 18
                    font.weight: Font.Bold
                }
                Text {
                    Layout.fillWidth: true
                    text: ShellState.osdLabel
                    color: Theme.foreground
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                }
                Text {
                    text: Math.round(ShellState.osdValue * 100) + "%"
                    color: Theme.muted
                    font.pixelSize: 12
                    font.weight: Font.Bold
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 7
                radius: 4
                color: Theme.surfaceAlt
                Rectangle {
                    width: parent.width * ShellState.osdValue
                    height: parent.height
                    radius: parent.radius
                    color: ShellState.osdMuted
                        ? Theme.accentWarm : Theme.accent
                }
            }
        }
    }
}
