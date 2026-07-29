import QtQuick
import QtQuick.Controls

Button {
    id: control
    property string glyph: ""
    property string subtitle: ""
    property bool compact: false
    property bool selected: false
    property bool chrome: false
    property bool danger: false

    implicitWidth: compact ? 42 : Math.max(104, contentItem.implicitWidth + 28)
    implicitHeight: compact ? 42 : subtitle.length > 0 ? 58 : 44
    leftPadding: compact ? 0 : 14
    rightPadding: compact ? 0 : 14
    enabled: true
    scale: down && !ShellState.reducedMotion ? 0.955 : 1
    opacity: enabled ? 1 : 0.48

    Behavior on scale {
        enabled: !ShellState.reducedMotion
        NumberAnimation { duration: Theme.motionFast; easing.type: Easing.OutCubic }
    }

    contentItem: Row {
        spacing: control.compact ? 0 : 10
        anchors.centerIn: parent

        Text {
            visible: control.glyph.length > 0
            width: control.compact ? control.width : implicitWidth
            horizontalAlignment: Text.AlignHCenter
            text: control.glyph
            color: control.enabled
                ? (control.selected ? Theme.ink
                    : control.chrome ? Theme.chromeText : Theme.foreground)
                : (control.chrome ? Theme.chromeMuted : Theme.muted)
            font.pixelSize: control.compact ? 17 : 16
            font.weight: Font.DemiBold
        }

        Column {
            visible: !control.compact
            spacing: 2
            Text {
                text: control.text
                color: control.enabled
                    ? (control.selected ? Theme.ink
                        : control.chrome ? Theme.chromeText : Theme.foreground)
                    : (control.chrome ? Theme.chromeMuted : Theme.muted)
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            Text {
                visible: control.subtitle.length > 0
                text: control.subtitle
                color: control.chrome ? Theme.chromeMuted : Theme.muted
                font.pixelSize: 11
            }
        }
    }

    background: Rectangle {
        color: control.down ? (control.danger ? Theme.clay : Theme.carrier)
             : control.selected ? Theme.carrier
             : control.hovered && control.enabled
                 ? (control.chrome ? Theme.separator : Theme.surfaceAlt)
             : "transparent"
        radius: control.compact ? height / 2 : Theme.radius
        border.width: control.activeFocus ? 3 : 0
        border.color: control.chrome ? Theme.chromeText : Theme.foreground

        Behavior on color {
            enabled: !ShellState.reducedMotion
            ColorAnimation { duration: Theme.motionFast }
        }
    }
}
