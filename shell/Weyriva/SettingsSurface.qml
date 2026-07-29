import QtQuick
import QtQuick.Layouts

ColumnLayout {
    spacing: 0

    Text {
        Layout.leftMargin: 12
        Layout.bottomMargin: 6
        text: "Appearance"
        color: Theme.muted
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }

    UtilityRow {
        Layout.fillWidth: true
        selected: ShellState.dark
        text: "Color mode"
        subtitle: "Switch the shell and matching wallpaper"
        value: ShellState.dark ? "Dark" : "Light"
        onClicked: ShellState.setDark(!ShellState.dark)
    }

    Text {
        Layout.leftMargin: 12
        Layout.topMargin: 16
        Layout.bottomMargin: 6
        text: "Accessibility"
        color: Theme.muted
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }

    UtilityRow {
        Layout.fillWidth: true
        selected: ShellState.reducedMotion
        text: "Reduced motion"
        subtitle: ShellState.reducedMotion
            ? "Panels use a short cross-fade"
            : "Panels move from their source"
        value: ShellState.reducedMotion ? "On" : "Off"
        onClicked: ShellState.reducedMotion = !ShellState.reducedMotion
    }

    Item { Layout.fillHeight: true }
}
