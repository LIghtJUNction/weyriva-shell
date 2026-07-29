function onQuery(text)
    launcher.setResults(text, {})
end

function onActivate(_id)
    for index = 1, 257 do
        noctalia.notify("Action limit fixture", `Action {index}`)
    end
end
