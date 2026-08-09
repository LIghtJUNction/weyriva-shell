import QtCore
import QtQuick
import Quickshell.Io
import Compat as WeyrivaCompat

Item {
    id: root
    visible: false

    property var bootstrap: ({})
    property string loadError: ""
    property bool initialized: false
    property bool launcherInitialized: false
    property var pendingActions: []
    property string currentQuery: ""

    function boundedString(value, limit, allowEmpty) {
        const text = String(value ?? "")
        if ((!allowEmpty && text.length === 0) || text.length > limit)
            throw new Error("bounded string is invalid")
        return text
    }

    function response(result, id) {
        return JSON.stringify({ ok: true, id: id, result: result, error: null })
    }

    function failure(code, message, id) {
        return JSON.stringify({
            ok: false,
            id: id ?? null,
            result: null,
            error: { code: code, message: String(message) }
        })
    }

    function exactKeys(value, required) {
        if (value === null || typeof value !== "object" || Array.isArray(value))
            return false
        const keys = Object.keys(value).sort()
        const expected = required.slice().sort()
        return JSON.stringify(keys) === JSON.stringify(expected)
    }

    function fileUrl(path) {
        return Qt.resolvedUrl(path)
    }

    function initialize() {
        try {
            bootstrap = JSON.parse(bootstrapFile.text())
            if (!exactKeys(bootstrap, [
                "protocol",
                "profile",
                "pluginId",
                "pluginDir",
                "main",
                "launcherProvider",
                "handlerTarget"
            ]))
                throw new Error("bootstrap shape is invalid")
            mainLoader.setSource(fileUrl(bootstrap.pluginDir + "/" + bootstrap.main), {
                pluginApi: pluginApi
            })
            launcherLoader.setSource(
                fileUrl(bootstrap.launcherProvider),
                { pluginApi: pluginApi, launcher: launcherAdapter }
            )
            initialized = true
        } catch (error) {
            loadError = String(error)
        }
    }

    function readyResult() {
        if (loadError.length > 0)
            throw new Error(loadError)
        if (!initialized
                || mainLoader.status !== Loader.Ready
                || launcherLoader.status !== Loader.Ready)
            throw new Error("v4 entries are not ready")
        pluginApi.mainInstance = mainLoader.item
        if (!launcherInitialized) {
            launcherInitialized = true
            if (typeof launcherLoader.item.init === "function")
                launcherLoader.item.init()
        }
        return {
            ready: true,
            profile: bootstrap.profile,
            handlerTarget: bootstrap.handlerTarget
        }
    }

    function query(params) {
        if (!exactKeys(params, ["query"]) || typeof params.query !== "string")
            throw new Error("query params are invalid")
        readyResult()
        currentQuery = params.query
        pendingActions = []
        launcherAdapter.reset()
        const provider = launcherLoader.item
        let handlesQuery = true
        if (typeof provider.handleCommand === "function")
            handlesQuery = provider.handleCommand(params.query) !== false
        if (typeof provider.getResults === "function") {
            launcherAdapter.updateResults(
                handlesQuery ? provider.getResults(params.query) : []
            )
        } else if (typeof provider.handleCommand !== "function") {
            throw new Error("launcher provider has no query callback")
        }
        return {
            query: params.query,
            results: launcherAdapter.normalizedResults,
            actions: pendingActions
        }
    }

    function clipboardText(command) {
        if (Array.isArray(command) && command.length === 2
                && command[0] === "wl-copy"
                && typeof command[1] === "string")
            return command[1]
        if (!Array.isArray(command) || command.length !== 3
                || command[0] !== "sh" || command[1] !== "-c"
                || typeof command[2] !== "string")
            throw new Error("unsupported detached command")
        const script = command[2]
        const prefix = "printf '%s' '"
        const suffix = "' | wl-copy"
        if (!script.startsWith(prefix) || !script.endsWith(suffix))
            throw new Error("unsupported detached command")
        const encoded = script.slice(prefix.length, script.length - suffix.length)
        const quoteEscape = "'\\''"
        let text = ""
        for (let index = 0; index < encoded.length; index++) {
            if (encoded[index] !== "'") {
                text += encoded[index]
                continue
            }
            if (encoded.slice(index, index + quoteEscape.length) !== quoteEscape)
                throw new Error("unsupported detached command")
            text += "'"
            index += quoteEscape.length - 1
        }
        return text
    }

    function activate(params) {
        if (!exactKeys(params, ["id"]) || typeof params.id !== "string")
            throw new Error("activate params are invalid")
        readyResult()
        pendingActions = []
        launcherAdapter.activate(params.id)
        const commands = WeyrivaCompat.Quickshell.takeDetachedCommands()
        if (commands.length === 0)
            throw new Error("activation did not request the pinned copy action")
        for (const command of commands) {
            const text = clipboardText(command)
            if (text.length > 16384)
                throw new Error("clipboard text exceeds 16 KiB")
            pendingActions.push({
                type: "clipboard",
                text: text,
                mime: "text/plain"
            })
        }
        return { activated: params.id, actions: pendingActions }
    }

    function drainActions(params) {
        if (!exactKeys(params, []))
            throw new Error("action params are invalid")
        const actions = pendingActions
        pendingActions = []
        return { actions: actions }
    }

    function dispatch(encoded) {
        let requestId = null
        try {
            const request = JSON.parse(encoded)
            if (!exactKeys(request, ["protocol", "id", "method", "params"])
                    || typeof request.id !== "number"
                    || !Number.isInteger(request.id)
                    || request.id < 0
                    || request.protocol !== "weyriva-v4-host/1")
                return failure("host_protocol", "request envelope is invalid", null)
            requestId = request.id
            if (request.method === "ready")
                return response(readyResult(), request.id)
            if (request.method === "query")
                return response(query(request.params), request.id)
            if (request.method === "activate")
                return response(activate(request.params), request.id)
            if (request.method === "drain_actions")
                return response(drainActions(request.params), request.id)
            if (request.method === "shutdown") {
                if (!exactKeys(request.params, []))
                    return failure("invalid_params", "shutdown params are invalid", request.id)
                Qt.callLater(Qt.quit)
                return response({ onExit: false, actions: [] }, request.id)
            }
            return failure("unknown_method", "unsupported v4 host method", request.id)
        } catch (error) {
            return failure("plugin_error", error, requestId)
        }
    }

    FileView {
        id: bootstrapFile
        path: StandardPaths.writableLocation(StandardPaths.RuntimeLocation) + "/bootstrap.json"
        blockLoading: true
    }

    QtObject {
        id: pluginApi
        property string pluginId: root.bootstrap.pluginId ?? ""
        property string pluginDir: root.bootstrap.pluginDir ?? ""
        property var pluginSettings: ({})
        property var manifest: ({})
        property string currentLanguage: "en"
        property var pluginTranslations: ({})
        property var mainInstance: null

        function saveSettings() {}
        function withCurrentScreen(callback) {
            if (typeof callback === "function")
                callback(null)
        }
        function toggleLauncher() {
            root.pendingActions.push({ type: "launcher_set_query", query: "" })
        }
        function openLauncher() {
            toggleLauncher()
        }
        function closeLauncher() {}
    }

    QtObject {
        id: launcherAdapter
        property var rawResults: []
        property var normalizedResults: []

        function reset() {
            rawResults = []
            normalizedResults = []
        }

        function updateResults(values) {
            if (values === undefined) {
                const provider = launcherLoader.item
                values = provider && typeof provider.getResults === "function"
                    ? provider.getResults(root.currentQuery)
                    : []
            }
            if (!Array.isArray(values) || values.length > 2000)
                throw new Error("launcher results are invalid")
            rawResults = values
            normalizedResults = values.map(function(value, index) {
                const id = root.boundedString(value.id ?? String(index), 256, false)
                const title = root.boundedString(
                    value.title ?? value.name ?? value.text ?? value.value,
                    512,
                    false
                )
                const result = { id: id, title: title }
                if (value.subtitle !== undefined || value.description !== undefined)
                    result.subtitle = root.boundedString(
                        value.subtitle ?? value.description,
                        1024,
                        true
                    )
                if (value.glyph !== undefined || value.icon !== undefined)
                    result.glyph = root.boundedString(value.glyph ?? value.icon, 128, true)
                if (value.category !== undefined)
                    result.category = root.boundedString(value.category, 256, true)
                return result
            })
        }

        function setSearchText(value) {
            root.pendingActions.push({
                type: "launcher_set_query",
                query: root.boundedString(value, 4096, true)
            })
        }

        function close() {}

        function activate(id) {
            const index = normalizedResults.findIndex(function(value) {
                return value.id === id
            })
            if (index < 0)
                throw new Error("unknown launcher result")
            const value = rawResults[index]
            const callback = value.activate ?? value.onActivate ?? value.action
            if (typeof callback !== "function")
                throw new Error("launcher result has no activation callback")
            callback()
        }
    }

    Loader {
        id: mainLoader
        onStatusChanged: {
            if (status === Loader.Error)
                root.loadError = "Main.qml failed to load"
        }
    }

    Loader {
        id: launcherLoader
        onStatusChanged: {
            if (status === Loader.Error)
                root.loadError = "LauncherProvider.qml failed to load"
        }
    }

    IpcHandler {
        target: "weyriva-v4-host"
        function request(encoded: string): string {
            return root.dispatch(encoded)
        }
    }

    Component.onCompleted: initialize()
}
