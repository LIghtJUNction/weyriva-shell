local _response = noctalia.http.get("https://example.invalid")

function onQuery(text)
    launcher.setResults(text, {})
end

function onActivate(_id)
end
