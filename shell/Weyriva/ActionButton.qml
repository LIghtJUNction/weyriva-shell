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
    property bool emphasized: false

    hoverEnabled: true
    implicitWidth: compact ? 36 : Math.max(88, contentItem.implicitWidth + 26)
    implicitHeight: compact ? 36 : subtitle.length > 0 ? 52 : 42
    leftPadding: compact ? 0 : 12
    rightPadding: compact ? 0 : 12
    enabled: true
    scale: down && !ShellState.reducedMotion ? 0.965 : 1
    opacity: enabled ? (down ? 0.76 : 1) : 0.42
    ToolTip.visible: compact && hovered
    ToolTip.text: text
    ToolTip.delay: 500

    Behavior on scale {
        enabled: !control.down && !ShellState.reducedMotion
        SpringAnimation { spring: 5; damping: 0.9; epsilon: 0.001 }
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
                ? (control.selected || control.emphasized ? Theme.onSelection
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
                    ? (control.selected || control.emphasized ? Theme.onSelection
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

    background: Item {
        ContinuousSurface {
            anchors.fill: parent
            fillColor: control.activeFocus ? Theme.focusRing : "transparent"
            cornerRadius: control.compact
                ? control.height / 2 : Theme.radiusSmall + 2
        }
        ContinuousSurface {
            anchors.fill: parent
            anchors.margins: control.activeFocus ? 2 : 0
            fillColor: control.down ? (control.danger ? Theme.clay : Theme.pressed)
                : control.selected || control.emphasized ? Theme.selection
                : control.hovered && control.enabled ? Theme.hover : "transparent"
            cornerRadius: control.compact
                ? height / 2 : Theme.radiusSmall
        }
    }
}
