pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io

Item {
    id: root

    required property bool active
    property var entries: []
    property string error: ""
    property bool loading: false
    property string copyingId: ""
    property bool wipeArmed: false
    readonly property var filteredEntries: {
        const query = search.text.trim().toLowerCase()
        if (query.length === 0)
            return entries
        return entries.filter(entry => entry.preview.toLowerCase().includes(query))
    }

    function parseEntries(text) {
        const result = []
        const lines = text.split("\n")
        for (let index = 0; index < lines.length && result.length < 150; ++index) {
            const separator = lines[index].indexOf("\t")
            if (separator <= 0)
                continue
            const id = lines[index].slice(0, separator)
            if (!/^\d{1,20}$/.test(id))
                continue
            const preview = lines[index].slice(separator + 1).trim()
            result.push({ id: id, preview: preview || "Non-text clipboard item" })
        }
        return result
    }

    function refresh() {
        if (listProcess.running)
            return
        loading = true
        error = ""
        listProcess.command = ["cliphist", "list"]
        listProcess.running = true
    }

    function copyEntry(entry) {
        if (copyProcess.running || !entry || !/^\d{1,20}$/.test(entry.id))
            return
        copyingId = entry.id
        error = ""
        copyProcess.command = [
            "weyriva", "ipc", "call", "weyriva.clipboard.copy",
            "--params", JSON.stringify({ id: entry.id })
        ]
        copyProcess.running = true
    }

    function activateSurface() {
        wipeArmed = false
        refresh()
        Qt.callLater(function() { search.forceActiveFocus() })
    }

    onActiveChanged: {
        if (active && visible)
            activateSurface()
    }
    onVisibleChanged: {
        if (visible && active)
            activateSurface()
    }

    Process {
        id: listProcess
        stdout: StdioCollector { id: listOutput }
        stderr: StdioCollector { id: listError }
        onRunningChanged: {
            if (running)
                return
            root.loading = false
            if (listError.text.trim().length > 0) {
                root.entries = []
                root.error = listError.text.trim() || "Clipboard history unavailable"
                return
            }
            root.entries = root.parseEntries(listOutput.text)
            root.error = ""
            Qt.callLater(function() { entryList.currentIndex = entryList.count > 0 ? 0 : -1 })
        }
    }

    Process {
        id: copyProcess
        stdout: StdioCollector { id: copyOutput }
        stderr: StdioCollector { id: copyError }
        onRunningChanged: {
            if (running)
                return
            root.copyingId = ""
            try {
                const envelope = JSON.parse(copyOutput.text)
                if (envelope.error)
                    throw new Error(envelope.error.message || "Clipboard copy failed")
                ShellState.showToast("Copied to clipboard", "info")
                ShellState.closeRoute()
                return
            } catch (requestError) {
                root.error = copyError.text.trim() || requestError.message
                    || "Clipboard copy failed"
            }
        }
    }

    Process {
        id: wipeProcess
        stderr: StdioCollector { id: wipeError }
        onRunningChanged: {
            if (running)
                return
            root.wipeArmed = false
            if (wipeError.text.trim().length > 0) {
                root.error = wipeError.text.trim() || "Could not clear history"
                return
            }
            ShellState.showToast("Clipboard history cleared", "info")
            root.refresh()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            TextField {
                id: search
                Layout.fillWidth: true
                implicitHeight: 44
                placeholderText: "Search clipboard · press / to focus"
                color: Theme.foreground
                placeholderTextColor: Theme.muted
                leftPadding: 12
                rightPadding: 12
                selectByMouse: true
                onTextChanged: entryList.currentIndex = entryList.count > 0 ? 0 : -1
                onAccepted: {
                    if (entryList.currentItem)
                        root.copyEntry(entryList.currentItem.modelData)
                }
                Keys.onDownPressed: entryList.forceActiveFocus()
                background: Rectangle {
                    color: Theme.surfaceAlt
                    radius: Theme.radiusSmall
                    border.width: search.activeFocus ? 2 : 1
                    border.color: search.activeFocus
                        ? Theme.focusRing : Theme.separator
                }
            }
            ActionButton {
                glyph: root.wipeArmed ? "!" : "×"
                text: "Clear clipboard history"
                compact: true
                danger: root.wipeArmed
                enabled: !wipeProcess.running && root.entries.length > 0
                onClicked: {
                    if (!root.wipeArmed) {
                        root.wipeArmed = true
                        ShellState.showToast("Press clear again to confirm", "attention")
                        return
                    }
                    wipeProcess.command = ["cliphist", "wipe"]
                    wipeProcess.running = true
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: root.error.length > 0
            text: root.error
            color: Theme.accentWarm
            elide: Text.ElideRight
            font.pixelSize: 11
        }

        ListView {
            id: entryList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: count > 0
            clip: true
            spacing: 3
            model: root.filteredEntries
            currentIndex: count > 0 ? 0 : -1
            Keys.onDownPressed: currentIndex = Math.min(currentIndex + 1, count - 1)
            Keys.onUpPressed: {
                if (currentIndex <= 0)
                    search.forceActiveFocus()
                else
                    --currentIndex
            }
            Keys.onPressed: event => {
                if (event.text === "/") {
                    search.forceActiveFocus()
                    event.accepted = true
                }
            }
            Keys.onReturnPressed: {
                if (currentItem)
                    root.copyEntry(currentItem.modelData)
            }
            Keys.onEnterPressed: {
                if (currentItem)
                    root.copyEntry(currentItem.modelData)
            }
            Keys.onEscapePressed: ShellState.closeRoute()

            delegate: Button {
                id: row
                required property var modelData
                width: ListView.view.width
                implicitHeight: 54
                enabled: root.copyingId.length === 0
                onClicked: root.copyEntry(modelData)
                contentItem: RowLayout {
                    spacing: 12
                    Text {
                        Layout.preferredWidth: 34
                        text: root.copyingId === row.modelData.id ? "…" : "⌁"
                        color: Theme.accent
                        horizontalAlignment: Text.AlignHCenter
                        font.pixelSize: 16
                        font.weight: Font.Bold
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.copyingId === row.modelData.id
                            ? "COPYING…" : row.modelData.preview
                        color: Theme.foreground
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        font.pixelSize: 13
                    }
                    Text {
                        text: row.modelData.id
                        color: Theme.muted
                        font.pixelSize: 9
                    }
                }
                background: Rectangle {
                    color: row.down ? Theme.pressed
                        : ListView.isCurrentItem ? Theme.hover
                        : row.hovered ? Theme.hover : "transparent"
                    radius: Theme.radiusSmall
                    border.width: row.activeFocus ? 2 : 0
                    border.color: Theme.focusRing
                }
            }
        }

        Column {
            Layout.alignment: Qt.AlignCenter
            visible: entryList.count === 0
            spacing: 8
            BrandMark {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 88
                height: 64
                quiet: true
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.loading ? "Reading clipboard history…"
                    : root.error.length > 0 ? "Clipboard unavailable"
                    : search.text.length > 0 ? "No matching clips"
                    : "Clipboard history is empty"
                color: Theme.foreground
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
        }
    }
}
