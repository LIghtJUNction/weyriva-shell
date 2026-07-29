pragma Singleton
import QtQuick
import Quickshell

QtObject {
    signal requestLock()

    property string route: ""
    property string presentationRoute: ""
    property var routeScreen: null
    property date now: new Date()
    property bool dark: false
    property bool reducedMotion: false
    property bool doNotDisturb: false
    property bool barVisible: true
    property string wallpaper: "/usr/share/weyriva/wallpapers/light/weyriva-cactus.png"

    function openRoute(nextRoute, sourceScreen) {
        if (!sourceScreen)
            return
        presentationRoute = nextRoute
        routeScreen = sourceScreen
        route = nextRoute
    }

    function toggleRoute(nextRoute, sourceScreen) {
        if (route === nextRoute && routeScreen === sourceScreen) {
            closeRoute()
            return
        }
        openRoute(nextRoute, sourceScreen)
    }

    function closeRoute() {
        route = ""
        routeScreen = null
    }

    function launch(command) {
        Quickshell.execDetached(command)
        closeRoute()
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
