pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Greetd
ShellRoot {
    id: root
    property date now: new Date()
    Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: root.now = new Date()
    }
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
            anchors.leftMargin: Math.max(64, parent.width * 0.08)
            width: Math.min(360, parent.width * 0.31)
            height: width * 0.78
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const context = getContext("2d")
                context.clearRect(0, 0, width, height)
                context.fillStyle = "#FAF9F5"
                context.beginPath()
                context.moveTo(width * 0.12, height * 0.23)
                context.bezierCurveTo(width * 0.24, height * 0.02,
                    width * 0.64, height * 0.02, width * 0.83, height * 0.17)
                context.bezierCurveTo(width * 1.02, height * 0.33,
                    width * 0.94, height * 0.72, width * 0.70, height * 0.88)
                context.bezierCurveTo(width * 0.47, height * 1.02,
                    width * 0.14, height * 0.91, width * 0.07, height * 0.63)
                context.bezierCurveTo(width * 0.02, height * 0.46,
                    width * 0.03, height * 0.33, width * 0.12, height * 0.23)
                context.closePath()
                context.fill()
                context.strokeStyle = "#141413"
                context.fillStyle = "#141413"
                context.lineWidth = Math.max(11, width * 0.047)
                context.lineCap = "round"
                context.lineJoin = "round"
                context.beginPath()
                context.moveTo(width * 0.15, height * 0.70)
                context.bezierCurveTo(width * 0.31, height * 0.20,
                    width * 0.53, height * 0.81, width * 0.84, height * 0.27)
                context.stroke()
                context.beginPath()
                context.moveTo(width * 0.23, height * 0.82)
                context.bezierCurveTo(width * 0.42, height * 0.49,
                    width * 0.65, height * 0.76, width * 0.89, height * 0.57)
                context.stroke()
                for (const point of [[0.16, 0.30], [0.84, 0.19], [0.88, 0.76]]) {
                    context.beginPath()
                    context.arc(
                        width * point[0], height * point[1],
                        Math.max(8, width * 0.03), 0, Math.PI * 2
                    )
                    context.fill()
                }
            }
        }
        Rectangle {
            id: credentialRegion
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: parent.width > 860
                ? Math.min(250, parent.width * 0.18) : 0
            width: Math.min(420, parent.width - 48)
            height: 396
            color: "#FAF9F5"
            radius: 20
            border.width: 1
            border.color: "#14141333"
            Rectangle {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: 22
                anchors.rightMargin: 22
                height: 4
                radius: 2
                color: "#D97757"
            }
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 30
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Text {
                            text: "WEYRIVA"
                            color: "#141413"
                            font.pixelSize: 25
                            font.weight: Font.Bold
                            font.letterSpacing: -0.5
                        }
                        Text {
                            text: Qt.formatDate(root.now, "dddd, MMMM d")
                            color: "#686761"
                            font.pixelSize: 11
                        }
                    }
                    Text {
                        text: Qt.formatTime(root.now, "HH:mm")
                        color: "#141413"
                        font.pixelSize: 27
                        font.weight: Font.DemiBold
                        Accessible.name: "Current time " + text
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: !Greetd.available ? "LOGIN SERVICE UNAVAILABLE"
                        : Greetd.state === GreetdState.Inactive
                            ? "READY TO SIGN IN" : "AUTHENTICATING"
                    color: !Greetd.available ? "#D97757" : "#686761"
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.15
                }
                Item { Layout.preferredHeight: 2 }
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
                    enabled: Greetd.state === GreetdState.Inactive
                    color: "#141413"
                    leftPadding: 14
                    rightPadding: 14
                    Accessible.name: "Username"
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
                    enabled: Greetd.state === GreetdState.Inactive
                    echoMode: TextInput.Password
                    color: "#141413"
                    leftPadding: 14
                    rightPadding: 14
                    Accessible.name: "Password"
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
                        border.color: errorText.text.length > 0
                            ? "#D97757"
                            : password.activeFocus ? "#141413" : "#14141333"
                    }
                }
                Button {
                    id: submitButton
                    Layout.fillWidth: true
                    implicitHeight: 44
                    enabled: Greetd.available
                        && Greetd.state === GreetdState.Inactive
                        && username.text.length > 0
                    text: Greetd.state === GreetdState.Inactive
                        ? "Sign in" : "Authenticating…"
                    onClicked: password.submit()
                    contentItem: Text {
                        text: submitButton.text
                        color: "#141413"
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 14
                        font.weight: Font.Bold
                    }
                    background: Rectangle {
                        color: submitButton.down ? "#D97757"
                            : submitButton.hovered ? "#E3DACC" : "#BCD1CA"
                        radius: 10
                        border.width: submitButton.activeFocus ? 2 : 1
                        border.color: "#141413"
                    }
                }
                Text {
                    id: errorText
                    Layout.fillWidth: true
                    Layout.minimumHeight: 26
                    text: ""
                    color: "#D97757"
                    wrapMode: Text.Wrap
                    font.pixelSize: 11
                    Accessible.name: text
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
