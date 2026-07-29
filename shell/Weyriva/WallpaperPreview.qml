import QtQuick
import QtQuick.Controls

Button {
    id: control
    required property string imageSource
    required property bool darkAppearance
    property bool selected: ShellState.wallpaper === imageSource

    implicitHeight: 210
    scale: down && !ShellState.reducedMotion ? 0.985 : 1

    Behavior on scale {
        enabled: !ShellState.reducedMotion
        NumberAnimation { duration: Theme.motionFast; easing.type: Easing.OutCubic }
    }

    onClicked: ShellState.useWallpaper(imageSource, darkAppearance)

    contentItem: Item {
        Image {
            anchors.fill: parent
            anchors.margins: 3
            source: control.imageSource
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            clip: true
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: 3
            anchors.rightMargin: 3
            anchors.bottomMargin: 3
            height: 38
            color: Theme.surface

            Text {
                anchors.centerIn: parent
                text: control.text
                color: Theme.foreground
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
        }
    }

    background: Rectangle {
        color: Theme.surfaceAlt
        radius: 12
        border.width: control.activeFocus || control.selected ? 2 : 1
        border.color: control.selected ? Theme.foreground : Theme.separator
    }
}
