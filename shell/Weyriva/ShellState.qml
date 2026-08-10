pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    signal requestLock()
    signal toastRequested()

    property string route: ""
    property string presentationRoute: ""
    property var routeScreen: null
    property string routeScreenName: ""
    property date now: new Date()
    property bool dark: false
    property bool reducedMotion: false
    property bool doNotDisturb: false
    property bool barVisible: true
    property string taskDraft: ""
    readonly property string lightWallpaper: Quickshell.shellPath(
        "../assets/wallpapers/weyriva-coast.svg"
    )
    readonly property string coralWallpaper: Quickshell.shellPath(
        "../assets/wallpapers/weyriva-coral.svg"
    )
    readonly property string darkWallpaper: Quickshell.shellPath(
        "../assets/wallpapers/weyriva-coast-night.svg"
    )
    property string wallpaper: lightWallpaper
    property string toastMessage: ""
    property string toastTone: "info"
    property int toastRevision: 0
    property string osdKind: ""
    property string osdLabel: ""
    property real osdValue: 0
    property bool osdMuted: false
    property int osdRevision: 0
    property bool preferencesReady: false

    property FileView preferenceFile: FileView {
        path: Quickshell.statePath("preferences.json")
        blockLoading: true
        atomicWrites: true
        printErrors: false
    }

    Component.onCompleted: loadPreferences()
    onDarkChanged: savePreferences()
    onReducedMotionChanged: savePreferences()
    onDoNotDisturbChanged: savePreferences()
    onBarVisibleChanged: savePreferences()
    onWallpaperChanged: savePreferences()

    function openRoute(nextRoute, sourceScreen) {
        if (!sourceScreen)
            return
        presentationRoute = nextRoute
        routeScreen = sourceScreen
        routeScreenName = sourceScreen.name || ""
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
        routeScreenName = ""
    }

    function launch(command) {
        Quickshell.execDetached(command)
        closeRoute()
    }

    function setDark(enabled) {
        dark = enabled
        wallpaper = enabled ? darkWallpaper : lightWallpaper
    }

    function useWallpaper(path, darkAppearance) {
        wallpaper = path
        dark = darkAppearance
    }

    function openTaskDraft(prompt, sourceScreen) {
        taskDraft = prompt
        openRoute("tasks", sourceScreen)
    }

    function showToast(message, tone) {
        toastMessage = message
        toastTone = tone || "info"
        ++toastRevision
        toastRequested()
    }

    function showOsd(kind, label, value, muted) {
        osdKind = kind
        osdLabel = label
        osdValue = Math.max(0, Math.min(1, value))
        osdMuted = muted
        ++osdRevision
    }

    function loadPreferences() {
        let stored = null
        try {
            const text = preferenceFile.text().trim()
            if (text.length > 0)
                stored = JSON.parse(text)
        } catch (error) {
            stored = null
        }
        if (stored && typeof stored === "object") {
            if (typeof stored.dark === "boolean")
                dark = stored.dark
            if (typeof stored.reducedMotion === "boolean")
                reducedMotion = stored.reducedMotion
            if (typeof stored.doNotDisturb === "boolean")
                doNotDisturb = stored.doNotDisturb
            if (typeof stored.barVisible === "boolean")
                barVisible = stored.barVisible
            if (typeof stored.wallpaper === "string"
                    && stored.wallpaper.startsWith("file:")) {
                const current = [lightWallpaper, coralWallpaper, darkWallpaper]
                const retiredPackagedAsset = stored.wallpaper.includes(
                    "/assets/wallpapers/weyriva-"
                ) && !current.includes(stored.wallpaper)
                wallpaper = retiredPackagedAsset
                    ? (dark ? darkWallpaper : lightWallpaper) : stored.wallpaper
            }
        }
        preferencesReady = true
    }

    function savePreferences() {
        if (!preferencesReady)
            return
        preferenceFile.setText(JSON.stringify({
            schema: 1,
            dark: dark,
            reducedMotion: reducedMotion,
            doNotDisturb: doNotDisturb,
            barVisible: barVisible,
            wallpaper: wallpaper
        }, null, 2) + "\n")
    }
}
