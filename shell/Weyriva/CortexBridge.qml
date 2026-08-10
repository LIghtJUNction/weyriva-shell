pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: root
    visible: false

    property var agents: []
    property string selectedAgent: ""
    property var agentDetail: ({})
    property var tools: []
    property var sessions: []
    property string selectedSession: ""
    property string sessionDraft: ""
    property string sessionAgent: ""
    property string sessionStatus: ""
    property var messages: []
    property var activity: []
    property bool available: false
    property bool loading: false
    property bool running: false
    property string response: ""
    property string error: ""
    property string runId: ""
    property string runAgent: ""
    property string runSession: ""
    property string model: ""
    property string ownerUid: ""
    property string pendingPrompt: ""
    property string pendingSession: ""
    property bool requestPending: false
    property var pendingApproval: null
    property int inputTokens: 0
    property int outputTokens: 0
    property string detailTarget: ""
    property string sessionIndexTarget: ""
    property string historyTargetAgent: ""
    property string historyTargetSession: ""
    property var probeQueue: []
    property int probeIndex: -1
    property string probeTarget: ""

    readonly property bool selectedAgentReachable: {
        for (let index = 0; index < agents.length; ++index) {
            if (agents[index].name === selectedAgent)
                return Boolean(agents[index].reachable)
        }
        return false
    }
    readonly property string cortexRoot: {
        const configured = String(Quickshell.env("CTX_ROOT") || "")
        if (configured.startsWith("/") && configured !== "/")
            return configured.replace(/\/+$/, "")
        return configured === "/" ? "/" : "/ctx"
    }

    signal completed(string status)

    function validName(value) {
        return /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/.test(String(value))
    }

    function validUid(value) {
        return /^(0|[1-9][0-9]{0,9})$/.test(String(value))
    }

    function validApprovalArgs(value) {
        if (!Array.isArray(value))
            return false
        for (let index = 0; index < value.length; ++index) {
            if (typeof value[index] !== "string")
                return false
        }
        return true
    }

    function agentIndex(name) {
        for (let index = 0; index < agents.length; ++index) {
            if (agents[index].name === name)
                return index
        }
        return -1
    }

    function parseAgents(text) {
        const parsed = []
        const lines = String(text).split("\n")
        for (let index = 0; index < lines.length; ++index) {
            const match = /^([\s|`+\-]*)([A-Za-z0-9][A-Za-z0-9._-]*) \[([^\]]+)\](?:\s+(.*))?$/.exec(lines[index])
            if (!match)
                continue
            let modelName = ""
            const modelMatch = /(?:^|\s)model=([^\s]+)/.exec(match[4] || "")
            if (modelMatch)
                modelName = modelMatch[1]
            parsed.push({
                name: match[2],
                status: match[3],
                model: modelName,
                depth: Math.floor(match[1].length / 3),
                reachable: false
            })
        }
        return parsed
    }

    function refreshAgents() {
        if (agentProcess.running || probeProcess.running)
            return
        loading = true
        error = ""
        agentProcess.command = ["ctx", "agent", "ps"]
        agentProcess.running = true
    }

    function beginProbes() {
        probeQueue = agents.map(agent => agent.name)
        probeIndex = -1
        runNextProbe()
    }

    function runNextProbe() {
        ++probeIndex
        if (probeIndex >= probeQueue.length) {
            finishProbes()
            return
        }
        probeTarget = probeQueue[probeIndex]
        probeProcess.command = ["ctx", "ping", "agent/" + probeTarget]
        probeProcess.running = true
    }

    function completeProbe(stdout, stderr) {
        const reachable = String(stdout).includes('"type":"pong"')
            && String(stderr).trim().length === 0
        const updated = []
        for (let index = 0; index < agents.length; ++index) {
            const agent = agents[index]
            updated.push({
                name: agent.name,
                status: agent.status,
                model: agent.model,
                depth: agent.depth,
                reachable: agent.name === probeTarget ? reachable : agent.reachable
            })
        }
        agents = updated
        runNextProbe()
    }

    function finishProbes() {
        loading = false
        available = false
        let nextAgent = ""
        for (let index = 0; index < agents.length; ++index) {
            if (agents[index].reachable) {
                available = true
                if (nextAgent.length === 0)
                    nextAgent = agents[index].name
            }
        }
        const currentIndex = agentIndex(selectedAgent)
        if (currentIndex >= 0 && agents[currentIndex].reachable)
            nextAgent = selectedAgent
        else if (nextAgent.length === 0 && agents.length > 0)
            nextAgent = agents[0].name
        selectedAgent = nextAgent
        error = agents.length === 0 ? "CortexFS returned no agents"
            : available ? "" : "No CortexFS agent socket is reachable"
        refreshDetail()
    }

    function selectAgent(name) {
        if (running || !validName(name) || selectedAgent === name)
            return
        selectedAgent = name
        refreshDetail()
    }

    function refreshDetail() {
        if (!validName(selectedAgent) || detailProcess.running
                || toolsProcess.running || ownerProcess.running)
            return
        detailTarget = selectedAgent
        agentDetail = ({})
        tools = []
        ownerUid = ""
        detailProcess.command = ["ctx", "agent", "status", selectedAgent]
        toolsProcess.command = ["ctx", "agent", "tools", selectedAgent]
        ownerProcess.command = ["ctx", "cat", "agent/" + selectedAgent + ".d/uid"]
        detailProcess.running = true
        toolsProcess.running = true
        ownerProcess.running = true
    }

    function detailBatchFinished() {
        if (detailProcess.running || toolsProcess.running || ownerProcess.running)
            return
        if (detailTarget !== selectedAgent) {
            refreshDetail()
            return
        }
        refreshSessions()
    }

    function parseTools(text) {
        const parsed = []
        const lines = String(text).split("\n")
        for (let index = 0; index < lines.length; ++index) {
            const fields = lines[index].split("\t")
            if (validName(fields[0])) {
                parsed.push({
                    name: fields[0],
                    path: fields.length > 1 ? fields[1] : "",
                    status: fields.length > 2 ? fields[2] : ""
                })
            }
        }
        return parsed
    }

    function sessionIndexPath(name) {
        return "home/" + ownerUid + "/agent/" + selectedAgent
            + "/session/index/" + name
    }

    function refreshSessions() {
        if (!validName(selectedAgent) || !validUid(ownerUid)) {
            sessions = []
            newSession()
            return
        }
        if (sessionListProcess.running || currentSessionProcess.running)
            return
        sessionIndexTarget = selectedAgent
        sessionListProcess.command = ["ctx", "cat", sessionIndexPath("list")]
        currentSessionProcess.command = ["ctx", "cat", sessionIndexPath("current")]
        sessionListProcess.running = true
        currentSessionProcess.running = true
    }

    function sessionIndexFinished() {
        if (sessionListProcess.running || currentSessionProcess.running)
            return
        if (sessionIndexTarget !== selectedAgent) {
            if (!detailProcess.running && !toolsProcess.running
                    && !ownerProcess.running
                    && detailTarget === selectedAgent)
                refreshSessions()
            return
        }
        const parsed = []
        const lines = String(sessionListOutput.text).split("\n")
        for (let index = 0; index < lines.length; ++index) {
            const name = lines[index].trim()
            if (!validName(name))
                continue
            let duplicate = false
            for (let other = 0; other < parsed.length; ++other)
                duplicate = duplicate || parsed[other] === name
            if (!duplicate)
                parsed.push(name)
        }
        const current = currentSessionOutput.text.trim()
        let next = ""
        if (sessionAgent === selectedAgent && validName(selectedSession)
                && parsed.includes(selectedSession))
            next = selectedSession
        else if (validName(current) && parsed.includes(current))
            next = current
        else if (parsed.length > 0)
            next = parsed[0]
        sessions = parsed
        sessionAgent = selectedAgent
        if (validName(next))
            selectSession(next)
        else
            newSession()
    }

    function selectSession(name) {
        if (running || !validName(name))
            return
        selectedSession = name
        sessionDraft = name
        sessionStatus = "loading"
        loadHistory()
    }

    function newSession() {
        if (running)
            return
        selectedSession = ""
        sessionAgent = selectedAgent
        sessionDraft = "task-" + Date.now().toString(36)
        sessionStatus = "new"
        messages = []
        response = ""
    }

    function loadHistory() {
        if (!validName(selectedAgent) || !validName(selectedSession)
                || historyProcess.running)
            return
        historyTargetAgent = selectedAgent
        historyTargetSession = selectedSession
        historyProcess.command = [
            "ctx", "agent", "history", selectedAgent,
            "--session", selectedSession
        ]
        historyProcess.running = true
    }

    function messageText(value) {
        if (!Array.isArray(value.content))
            return typeof value.content === "string" ? value.content
                : typeof value.text === "string" ? value.text : ""
        let text = ""
        for (let index = 0; index < value.content.length; ++index) {
            const part = value.content[index]
            if (part && typeof part.text === "string")
                text += part.text
        }
        return text
    }

    function parseHistory(text) {
        const parsed = []
        const lines = String(text).split("\n")
        for (let index = 0; index < lines.length; ++index) {
            try {
                const value = JSON.parse(lines[index])
                const content = messageText(value)
                if (content.length > 0 && typeof value.role === "string")
                    parsed.push({ role: value.role, text: content, run: value.run || "" })
            } catch (parseError) {
                if (lines[index].trim().length > 0)
                    error = "A CortexFS history frame could not be read"
            }
        }
        return parsed
    }

    function submit(prompt, session) {
        const cleanPrompt = String(prompt).trim()
        const cleanSession = String(session).trim()
        if (running || !validName(selectedAgent))
            return
        if (!selectedAgentReachable) {
            error = "The selected CortexFS agent is not reachable"
            return
        }
        if (!validName(cleanSession)) {
            error = "Session names may use letters, numbers, dots, dashes, and underscores"
            return
        }
        if (cleanPrompt.length === 0 || cleanPrompt.length > 32768) {
            error = cleanPrompt.length === 0
                ? "Describe a task first" : "Task text exceeds 32 KiB"
            return
        }
        if (selectedSession !== cleanSession) {
            selectedSession = cleanSession
            sessionDraft = cleanSession
            sessionAgent = selectedAgent
            messages = []
        }
        messages = messages.concat([{ role: "user", text: cleanPrompt, run: "" }])
        response = ""
        error = ""
        runId = ""
        model = ""
        pendingApproval = null
        activity = ["Preparing a capability-bound CortexFS run"]
        inputTokens = 0
        outputTokens = 0
        pendingPrompt = cleanPrompt
        pendingSession = cleanSession
        runAgent = selectedAgent
        runSession = cleanSession
        sessionStatus = "starting"
        running = true
        uuidProcess.command = ["uuidgen", "--random"]
        uuidProcess.running = true
    }

    function approve(decision) {
        const request = pendingApproval
        if (!request || !taskSocket.connected
                || !(decision === "allow_once" || decision === "deny"))
            return
        taskSocket.write(JSON.stringify({
            op: "approve", run: request.run, id: request.id, decision: decision
        }) + "\n")
        taskSocket.flush()
        activity = activity.concat([
            (decision === "allow_once" ? "Allowed once · " : "Denied · ")
                + request.name
        ])
        pendingApproval = null
    }

    function cancel() {
        if (!running || !validName(runAgent) || !validName(runId)
                || cancelProcess.running)
            return
        cancelProcess.command = [
            "ctx", "agent", "cancel", runAgent,
            "--session", runSession, "--raw", runId
        ]
        cancelProcess.running = true
        activity = activity.concat(["Cancellation requested"])
    }

    function appendResponse(text) {
        const availableBytes = Math.max(0, 131072 - response.length)
        if (availableBytes > 0)
            response += String(text).slice(0, availableBytes)
    }

    function failRun(message) {
        const wasRunning = running
        requestPending = false
        running = false
        pendingApproval = null
        pendingPrompt = ""
        sessionStatus = "error"
        error = message
        if (taskSocket.connected)
            taskSocket.connected = false
        if (wasRunning)
            completed("error")
    }

    function handleEvent(line) {
        if (String(line).trim().length === 0)
            return
        let value
        try {
            value = JSON.parse(line)
        } catch (parseError) {
            failRun("CortexFS returned an invalid event")
            return
        }
        const eventRun = typeof value.run === "string" ? value.run : ""
        if (eventRun.length > 0 && runId.length > 0 && eventRun !== runId) {
            failRun("CortexFS returned an event for a different run")
            return
        }
        switch (value.type) {
        case "start":
            model = typeof value.model === "string" ? value.model : ""
            sessionStatus = "active"
            activity = activity.concat(["Run started · " + (model || runAgent)])
            break
        case "delta":
            if (typeof value.text === "string")
                appendResponse(value.text)
            break
        case "message":
            if (value.role === "assistant" && response.length === 0)
                appendResponse(messageText(value))
            else if (value.role === "tool")
                activity = activity.concat(["Tool result received"])
            break
        case "tool_call":
            activity = activity.concat(["Running tool · " + (value.name || "unknown")])
            break
        case "approval_request":
            if (validName(value.run) && validName(value.id) && validName(value.name)
                    && validApprovalArgs(value.args))
                pendingApproval = value
            else
                error = "CortexFS sent an invalid approval request"
            break
        case "approval_result":
            activity = activity.concat([
                "Approval " + (value.decision || "resolved") + " · "
                    + (value.name || "tool")
            ])
            break
        case "usage":
            inputTokens = Number(value.input_tokens || 0)
            outputTokens = Number(value.output_tokens || 0)
            break
        case "error":
            error = value.code && value.message === value.code
                ? "CortexFS runtime · " + value.code
                : (value.code ? value.code + " · " : "")
                    + (value.message || "CortexFS task failed")
            sessionStatus = "error"
            activity = activity.concat(["Runtime error · " + error])
            break
        case "done":
            running = false
            pendingApproval = null
            pendingPrompt = ""
            sessionStatus = value.status || "done"
            if (response.length > 0)
                messages = messages.concat([{ role: "assistant", text: response, run: runId }])
            activity = activity.concat(["Run " + sessionStatus])
            taskSocket.connected = false
            completed(sessionStatus)
            refreshAgents()
            break
        }
    }

    Process {
        id: agentProcess
        stdout: StdioCollector { id: agentOutput }
        stderr: StdioCollector { id: agentError }
        onRunningChanged: {
            if (running)
                return
            root.agents = root.parseAgents(agentOutput.text)
            if (root.agents.length === 0) {
                root.loading = false
                root.available = false
                root.error = agentError.text.trim() || "CortexFS is unavailable"
                root.selectedAgent = ""
                return
            }
            root.beginProbes()
        }
    }

    Process {
        id: probeProcess
        stdout: StdioCollector { id: probeOutput }
        stderr: StdioCollector { id: probeError }
        onRunningChanged: {
            if (!running)
                root.completeProbe(probeOutput.text, probeError.text)
        }
    }

    Process {
        id: detailProcess
        stdout: StdioCollector { id: detailOutput }
        onRunningChanged: {
            if (running)
                return
            if (root.detailTarget === root.selectedAgent) {
                const detail = {}
                const lines = String(detailOutput.text).split("\n")
                if (lines.length > 0)
                    detail.status = lines[0].trim()
                for (let index = 1; index < lines.length; ++index) {
                    const separator = lines[index].indexOf("=")
                    if (separator > 0)
                        detail[lines[index].slice(0, separator)]
                            = lines[index].slice(separator + 1)
                }
                root.agentDetail = detail
            }
            root.detailBatchFinished()
        }
    }

    Process {
        id: toolsProcess
        stdout: StdioCollector { id: toolsOutput }
        onRunningChanged: {
            if (running)
                return
            if (root.detailTarget === root.selectedAgent)
                root.tools = root.parseTools(toolsOutput.text)
            root.detailBatchFinished()
        }
    }

    Process {
        id: ownerProcess
        stdout: StdioCollector { id: ownerOutput }
        onRunningChanged: {
            if (running)
                return
            const uid = ownerOutput.text.trim()
            if (root.detailTarget === root.selectedAgent && root.validUid(uid))
                root.ownerUid = uid
            root.detailBatchFinished()
        }
    }

    Process {
        id: sessionListProcess
        stdout: StdioCollector { id: sessionListOutput }
        onRunningChanged: {
            if (!running)
                root.sessionIndexFinished()
        }
    }

    Process {
        id: currentSessionProcess
        stdout: StdioCollector { id: currentSessionOutput }
        onRunningChanged: {
            if (!running)
                root.sessionIndexFinished()
        }
    }

    Process {
        id: historyProcess
        stdout: StdioCollector { id: historyOutput }
        stderr: StdioCollector { id: historyError }
        onRunningChanged: {
            if (running)
                return
            const isCurrent = root.historyTargetAgent === root.selectedAgent
                && root.historyTargetSession === root.selectedSession
            if (isCurrent) {
                if (historyError.text.trim().length > 0)
                    root.error = historyError.text.trim()
                else {
                    root.messages = root.parseHistory(historyOutput.text)
                    root.response = ""
                    root.sessionStatus = root.messages.length > 0 ? "ready" : "empty"
                }
            } else if (root.validName(root.selectedSession)) {
                root.loadHistory()
            }
        }
    }

    Process {
        id: uuidProcess
        stdout: StdioCollector { id: uuidOutput }
        stderr: StdioCollector { id: uuidError }
        onRunningChanged: {
            if (running)
                return
            const entropy = uuidOutput.text.trim().toLowerCase().replace(/-/g, "")
            if (!/^[0-9a-f]{32}$/.test(entropy)) {
                root.failRun(
                    uuidError.text.trim() || "Could not create a secure task ID"
                )
                return
            }
            root.requestPending = true
            root.runId = "ctx-" + entropy
            const prefix = root.cortexRoot === "/" ? "" : root.cortexRoot
            taskSocket.path = prefix + "/agent/" + root.runAgent + ".sock"
            taskSocket.connected = true
        }
    }

    Process {
        id: cancelProcess
        stderr: StdioCollector { id: cancelError }
        onRunningChanged: {
            if (!running && cancelError.text.trim().length > 0)
                root.error = cancelError.text.trim()
        }
    }

    Socket {
        id: taskSocket
        connected: false
        parser: SplitParser {
            splitMarker: "\n"
            onRead: data => root.handleEvent(data)
        }
        onConnectionStateChanged: {
            if (connected && root.requestPending) {
                write(JSON.stringify({
                    op: "send", id: root.runId,
                    session: root.pendingSession, scope: "private",
                    input: root.pendingPrompt
                }) + "\n")
                flush()
                root.requestPending = false
                root.pendingPrompt = ""
            } else if (!connected && root.running && !root.requestPending)
                socketFailureTimer.restart()
        }
        // Quickshell's qmltypes omits QLocalSocket enum registration.
        // qmllint disable signal-handler-parameters
        onError: {
            if (root.running)
                socketFailureTimer.restart()
        }
        // qmllint enable signal-handler-parameters
    }

    Timer {
        id: socketFailureTimer
        interval: 0
        onTriggered: {
            if (root.running)
                root.failRun(
                    root.error
                        || "Cannot connect to the selected CortexFS agent socket"
                )
        }
    }
}
