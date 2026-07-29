function onQuery(text)
    local results = {}
    local title = string.rep("x", 500)
    local subtitle = string.rep("y", 1000)
    for index = 1, 800 do
        table.insert(results, {
            id = `large-{index}`,
            title = title,
            subtitle = subtitle,
        })
    end
    launcher.setResults(text, results)
end

function onActivate(_id)
end
