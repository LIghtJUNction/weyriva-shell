local requestedPath = noctalia.getConfig("path")
local content = noctalia.readFile(requestedPath)

function onQuery(text)
    launcher.setResults(text, {
        {
            id = "read",
            title = content,
            subtitle = text,
        },
    })
end

function onActivate(_id)
end
