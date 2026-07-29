pragma ComponentBehavior: Bound
import QtQuick
import Quickshell.Io

Item {
    id: root

    property string input: ""
    property var providers: []
    property var results: []
    property string error: ""
    property bool loading: false
    property bool activationPending: false
    property string pendingResultId: ""
    property var activeProvider: providerForInput(input)
    readonly property bool providerMode: activeProvider !== null
    readonly property string providerQuery: {
        if (!activeProvider)
            return ""
        return input.trim().slice(activeProvider.prefix.length).trimStart()
    }

    signal queryReplacementRequested(string providerPrefix, string query)

    implicitWidth: 0
    implicitHeight: 0
    visible: false

    function providerForInput(value) {
        const trimmed = value.trimStart()
        for (let index = 0; index < providers.length; ++index) {
            const prefix = providers[index].prefix
            if (trimmed === prefix || trimmed.startsWith(prefix + " "))
                return providers[index]
        }
        return null
    }

    function responseResult(text) {
        let envelope
        try {
            envelope = JSON.parse(text)
        } catch (parseError) {
            throw new Error("Weyriva returned invalid JSON")
        }
        if (envelope.error)
            throw new Error(envelope.error.message || "Plugin request failed")
        if (!envelope.result)
            throw new Error("Weyriva returned no plugin result")
        return envelope.result
    }

    function refreshProviders() {
        if (statusProcess.running)
            return
        statusProcess.command = ["weyriva", "plugin", "status"]
        statusProcess.running = true
    }

    function scheduleQuery() {
        queryTimer.stop()
        results = []
        error = ""
        if (!providerMode) {
            loading = false
            return
        }
        loading = true
        queryTimer.interval = Math.max(0, activeProvider.debounce_ms || 120)
        queryTimer.start()
    }

    function runQuery() {
        if (!providerMode || queryProcess.running)
            return
        queryProcess.expectedProvider = activeProvider.reference
        queryProcess.expectedQuery = providerQuery
        queryProcess.command = [
            "weyriva", "plugin", "query",
            activeProvider.reference, providerQuery
        ]
        queryProcess.running = true
    }

    function activate(resultId) {
        if (!providerMode || activationProcess.running)
            return
        const provider = activeProvider
        activationPending = true
        pendingResultId = resultId
        error = ""
        activationProcess.expectedProvider = provider.reference
        activationProcess.expectedPrefix = provider.prefix
        activationProcess.command = [
            "weyriva", "plugin", "activate",
            provider.reference, resultId
        ]
        activationProcess.running = true
    }

    onInputChanged: scheduleQuery()
    Component.onCompleted: refreshProviders()

    Timer {
        id: queryTimer
        repeat: false
        onTriggered: root.runQuery()
    }

    Process {
        id: statusProcess

        stdout: StdioCollector { id: statusOutput }

        onRunningChanged: {
            if (running)
                return
            if (statusOutput.text.trim().length === 0) {
                root.error = "Plugin service is unavailable"
                root.providers = []
                return
            }
            try {
                const status = root.responseResult(statusOutput.text)
                const nextProviders = []
                for (let index = 0; index < status.plugins.length; ++index) {
                    const plugin = status.plugins[index]
                    if (!plugin.enabled || plugin.lifecycle !== "running")
                        continue
                    const provider = plugin.provider
                    nextProviders.push({
                        reference: plugin.id + ":" + provider.entry_id,
                        prefix: "/" + provider.prefix,
                        glyph: provider.glyph || "",
                        name: provider.name,
                        debounce_ms: provider.debounce_ms || 120,
                        categories: provider.categories || []
                    })
                }
                root.providers = nextProviders
                root.error = ""
                root.scheduleQuery()
            } catch (requestError) {
                root.providers = []
                root.error = requestError.message
            }
        }
    }

    Process {
        id: queryProcess

        property string expectedProvider: ""
        property string expectedQuery: ""

        stdout: StdioCollector { id: queryOutput }

        onRunningChanged: {
            if (running)
                return
            if (
                !root.activeProvider
                || root.activeProvider.reference !== expectedProvider
                || root.providerQuery !== expectedQuery
            ) {
                root.scheduleQuery()
                return
            }
            root.loading = false
            if (queryOutput.text.trim().length === 0) {
                root.results = []
                root.error = "Plugin query failed"
                return
            }
            try {
                const result = root.responseResult(queryOutput.text)
                root.results = result.results || []
                root.error = ""
            } catch (requestError) {
                root.results = []
                root.error = requestError.message
            }
        }
    }

    Process {
        id: activationProcess

        property string expectedProvider: ""
        property string expectedPrefix: ""

        stdout: StdioCollector { id: activationOutput }

        onRunningChanged: {
            if (running)
                return
            root.activationPending = false
            root.pendingResultId = ""
            if (activationOutput.text.trim().length === 0) {
                root.error = "Plugin activation failed"
                return
            }
            try {
                const result = root.responseResult(activationOutput.text)
                const outcomes = result.action_results || []
                for (let index = 0; index < outcomes.length; ++index) {
                    if (outcomes[index].type === "set_query")
                        root.queryReplacementRequested(
                            expectedPrefix,
                            outcomes[index].query
                        )
                }
                root.error = ""
            } catch (requestError) {
                root.error = requestError.message
            }
        }
    }
}
