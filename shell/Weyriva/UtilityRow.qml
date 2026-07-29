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

    implicitHeight: subtitle.length > 0 ? 56 : 46
    leftPadding: 12
    rightPadding: 12
    scale: down && !ShellState.reducedMotion ? 0.985 : 1
    opacity: enabled ? 1 : 0.45

    Behavior on scale {
        enabled: !ShellState.reducedMotion
        NumberAnimation {
            duration: Theme.motionFast
            easing.type: Easing.OutCubic
        }
    }

    contentItem: RowLayout {
        spacing: 11

        Text {
            visible: control.glyph.length > 0
            text: control.glyph
            color: control.danger ? Theme.clay : Theme.foreground
            font.pixelSize: 16
            font.weight: Font.DemiBold
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
            Layout.preferredWidth: 8
            Layout.preferredHeight: 8
            radius: 4
            color: Theme.accent
        }
    }

    background: Rectangle {
        color: control.down ? Theme.selection
            : control.selected ? Theme.surfaceAlt
            : control.hovered && control.enabled ? Theme.surfaceAlt
            : "transparent"
        radius: 8
        border.width: control.activeFocus ? 2 : 0
        border.color: Theme.foreground

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: 12
            height: 1
            color: Theme.separator
        }
    }
}
