import QtQuick
import QtQuick.Controls

Button {
    id: control
    required property string imageSource
    required property bool darkAppearance
    property bool selected: ShellState.wallpaper === imageSource

    implicitHeight: 220
    scale: down && !ShellState.reducedMotion ? 0.975 : 1

    Behavior on scale {
        enabled: !ShellState.reducedMotion
        NumberAnimation { duration: Theme.motionFast; easing.type: Easing.OutCubic }
    }

    onClicked: ShellState.useWallpaper(imageSource, darkAppearance)

    contentItem: Item {
        Image {
            anchors.fill: parent
            anchors.margins: 6
            source: control.imageSource
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            clip: true
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 12
            height: 42
            radius: 15
            color: control.darkAppearance ? Theme.ink : Theme.ivory

            Text {
                anchors.centerIn: parent
                text: control.text
                color: control.darkAppearance ? Theme.ivory : Theme.ink
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
        }
    }

    background: Rectangle {
        color: Theme.paper
        radius: 28
        border.width: control.activeFocus || control.selected ? 4 : 1
        border.color: control.selected ? Theme.ink : Theme.separator
    }
}
