import QtQuick

Item {
    id: root
    required property var modelData
    readonly property bool fromUser: modelData.role === "user"
    implicitHeight: message.height + 10

    Item {
        id: message
        anchors.right: root.fromUser ? parent.right : undefined
        anchors.left: root.fromUser ? undefined : parent.left
        width: parent.width * (root.fromUser ? 0.78 : 0.94)
        height: copy.implicitHeight + 46

        ContinuousSurface {
            anchors.fill: parent
            visible: root.fromUser
            fillColor: Theme.selection
            cornerRadius: 21
        }
        Rectangle {
            visible: !root.fromUser
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            width: 4
            radius: 2
            color: Theme.seaGlass
        }
        Text {
            id: roleLabel
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.leftMargin: root.fromUser ? 17 : 16
            anchors.topMargin: 12
            text: root.fromUser ? "YOU" : root.modelData.role.toUpperCase()
            color: root.fromUser ? Theme.onSelection : Theme.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1
        }
        Text {
            id: copy
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: roleLabel.bottom
            anchors.leftMargin: root.fromUser ? 17 : 16
            anchors.rightMargin: 17
            anchors.topMargin: 7
            text: root.modelData.text
            color: root.fromUser ? Theme.onSelection : Theme.foreground
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            font.pixelSize: 13
            lineHeight: 1.35
        }
    }
}
