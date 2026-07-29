pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Greetd

ShellRoot {
    PanelWindow {
        id: window
        anchors { top: true; left: true; right: true; bottom: true }
        color: "#BCD1CA"
        focusable: true
        Component.onCompleted: username.forceActiveFocus()

        Canvas {
            visible: parent.width > 900
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Math.max(76, parent.width * 0.09)
            width: Math.min(360, parent.width * 0.30)
            height: width * 0.78
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const context = getContext("2d")
                context.clearRect(0, 0, width, height)
                context.strokeStyle = "#141413"
                context.fillStyle = "#141413"
                context.lineWidth = Math.max(11, width * 0.042)
                context.lineCap = "round"
                context.lineJoin = "round"

                context.beginPath()
                context.moveTo(width * 0.12, height * 0.68)
                context.bezierCurveTo(
                    width * 0.26, height * 0.12,
                    width * 0.56, height * 0.87,
                    width * 0.84, height * 0.25
                )
                context.stroke()

                context.beginPath()
                context.moveTo(width * 0.20, height * 0.82)
                context.bezierCurveTo(
                    width * 0.39, height * 0.48,
                    width * 0.61, height * 0.76,
                    width * 0.90, height * 0.60
                )
                context.stroke()

                const dot = Math.max(9, width * 0.03)
                for (const point of [[0.17, 0.32], [0.83, 0.18], [0.87, 0.77]]) {
                    context.beginPath()
                    context.arc(
                        width * point[0],
                        height * point[1],
                        dot,
                        0,
                        Math.PI * 2
                    )
                    context.fill()
                }
            }
        }

        Item {
            id: authCard
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: parent.width > 900
                ? Math.min(270, parent.width * 0.19) : 0
            width: Math.min(500, parent.width - 56)
            height: 438

            Canvas {
                anchors.fill: parent
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onPaint: {
                    const context = getContext("2d")
                    context.clearRect(0, 0, width, height)
                    context.fillStyle = "#FAF9F5"
                    context.strokeStyle = "#141413"
                    context.lineWidth = 6
                    context.lineJoin = "round"
                    context.beginPath()
                    context.moveTo(width * 0.08, height * 0.05)
                    context.bezierCurveTo(
                        width * 0.30, -2,
                        width * 0.79, height * 0.01,
                        width * 0.95, height * 0.12
                    )
                    context.bezierCurveTo(
                        width + 2, height * 0.38,
                        width * 0.98, height * 0.80,
                        width * 0.89, height * 0.95
                    )
                    context.bezierCurveTo(
                        width * 0.60, height + 2,
                        width * 0.18, height * 0.98,
                        width * 0.06, height * 0.87
                    )
                    context.bezierCurveTo(
                        -2, height * 0.56,
                        width * 0.01, height * 0.20,
                        width * 0.08, height * 0.05
                    )
                    context.closePath()
                    context.fill()
                    context.stroke()
                }
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 50
                anchors.rightMargin: 50
                anchors.topMargin: 44
                anchors.bottomMargin: 42
                spacing: 14

                Text {
                    text: "A QUIET PLACE TO BEGIN"
                    color: "#686761"
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1.15
                }
                Text {
                    text: "Weyriva"
                    color: "#141413"
                    font.pixelSize: 43
                    font.bold: true
                    font.letterSpacing: -1
                }
                Text {
                    text: "Sign in to your Niri desktop."
                    color: "#686761"
                    font.pixelSize: 14
                }
                Item { Layout.preferredHeight: 5 }
                TextField {
                    id: username
                    Layout.fillWidth: true
                    implicitHeight: 50
                    placeholderText: "Username"
                    color: "#141413"
                    placeholderTextColor: "#686761"
                    leftPadding: 18
                    rightPadding: 18
                    enabled: Greetd.state === GreetdState.Inactive
                    onAccepted: password.forceActiveFocus()
                    background: Rectangle {
                        color: "#F0EEE6"
                        radius: 17
                        border.width: username.activeFocus ? 3 : 1
                        border.color: "#141413"
                    }
                }
                TextField {
                    id: password
                    Layout.fillWidth: true
                    implicitHeight: 50
                    placeholderText: "Password"
                    echoMode: TextInput.Password
                    color: "#141413"
                    placeholderTextColor: "#686761"
                    leftPadding: 18
                    rightPadding: 18
                    enabled: Greetd.state === GreetdState.Inactive
                    background: Rectangle {
                        color: "#F0EEE6"
                        radius: 17
                        border.width: password.activeFocus ? 3 : 1
                        border.color: "#141413"
                    }
                    onAccepted: submit()
                    function submit() {
                        errorText.text = ""
                        if (Greetd.state === GreetdState.Inactive)
                            Greetd.createSession(username.text)
                    }
                }
                Button {
                    id: submitButton
                    Layout.fillWidth: true
                    implicitHeight: 50
                    text: Greetd.state === GreetdState.Inactive
                        ? "Sign in" : "Authenticating…"
                    enabled: Greetd.available
                        && Greetd.state === GreetdState.Inactive
                        && username.text.length > 0
                    onClicked: password.submit()
                    scale: down ? 0.965 : 1

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
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                    }
                    background: Rectangle {
                        color: submitButton.down ? "#D97757"
                            : submitButton.hovered ? "#AFC5BE" : "#BCD1CA"
                        radius: 18
                        border.width: submitButton.activeFocus ? 3 : 1
                        border.color: "#141413"
                    }
                }
                Text {
                    id: errorText
                    Layout.fillWidth: true
                    Layout.minimumHeight: 34
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
        function onError(error) { errorText.text = error }
        function onReadyToLaunch() {
            Greetd.launch(["/usr/bin/weyriva", "session", "start"])
        }
    }
}
