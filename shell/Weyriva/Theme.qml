pragma Singleton
import QtQuick

QtObject {
    readonly property color ink: "#141413"
    readonly property color ivory: "#FAF9F5"
    readonly property color paper: "#F0EEE6"
    readonly property color cactus: "#BCD1CA"
    readonly property color clay: "#D97757"
    readonly property color background: ShellState.dark ? ink : cactus
    readonly property color surface: ShellState.dark ? "#232321" : ivory
    readonly property color surfaceAlt: ShellState.dark ? "#32322F" : paper
    readonly property color foreground: ShellState.dark ? ivory : ink
    readonly property color muted: ShellState.dark ? "#C8C6BD" : "#686761"
    readonly property color separator: ShellState.dark
        ? Qt.rgba(0.98, 0.976, 0.961, 0.18)
        : Qt.rgba(0.078, 0.078, 0.074, 0.18)
    readonly property color accent: ShellState.dark ? "#9AB8AE" : cactus
    readonly property color selection: accent
    readonly property color onSelection: ink
    readonly property color chrome: ShellState.dark ? "#1B1B1A" : ivory
    readonly property color chromeText: foreground
    readonly property color chromeMuted: muted
    readonly property int radius: 18
    readonly property int motionFast: ShellState.reducedMotion ? 0 : 105
    readonly property int motionPanel: ShellState.reducedMotion ? 90 : 210
}
