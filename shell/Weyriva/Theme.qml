pragma Singleton
import QtQuick

QtObject {
    readonly property color ink: "#141413"
    readonly property color ivory: "#FAF9F5"
    readonly property color paper: "#F0EEE6"
    readonly property color seaGlass: "#BCD1CA"
    readonly property color clay: "#D97757"
    readonly property color background: ShellState.dark ? "#111210" : ivory
    readonly property color surfaceFrame: ShellState.dark ? "#34362F" : "#DEDBD1"
    readonly property color surface: ShellState.dark ? "#1C1E1A" : ivory
    readonly property color surfaceAlt: ShellState.dark ? "#282A25" : "#F0EEE7"
    readonly property color surfaceRaised: ShellState.dark ? "#34362F" : "#FFFFFF"
    readonly property color navigationMaterial: ShellState.dark ? "#242620" : "#EAE8E0"
    readonly property color inspectorMaterial: ShellState.dark ? "#20221E" : "#F4F2EC"
    readonly property color scrim: ShellState.dark
        ? Qt.rgba(0.02, 0.02, 0.02, 0.42)
        : Qt.rgba(0.078, 0.078, 0.074, 0.16)
    readonly property color foreground: ShellState.dark ? ivory : ink
    readonly property color muted: ShellState.dark ? "#B8B6AC" : "#686761"
    readonly property color separator: ShellState.dark
        ? Qt.rgba(0.98, 0.976, 0.961, 0.14)
        : Qt.rgba(0.078, 0.078, 0.074, 0.14)
    readonly property color hover: ShellState.dark
        ? Qt.rgba(0.98, 0.976, 0.961, 0.08)
        : Qt.rgba(0.078, 0.078, 0.074, 0.06)
    readonly property color pressed: ShellState.dark ? "#3A403B" : "#E2E8E3"
    readonly property color accent: ShellState.dark ? seaGlass : "#6F9187"
    readonly property color accentWarm: ShellState.dark ? "#EE9B7F" : clay
    readonly property color selection: ShellState.dark ? ivory : ink
    readonly property color onSelection: ShellState.dark ? ink : ivory
    readonly property color focusRing: ShellState.dark ? ivory : ink
    readonly property color chrome: ShellState.dark
        ? Qt.rgba(0.11, 0.12, 0.10, 0.96) : Qt.rgba(0.98, 0.976, 0.961, 0.94)
    readonly property color chromeText: foreground
    readonly property color chromeMuted: muted
    readonly property int radius: 24
    readonly property int radiusSmall: 14
    readonly property int panelRadius: 36
    readonly property int pillRadius: 999
    readonly property int motionFast: ShellState.reducedMotion ? 0 : 90
    readonly property int motionPanel: ShellState.reducedMotion ? 90 : 180
}
