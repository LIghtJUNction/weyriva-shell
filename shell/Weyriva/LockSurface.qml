import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var authContext

    function clearPassword() {
        password.text = ""
    }

    function focusPassword() {
        password.forceActiveFocus()
    }

    BrandMark {
        visible: parent.width > 860
        width: 270
        height: 210
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Math.max(72, parent.width * 0.11)
    }

    Rectangle {
        id: credentialRegion

        anchors.centerIn: parent
        anchors.horizontalCenterOffset: parent.width > 860
            ? Math.min(230, parent.width * 0.17) : 0
        width: Math.min(420, parent.width - 48)
        height: 286
        color: Theme.surface
        radius: 18
        border.width: 1
        border.color: Theme.separator

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 32
            spacing: 10

            Text {
                text: "Weyriva"
                color: Theme.foreground
                font.pixelSize: 34
                font.weight: Font.Bold
                font.letterSpacing: -0.7
            }

            Item { Layout.preferredHeight: 4 }

            Text {
                text: "Password"
                color: Theme.foreground
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }

            TextField {
                id: password

                Layout.fillWidth: true
                implicitHeight: 48
                echoMode: TextInput.Password
                color: Theme.foreground
                placeholderTextColor: Theme.muted
                leftPadding: 14
                rightPadding: 14
                enabled: !root.authContext.active
                Component.onCompleted: forceActiveFocus()
                onAccepted: submit()

                function submit() {
                    if (root.authContext.active)
                        return
                    root.authContext.pendingResponse = text
                    root.authContext.start()
                }

                background: Rectangle {
                    color: Theme.surfaceAlt
                    radius: 10
                    border.width: password.activeFocus ? 2 : 1
                    border.color: password.activeFocus
                        ? Theme.foreground : Theme.separator
                }
            }

            ActionButton {
                Layout.fillWidth: true
                text: root.authContext.active ? "Authenticating…" : "Unlock"
                enabled: !root.authContext.active
                onClicked: password.submit()
            }

            Text {
                Layout.fillWidth: true
                Layout.minimumHeight: 28
                text: root.authContext.message
                color: root.authContext.messageIsError
                    ? Theme.clay : Theme.muted
                wrapMode: Text.Wrap
                font.pixelSize: 12
            }
        }
    }
}
