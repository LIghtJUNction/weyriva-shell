import QtQuick
import Quickshell.Io

Item {
    id: root

    visible: false
    property string pendingKind: ""

    function adjust(kind, direction) {
        if (actionProcess.running || queryProcess.running) {
            ShellState.showToast("A system adjustment is already running", "attention")
            return
        }
        if (!(["volume", "brightness"].includes(kind))
                || !(["up", "down", "toggle"].includes(direction))) {
            ShellState.showToast("Unsupported system adjustment", "attention")
            return
        }
        if (kind === "brightness" && direction === "toggle") {
            ShellState.showToast("Brightness does not support toggle", "attention")
            return
        }
        pendingKind = kind
        if (kind === "volume") {
            actionProcess.command = direction === "toggle"
                ? ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]
                : ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@",
                    direction === "up" ? "5%+" : "5%-", "--limit", "1.0"]
        } else {
            actionProcess.command = [
                "brightnessctl", "set", direction === "up" ? "+5%" : "5%-"
            ]
        }
        actionProcess.running = true
    }

    function queryCurrent() {
        queryProcess.command = pendingKind === "volume"
            ? ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]
            : ["brightnessctl", "-m"]
        queryProcess.running = true
    }

    function publish(text) {
        if (pendingKind === "volume") {
            const match = /Volume:\s+([0-9.]+)/.exec(text)
            if (!match)
                throw new Error("Could not read volume")
            const muted = text.includes("[MUTED]")
            ShellState.showOsd(
                "volume", muted ? "Muted" : "Volume",
                Number(match[1]), muted
            )
            return
        }
        const match = /,(\d+)%/.exec(text)
        if (!match)
            throw new Error("Could not read brightness")
        ShellState.showOsd("brightness", "Brightness", Number(match[1]) / 100, false)
    }

    Process {
        id: actionProcess
        stderr: StdioCollector { id: actionError }
        onRunningChanged: {
            if (running)
                return
            if (actionError.text.trim().length > 0) {
                ShellState.showToast(
                    actionError.text.trim() || "System adjustment failed",
                    "attention"
                )
                root.pendingKind = ""
                return
            }
            root.queryCurrent()
        }
    }

    Process {
        id: queryProcess
        stdout: StdioCollector { id: queryOutput }
        stderr: StdioCollector { id: queryError }
        onRunningChanged: {
            if (running)
                return
            if (queryOutput.text.trim().length === 0) {
                ShellState.showToast(
                    queryError.text.trim() || "Could not read system state",
                    "attention"
                )
                root.pendingKind = ""
                return
            }
            try {
                root.publish(queryOutput.text)
            } catch (error) {
                ShellState.showToast(error.message, "attention")
            }
            root.pendingKind = ""
        }
    }
}
