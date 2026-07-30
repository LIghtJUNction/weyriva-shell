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

    implicitWidth: compact ? 32 : Math.max(82, contentItem.implicitWidth + 22)
    implicitHeight: compact ? 32 : subtitle.length > 0 ? 48 : 38
    leftPadding: compact ? 0 : 12
    rightPadding: compact ? 0 : 12
    enabled: true
    scale: down && !ShellState.reducedMotion ? 0.96 : 1
    opacity: enabled ? (down ? 0.76 : 1) : 0.42
    ToolTip.visible: compact && hovered
    ToolTip.text: text
    ToolTip.delay: 500

    Behavior on scale {
        enabled: !control.down && !ShellState.reducedMotion
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
                ? (control.selected ? Theme.onSelection
                    : control.chrome ? Theme.chromeText : Theme.foreground)
                : (control.chrome ? Theme.chromeMuted : Theme.muted)
            font.pixelSize: control.compact ? 14 : 15
            font.weight: Font.DemiBold
        }

        Column {
            visible: !control.compact
            spacing: 2
            Text {
                text: control.text
                color: control.enabled
                    ? (control.selected ? Theme.onSelection
                        : control.chrome ? Theme.chromeText : Theme.foreground)
                    : (control.chrome ? Theme.chromeMuted : Theme.muted)
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: -0.1
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
        color: control.down ? (control.danger ? Theme.clay : Theme.pressed)
             : control.selected ? Theme.selection
             : control.hovered && control.enabled
                 ? Theme.hover
             : "transparent"
        radius: Theme.radiusSmall
        border.width: control.activeFocus ? 2 : 0
        border.color: Theme.focusRing
    }
}
