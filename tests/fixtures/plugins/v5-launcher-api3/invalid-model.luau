function onQuery(text)
    launcher.setResults(text, {
        {
            id = "",
            title = "This result has an invalid empty id",
        },
    })
end

function onActivate(_id)
end
