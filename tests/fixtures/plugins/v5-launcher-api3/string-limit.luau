function onQuery(text)
    launcher.setResults(text, {
        {
            id = "too-long",
            title = string.rep("x", 513),
        },
    })
end

function onActivate(_id)
end
