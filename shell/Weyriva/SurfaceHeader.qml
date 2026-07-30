import QtQuick
import QtQuick.Layouts

RowLayout {
    id: root

    required property string title
    required property bool utility

    implicitHeight: root.utility ? 32 : 38
    spacing: 12

    Text {
        Layout.fillWidth: true
        text: root.title
        color: Theme.foreground
        font.pixelSize: root.utility ? 18 : 22
        font.weight: Font.DemiBold
        font.letterSpacing: root.utility ? -0.2 : -0.45
    }

    ActionButton {
        glyph: "×"
        text: "Close"
        compact: true
        onClicked: ShellState.closeRoute()
    }
}
