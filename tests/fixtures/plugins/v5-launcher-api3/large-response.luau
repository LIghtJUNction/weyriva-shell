function onQuery(text)
    local results = {}
    for index = 1, 1454 do
        table.insert(results, {
            id = `kaomoji-{index}`,
            title = `(＾▽＾) {index}`,
            subtitle = "happy, smile, fixture",
            glyph = "mood-smile",
            category = "Happy",
        })
    end
    launcher.setResults(text, results)
end

function onActivate(_id)
end
