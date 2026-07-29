import QtQuick
import QtQuick.Layouts

ColumnLayout {
    spacing: 10

    Text {
        text: "Desktop background"
        color: Theme.muted
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 12

        WallpaperPreview {
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "Cactus daylight"
            imageSource:
                "/usr/share/weyriva/wallpapers/light/weyriva-cactus.png"
            darkAppearance: false
        }

        WallpaperPreview {
            Layout.fillWidth: true
            Layout.fillHeight: true
            text: "Cactus after dark"
            imageSource:
                "/usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png"
            darkAppearance: true
        }
    }

    Text {
        Layout.fillWidth: true
        text: ShellState.dark ? "Cactus after dark" : "Cactus daylight"
        color: Theme.muted
        horizontalAlignment: Text.AlignRight
        font.pixelSize: 11
    }
}
