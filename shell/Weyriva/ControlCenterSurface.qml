import QtQuick
import QtQuick.Layouts

ColumnLayout {
    spacing: 0

    UtilityRow {
        Layout.fillWidth: true
        selected: ShellState.doNotDisturb
        glyph: "◐"
        text: "Do Not Disturb"
        value: ShellState.doNotDisturb ? "On" : "Off"
        onClicked: ShellState.doNotDisturb = !ShellState.doNotDisturb
    }

    UtilityRow {
        Layout.fillWidth: true
        selected: ShellState.dark
        glyph: "◑"
        text: "Appearance"
        value: ShellState.dark ? "Dark" : "Light"
        onClicked: ShellState.setDark(!ShellState.dark)
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: ">_"
        text: "Terminal"
        onClicked: ShellState.launch(["foot"])
    }

    UtilityRow {
        Layout.fillWidth: true
        glyph: "□"
        text: "Lock"
        onClicked: ShellState.requestLock()
    }

    Item { Layout.fillHeight: true }
}
