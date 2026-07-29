function onQuery(text)
    launcher.setResults(text, {})
end

function onActivate(id)
    if id == "mime" then
        noctalia.copyToClipboard("fixture", "text/plain\nunsafe")
    elseif id == "mime-limit" then
        noctalia.copyToClipboard("fixture", `text/{string.rep("x", 252)}`)
    elseif id == "notify" then
        noctalia.notify("missing-message")
    elseif id == "title-limit" then
        noctalia.notify(string.rep("x", 513), "fixture")
    elseif id == "message-limit" then
        noctalia.notify("fixture", string.rep("x", 4097))
    elseif id == "default-mime" then
        noctalia.copyToClipboard("fixture")
    end
end
