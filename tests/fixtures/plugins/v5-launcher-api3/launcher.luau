local catalog = noctalia.json.decode(noctalia.readFile("data/catalog.json"))
local rawAccent = noctalia.string.trim(noctalia.getConfig("accent") or "cactus")
local accentPayload = noctalia.json.encode({ accent = rawAccent })
local accent = noctalia.json.decode(accentPayload).accent
local pluginRoot = noctalia.pluginDir()
local latestCount = 0

noctalia.state.watch("queryCount", function(value)
    latestCount = value
end)

function onQuery(text)
    local queryCount = (noctalia.state.get("queryCount") or 0) + 1
    noctalia.state.set("queryCount", queryCount)

    local results = {}
    for _, item in catalog do
        local matches = text == "" or string.find(string.lower(item.title), string.lower(text), 1, true)
        if matches then
            local title = if text == "" then item.title else `{item.title} · {text}`
            table.insert(results, {
                id = item.id,
                title = title,
                subtitle = `{accent} · query #{latestCount}`,
                glyph = "W",
                category = item.category,
            })
        end
    end
    launcher.setResults(text, results)
end

function onActivate(id)
    noctalia.copyToClipboard(`fixture:{id}`, "text/plain")
    noctalia.notify("Weyriva launcher fixture", `Activated {id} from {pluginRoot}`)
    launcher.setQuery("")
end

function onIpc(event, payload)
    if event == "status" then
        local nextQuery = payload.query or ""
        launcher.setQuery(nextQuery)
        return {
            queryCount = noctalia.state.get("queryCount") or 0,
            accent = accent,
        }
    end
    noctalia.notifyError("Weyriva launcher fixture", `Unknown fixture event: {event}`)
    return {
        error = "unknown_event",
    }
end

function onExit(signal, reason)
    noctalia.notify("Weyriva launcher fixture", `Fixture exit {signal}:{reason}`)
end
