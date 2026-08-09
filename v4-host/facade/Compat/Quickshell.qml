pragma Singleton

import QtQuick

QtObject {
    property var detachedCommands: []

    function execDetached(command) {
        if (!Array.isArray(command))
            throw new Error("detached command must be an argv array")
        detachedCommands = detachedCommands.concat([command.slice()])
    }

    function takeDetachedCommands() {
        const commands = detachedCommands
        detachedCommands = []
        return commands
    }
}
