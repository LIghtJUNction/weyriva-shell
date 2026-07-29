pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Greetd

ShellRoot {
    FloatingWindow {
        id: window
        visible: true
        fullscreen: true
        title: "Weyriva Greeter"
        color: "#BCD1CA"
        Component.onCompleted: username.forceActiveFocus()

        Canvas {
            visible: parent.width > 860
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Math.max(72, parent.width * 0.10)
            width: Math.min(300, parent.width * 0.27)
            height: width * 0.76
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const context = getContext("2d")
                context.clearRect(0, 0, width, height)
                context.strokeStyle = "#141413"
                context.fillStyle = "#141413"
                context.lineWidth = Math.max(10, width * 0.042)
                context.lineCap = "round"
                context.lineJoin = "round"

                context.fillStyle = "#FAF9F5"
                context.beginPath()
                context.moveTo(width * 0.13, height * 0.16)
                context.bezierCurveTo(
                    width * 0.31, height * 0.01,
                    width * 0.78, height * 0.07,
                    width * 0.89, height * 0.28
                )
                context.bezierCurveTo(
                    width * 0.98, height * 0.56,
                    width * 0.77, height * 0.91,
                    width * 0.47, height * 0.94
                )
                context.bezierCurveTo(
                    width * 0.20, height * 0.98,
                    width * 0.03, height * 0.72,
                    width * 0.09, height * 0.45
                )
                context.closePath()
                context.fill()

                context.fillStyle = "#141413"
                context.beginPath()
                context.moveTo(width * 0.10, height * 0.72)
                context.bezierCurveTo(
                    width * 0.27, height * 0.12,
                    width * 0.55, height * 0.84,
                    width * 0.86, height * 0.24
                )
                context.stroke()

                context.beginPath()
                context.moveTo(width * 0.20, height * 0.84)
                context.bezierCurveTo(
                    width * 0.40, height * 0.48,
                    width * 0.63, height * 0.74,
                    width * 0.91, height * 0.58
                )
                context.stroke()

                const dot = Math.max(8, width * 0.03)
                for (const point of [[0.16, 0.31], [0.84, 0.18], [0.88, 0.76]]) {
                    context.beginPath()
                    context.arc(
                        width * point[0], height * point[1],
                        dot, 0, Math.PI * 2
                    )
                    context.fill()
                }
            }
        }

        Rectangle {
            id: credentialRegion
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: parent.width > 860
                ? Math.min(230, parent.width * 0.17) : 0
            width: Math.min(420, parent.width - 48)
            height: 360
            color: "#FAF9F5"
            radius: 18
            border.width: 1
            border.color: "#14141333"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 32
                spacing: 9

                Text {
                    text: "Weyriva"
                    color: "#141413"
                    font.pixelSize: 34
                    font.weight: Font.Bold
                    font.letterSpacing: -0.7
                }

                Item { Layout.preferredHeight: 4 }

                Text {
                    text: "Username"
                    color: "#141413"
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }

                TextField {
                    id: username
                    Layout.fillWidth: true
                    implicitHeight: 46
                    color: "#141413"
                    leftPadding: 14
                    rightPadding: 14
                    enabled: Greetd.state === GreetdState.Inactive
                    onAccepted: password.forceActiveFocus()
                    background: Rectangle {
                        color: "#F0EEE6"
                        radius: 10
                        border.width: username.activeFocus ? 2 : 1
                        border.color: username.activeFocus
                            ? "#141413" : "#14141333"
                    }
                }

                Text {
                    text: "Password"
                    color: "#141413"
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }

                TextField {
                    id: password
                    Layout.fillWidth: true
                    implicitHeight: 46
                    echoMode: TextInput.Password
                    color: "#141413"
                    leftPadding: 14
                    rightPadding: 14
                    enabled: Greetd.state === GreetdState.Inactive
                    onAccepted: submit()

                    function submit() {
                        errorText.text = ""
                        if (Greetd.state === GreetdState.Inactive)
                            Greetd.createSession(username.text)
                    }

                    background: Rectangle {
                        color: "#F0EEE6"
                        radius: 10
                        border.width: password.activeFocus ? 2 : 1
                        border.color: password.activeFocus
                            ? "#141413" : "#14141333"
                    }
                }

                Button {
                    id: submitButton
                    Layout.fillWidth: true
                    implicitHeight: 44
                    text: Greetd.state === GreetdState.Inactive
                        ? "Sign in" : "Authenticating…"
                    enabled: Greetd.available
                        && Greetd.state === GreetdState.Inactive
                        && username.text.length > 0
                    scale: down ? 0.975 : 1
                    onClicked: password.submit()

                    Behavior on scale {
                        NumberAnimation {
                            duration: 105
                            easing.type: Easing.OutCubic
                        }
                    }

                    contentItem: Text {
                        text: submitButton.text
                        color: "#141413"
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                    }

                    background: Rectangle {
                        color: submitButton.down ? "#D97757"
                            : submitButton.hovered ? "#AFC5BE" : "#BCD1CA"
                        radius: 10
                        border.width: submitButton.activeFocus ? 2 : 1
                        border.color: "#141413"
                    }
                }

                Text {
                    id: errorText
                    Layout.fillWidth: true
                    Layout.minimumHeight: 28
                    wrapMode: Text.Wrap
                    color: "#D97757"
                    font.pixelSize: 12
                }
            }
        }
    }

    Connections {
        target: Greetd
        function onAuthMessage(message, error, responseRequired, echoResponse) {
            errorText.text = error ? message : ""
            if (responseRequired)
                Greetd.respond(password.text)
        }
        function onAuthFailure(message) {
            errorText.text = message
            password.clear()
            password.forceActiveFocus()
        }
        function onError(error) {
            errorText.text = error
            password.clear()
            password.forceActiveFocus()
        }
        function onReadyToLaunch() {
            password.clear()
            Greetd.launch(["/usr/bin/weyriva", "session", "start"])
        }
    }
}
