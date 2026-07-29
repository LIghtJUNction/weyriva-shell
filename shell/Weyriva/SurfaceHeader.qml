import QtQuick
import QtQuick.Layouts

RowLayout {
    id: root

    required property string title
    required property bool utility

    spacing: 10

    Text {
        Layout.fillWidth: true
        text: root.title
        color: Theme.foreground
        font.pixelSize: root.utility ? 19 : 24
        font.weight: Font.DemiBold
        font.letterSpacing: root.utility ? -0.1 : -0.4
    }

    ActionButton {
        glyph: "×"
        text: "Close"
        compact: true
        onClicked: ShellState.closeRoute()
    }
}
