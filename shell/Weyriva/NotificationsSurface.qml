pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property var notificationServer

    function dismissAllNotifications() {
        const values = notificationServer.trackedNotifications.values
        for (let index = values.length - 1; index >= 0; --index)
            values[index].dismiss()
    }

    ColumnLayout {
        anchors.fill: parent
        visible: root.notificationServer.trackedNotifications.values.length > 0
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.bottomMargin: 6

            Text {
                Layout.fillWidth: true
                text: root.notificationServer.trackedNotifications.values.length
                    + " recent"
                color: Theme.muted
                font.pixelSize: 11
                font.weight: Font.DemiBold
            }

            ActionButton {
                text: "Clear all"
                onClicked: root.dismissAllNotifications()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: Theme.separator
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 0
            model: root.notificationServer.trackedNotifications

            delegate: Item {
                required property var modelData

                width: ListView.view.width
                height: 70

                Column {
                    anchors.left: parent.left
                    anchors.right: dismissButton.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 2
                    anchors.rightMargin: 10
                    spacing: 3

                    Text {
                        width: parent.width
                        text: modelData.summary || modelData.appName
                        color: Theme.foreground
                        elide: Text.ElideRight
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        width: parent.width
                        text: modelData.body
                        textFormat: Text.PlainText
                        color: Theme.muted
                        maximumLineCount: 2
                        wrapMode: Text.Wrap
                        elide: Text.ElideRight
                        font.pixelSize: 11
                    }
                }

                ActionButton {
                    id: dismissButton
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    glyph: "×"
                    text: "Dismiss"
                    compact: true
                    onClicked: modelData.dismiss()
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Theme.separator
                }
            }
        }
    }

    Column {
        anchors.centerIn: parent
        visible: root.notificationServer.trackedNotifications.values.length === 0
        spacing: 8

        BrandMark {
            anchors.horizontalCenter: parent.horizontalCenter
            width: 92
            height: 68
            quiet: true
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "No notifications"
            color: Theme.foreground
            font.pixelSize: 14
            font.weight: Font.DemiBold
        }
    }
}
