function onQuery(query)
    local suffix = query == "" and "world" or query
    launcher.setResults(query, {
        {
            id = "hello",
            title = "Hello, " .. suffix .. "!",
            subtitle = "Weyriva Luau example",
            glyph = "W",
            category = "Examples",
        },
    })
end

function onActivate(id)
    if id == "hello" then
        noctalia.notify("Weyriva", "Hello from the Luau plugin host.")
    end
end
