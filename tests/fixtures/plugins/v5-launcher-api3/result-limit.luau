function onQuery(text)
    local results = {}
    for index = 1, 2001 do
        table.insert(results, {
            id = `result-{index}`,
            title = `Result {index}`,
        })
    end
    launcher.setResults(text, results)
end

function onActivate(_id)
end
