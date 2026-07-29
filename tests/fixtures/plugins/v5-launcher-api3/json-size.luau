local generatedBytes = noctalia.getConfig("generated_bytes")
local raw = if type(generatedBytes) == "number"
    then `[{string.rep(" ", generatedBytes)}]`
    else noctalia.readFile("large-catalog.json")
local parsed = noctalia.json.decode(raw)

function onQuery(text)
    launcher.setResults(`decoded:{text}`, {
        {
            id = "json-size",
            title = tostring(#parsed),
            subtitle = tostring(#raw),
        },
    })
end

function onActivate(_id)
end
