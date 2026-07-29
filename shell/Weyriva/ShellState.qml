pragma Singleton
import QtQuick
import Quickshell

QtObject {
    signal requestLock()

    property string route: ""
    property date now: new Date()
    property bool dark: false
    property bool reducedMotion: false
    property bool doNotDisturb: false
    property bool barVisible: true
    property string wallpaper: "/usr/share/weyriva/wallpapers/light/weyriva-cactus.png"

    function toggleRoute(nextRoute) {
        route = route === nextRoute ? "" : nextRoute
    }

    function launch(command) {
        Quickshell.execDetached(command)
        route = ""
    }

    function setDark(enabled) {
        dark = enabled
        wallpaper = enabled
            ? "/usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png"
            : "/usr/share/weyriva/wallpapers/light/weyriva-cactus.png"
    }

    function useWallpaper(path, darkAppearance) {
        wallpaper = path
        dark = darkAppearance
    }
}
