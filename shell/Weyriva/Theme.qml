pragma Singleton
import QtQuick

QtObject {
    readonly property color ink: "#141413"
    readonly property color ivory: "#FAF9F5"
    readonly property color paper: "#F0EEE6"
    readonly property color cactus: "#BCD1CA"
    readonly property color clay: "#D97757"
    readonly property color background: ShellState.dark ? ink : cactus
    readonly property color surface: ivory
    readonly property color surfaceAlt: paper
    readonly property color foreground: ink
    readonly property color muted: "#686761"
    readonly property color carrier: cactus
    readonly property color separator: "#14141333"
    readonly property color chrome: ShellState.dark ? ink : ivory
    readonly property color chromeText: ShellState.dark ? ivory : ink
    readonly property color chromeMuted: ShellState.dark ? "#C8C6BD" : muted
    readonly property int radius: 18
    readonly property int motionFast: ShellState.reducedMotion ? 0 : 105
    readonly property int motionPanel: ShellState.reducedMotion ? 90 : 210
}
