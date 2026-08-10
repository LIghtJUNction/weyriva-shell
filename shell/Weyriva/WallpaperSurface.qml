import QtQuick
import QtQuick.Layouts

ColumnLayout {
    readonly property string selectedName: {
        if (ShellState.wallpaper === ShellState.lightWallpaper)
            return "Quiet coast"
        if (ShellState.wallpaper === ShellState.coralWallpaper)
            return "Coral field"
        if (ShellState.wallpaper === ShellState.darkWallpaper)
            return "Coast at night"
        return "Custom background"
    }

    spacing: 10

    Text {
        text: "EDITORIAL BACKGROUNDS"
        color: Theme.muted
        font.pixelSize: 10
        font.weight: Font.Bold
        font.letterSpacing: 1.1
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 12

        WallpaperPreview {
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "Quiet coast"
            imageSource: ShellState.lightWallpaper
            darkAppearance: false
        }

        WallpaperPreview {
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "Coral field"
            imageSource: ShellState.coralWallpaper
            darkAppearance: false
        }

        WallpaperPreview {
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "Coast at night"
            imageSource: ShellState.darkWallpaper
            darkAppearance: true
        }
    }

    Text {
        Layout.fillWidth: true
        text: "Selected · " + parent.selectedName
        color: Theme.muted
        horizontalAlignment: Text.AlignRight
        font.pixelSize: 11
    }
}
