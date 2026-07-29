local _value = noctalia.json.parse("{}")

function onQuery(text)
    launcher.setResults(text, {})
end

function onActivate(_id)
end
