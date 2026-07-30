import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Button {
    id: control

    property string subtitle: ""
    property string value: ""
    property string glyph: ""
    property bool selected: false
    property bool danger: false

    implicitHeight: subtitle.length > 0 ? 54 : 44
    leftPadding: 10
    rightPadding: 10
    scale: down && !ShellState.reducedMotion ? 0.975 : 1
    opacity: enabled ? (down ? 0.78 : 1) : 0.42

    Behavior on scale {
        enabled: !control.down && !ShellState.reducedMotion
        NumberAnimation {
            duration: Theme.motionFast
            easing.type: Easing.OutCubic
        }
    }

    contentItem: RowLayout {
        spacing: 11

        Text {
            visible: control.glyph.length > 0
            Layout.preferredWidth: 22
            text: control.glyph
            color: control.danger ? Theme.clay : Theme.foreground
            font.pixelSize: 16
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
        }

        Column {
            Layout.fillWidth: true
            spacing: 2

            Text {
                width: parent.width
                text: control.text
                color: control.danger ? Theme.clay : Theme.foreground
                elide: Text.ElideRight
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: -0.1
            }

            Text {
                visible: control.subtitle.length > 0
                width: parent.width
                text: control.subtitle
                color: Theme.muted
                elide: Text.ElideRight
                font.pixelSize: 11
            }
        }

        Text {
            visible: control.value.length > 0
            text: control.value
            color: control.selected ? Theme.foreground : Theme.muted
            font.pixelSize: 12
            font.weight: Font.DemiBold
        }

        Rectangle {
            visible: control.selected
            Layout.preferredWidth: 3
            Layout.preferredHeight: 16
            radius: 2
            color: Theme.accent
        }
    }

    background: Rectangle {
        color: control.down ? Theme.pressed
            : control.hovered && control.enabled ? Theme.hover
            : "transparent"
        radius: Theme.radiusSmall
        border.width: control.activeFocus ? 2 : 0
        border.color: Theme.focusRing

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: 42
            anchors.rightMargin: 10
            height: 1
            color: Theme.separator
        }
    }
}
