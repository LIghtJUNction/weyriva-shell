pragma Singleton
import QtQuick

QtObject {
    readonly property color ink: "#141413"
    readonly property color ivory: "#FAF9F5"
    readonly property color paper: "#F0EEE6"
    readonly property color cactus: "#BCD1CA"
    readonly property color clay: "#D97757"
    readonly property color background: ShellState.dark ? "#10110F" : cactus
    readonly property color surface: ShellState.dark ? "#1E1F1C" : ivory
    readonly property color surfaceAlt: ShellState.dark ? "#292A27" : paper
    readonly property color foreground: ShellState.dark ? ivory : ink
    readonly property color muted: ShellState.dark ? "#B8B6AC" : "#686761"
    readonly property color separator: ShellState.dark
        ? Qt.rgba(0.98, 0.976, 0.961, 0.14)
        : Qt.rgba(0.078, 0.078, 0.074, 0.14)
    readonly property color hover: ShellState.dark
        ? Qt.rgba(0.98, 0.976, 0.961, 0.08)
        : Qt.rgba(0.078, 0.078, 0.074, 0.06)
    readonly property color pressed: ShellState.dark ? "#39443F" : "#DDE8E3"
    readonly property color accent: ShellState.dark ? "#B4CCC4" : "#789B90"
    readonly property color selection: cactus
    readonly property color onSelection: ink
    readonly property color focusRing: ShellState.dark ? ivory : ink
    readonly property color chrome: ShellState.dark ? "#181916" : ivory
    readonly property color chromeText: foreground
    readonly property color chromeMuted: muted
    readonly property int radius: 16
    readonly property int radiusSmall: 7
    readonly property int motionFast: ShellState.reducedMotion ? 0 : 90
    readonly property int motionPanel: ShellState.reducedMotion ? 90 : 180
}
