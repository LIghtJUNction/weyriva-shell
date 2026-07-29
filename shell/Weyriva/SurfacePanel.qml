pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

Item {
    id: root
    required property var notificationServer
    required property string presentation
    required property bool active

    property date calendarMonth: new Date(
        ShellState.now.getFullYear(),
        ShellState.now.getMonth(),
        1
    )
    property date selectedDate: ShellState.now

    readonly property bool utility: presentation === "utility"
    readonly property string title: {
        switch (ShellState.route) {
        case "launcher": return "Find your next move"
        case "control-center": return "Control center"
        case "calendar": return "Calendar"
        case "notifications": return "Notifications"
        case "wallpaper": return "Choose a world"
        case "settings": return "Weyriva settings"
        default: return "Weyriva"
        }
    }
    readonly property int firstWeekday: calendarMonth.getDay()
    readonly property int daysInMonth: new Date(
        calendarMonth.getFullYear(),
        calendarMonth.getMonth() + 1,
        0
    ).getDate()

    component LauncherButton: ActionButton {
        required property var modelData

        function launch() {
            modelData.execute()
            ShellState.route = ""
        }

        onClicked: launch()
    }

    ScriptModel {
        id: filteredApplications
        objectProp: "modelData"
        values: {
            const query = search.text.trim().toLowerCase()
            return DesktopEntries.applications.values.filter(application => {
                if (query.length === 0)
                    return true
                const name = application.name || ""
                const genericName = application.genericName || ""
                return name.toLowerCase().includes(query)
                    || genericName.toLowerCase().includes(query)
            })
        }
    }

    opacity: active ? 1 : 0
    scale: active || ShellState.reducedMotion ? 1 : (utility ? 1 : 0.985)
    focus: active
    Keys.onEscapePressed: ShellState.route = ""

    transform: Translate {
        id: slide
        x: root.active || ShellState.reducedMotion ? 0 : (root.utility ? 28 : 0)
        y: root.active || ShellState.reducedMotion ? 0 : (root.utility ? -8 : 18)

        Behavior on x {
            enabled: !ShellState.reducedMotion
            SmoothedAnimation { duration: Theme.motionPanel; velocity: 260 }
        }
        Behavior on y {
            enabled: !ShellState.reducedMotion
            SmoothedAnimation { duration: Theme.motionPanel; velocity: 260 }
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: ShellState.reducedMotion ? 90 : 155
            easing.type: Easing.OutCubic
        }
    }
    Behavior on scale {
        enabled: !ShellState.reducedMotion
        SmoothedAnimation { duration: Theme.motionPanel; velocity: 2.5 }
    }

    onActiveChanged: {
        if (!active)
            return
        forceActiveFocus()
        if (ShellState.route === "launcher")
            Qt.callLater(function() { search.forceActiveFocus() })
    }

    function moveMonth(offset) {
        calendarMonth = new Date(
            calendarMonth.getFullYear(),
            calendarMonth.getMonth() + offset,
            1
        )
    }

    function showToday() {
        selectedDate = ShellState.now
        calendarMonth = new Date(
            ShellState.now.getFullYear(),
            ShellState.now.getMonth(),
            1
        )
    }

    function dismissAllNotifications() {
        const values = notificationServer.trackedNotifications.values
        for (let index = values.length - 1; index >= 0; --index)
            values[index].dismiss()
    }

    Canvas {
        id: carrier
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const context = getContext("2d")
            context.clearRect(0, 0, width, height)
            context.fillStyle = Theme.ivory
            context.strokeStyle = Theme.ink
            context.lineWidth = root.utility ? 4 : 5
            context.lineJoin = "round"

            const inset = 5
            context.beginPath()
            context.moveTo(inset + width * 0.04, inset + height * 0.02)
            context.bezierCurveTo(
                width * 0.24, -1,
                width * 0.77, inset,
                width - inset - width * 0.025, inset + height * 0.05
            )
            context.bezierCurveTo(
                width + 1, height * 0.28,
                width - inset, height * 0.74,
                width - inset - width * 0.05, height - inset
            )
            context.bezierCurveTo(
                width * 0.72, height + 1,
                width * 0.25, height - inset,
                inset + width * 0.03, height - inset - height * 0.05
            )
            context.bezierCurveTo(
                -1, height * 0.73,
                inset, height * 0.25,
                inset + width * 0.04, inset + height * 0.02
            )
            context.closePath()
            context.fill()
            context.stroke()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: root.utility ? 27 : 38
        anchors.rightMargin: root.utility ? 27 : 38
        anchors.topMargin: root.utility ? 25 : 32
        anchors.bottomMargin: root.utility ? 27 : 34
        spacing: root.utility ? 14 : 20

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Column {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: root.title
                    color: Theme.ink
                    font.pixelSize: root.utility ? 22 : 30
                    font.weight: Font.Bold
                    font.letterSpacing: root.utility ? -0.2 : -0.7
                }
                Text {
                    visible: !root.utility
                    text: ShellState.route === "launcher"
                        ? "Applications, without the noise."
                        : ShellState.route === "wallpaper"
                            ? "One field. One carrier. One clear mood."
                            : "Appearance and access, kept explicit."
                    color: Theme.muted
                    font.pixelSize: 13
                }
            }

            ActionButton {
                glyph: "×"
                text: "Close"
                compact: true
                onClicked: ShellState.route = ""
            }
        }

        ColumnLayout {
            visible: ShellState.route === "launcher"
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            TextField {
                id: search
                Layout.fillWidth: true
                implicitHeight: 58
                placeholderText: "Search applications"
                color: Theme.ink
                placeholderTextColor: Theme.muted
                font.pixelSize: 17
                leftPadding: 20
                rightPadding: 20
                selectByMouse: true
                onTextChanged: Qt.callLater(launcherList.resetSelection)
                onAccepted: launcherList.launchCurrent()
                Keys.onDownPressed: {
                    launcherList.resetSelection()
                    launcherList.forceActiveFocus()
                }
                background: Rectangle {
                    color: Theme.paper
                    radius: 20
                    border.width: search.activeFocus ? 3 : 1
                    border.color: Theme.ink
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: launcherList
                    anchors.fill: parent
                    clip: true
                    spacing: 5
                    currentIndex: count > 0 ? 0 : -1
                    keyNavigationEnabled: false
                    model: filteredApplications

                    function resetSelection() {
                        currentIndex = count > 0 ? 0 : -1
                    }

                    function launchCurrent() {
                        const item = currentItem as LauncherButton
                        if (item)
                            item.launch()
                    }

                    onCountChanged: resetSelection()
                    Keys.onDownPressed: {
                        if (count > 0)
                            currentIndex = Math.min(currentIndex + 1, count - 1)
                    }
                    Keys.onUpPressed: {
                        if (count > 0)
                            currentIndex = Math.max(currentIndex - 1, 0)
                    }
                    Keys.onReturnPressed: launchCurrent()
                    Keys.onEnterPressed: launchCurrent()
                    Keys.onEscapePressed: ShellState.route = ""

                    delegate: LauncherButton {
                        id: application

                        width: ListView.view.width
                        text: modelData.name
                        subtitle: modelData.genericName
                        selected: ListView.isCurrentItem
                    }
                }

                Column {
                    anchors.centerIn: parent
                    visible: launcherList.count === 0
                    spacing: 12

                    BrandMark {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 150
                        height: 112
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "No applications found"
                        color: Theme.ink
                        font.pixelSize: 18
                        font.weight: Font.Bold
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Try a shorter or different name."
                        color: Theme.muted
                        font.pixelSize: 13
                    }
                }
            }
        }

        GridLayout {
            visible: ShellState.route === "control-center"
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: 2
            columnSpacing: 8
            rowSpacing: 8

            ActionButton {
                Layout.fillWidth: true
                Layout.fillHeight: true
                selected: ShellState.doNotDisturb
                glyph: "●"
                text: ShellState.doNotDisturb ? "Focus on" : "Focus off"
                subtitle: "Do not disturb"
                onClicked: ShellState.doNotDisturb = !ShellState.doNotDisturb
            }
            ActionButton {
                Layout.fillWidth: true
                Layout.fillHeight: true
                selected: ShellState.dark
                glyph: ShellState.dark ? "◐" : "○"
                text: ShellState.dark ? "Dark field" : "Light field"
                subtitle: "Appearance"
                onClicked: ShellState.setDark(!ShellState.dark)
            }
            ActionButton {
                Layout.fillWidth: true
                Layout.fillHeight: true
                glyph: "›_"
                text: "Terminal"
                subtitle: "Open Foot"
                onClicked: ShellState.launch(["foot"])
            }
            ActionButton {
                Layout.fillWidth: true
                Layout.fillHeight: true
                glyph: "▣"
                text: "Lock"
                subtitle: "Secure this session"
                onClicked: ShellState.requestLock()
            }
        }

        ColumnLayout {
            visible: ShellState.route === "calendar"
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: Qt.formatDate(root.calendarMonth, "MMMM yyyy")
                    color: Theme.ink
                    font.pixelSize: 18
                    font.weight: Font.DemiBold
                }
                ActionButton {
                    glyph: "‹"
                    text: "Previous month"
                    compact: true
                    onClicked: root.moveMonth(-1)
                }
                ActionButton {
                    glyph: "›"
                    text: "Next month"
                    compact: true
                    onClicked: root.moveMonth(1)
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 7
                rowSpacing: 4
                columnSpacing: 4

                Repeater {
                    model: ["S", "M", "T", "W", "T", "F", "S"]
                    delegate: Text {
                        required property string modelData
                        Layout.fillWidth: true
                        text: modelData
                        color: Theme.muted
                        horizontalAlignment: Text.AlignHCenter
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                }

                Repeater {
                    model: 42
                    delegate: Button {
                        id: dayCell
                        required property int index
                        property int dayNumber: index - root.firstWeekday + 1
                        property bool valid: dayNumber > 0
                            && dayNumber <= root.daysInMonth
                        property bool today: valid
                            && dayNumber === ShellState.now.getDate()
                            && root.calendarMonth.getMonth()
                                === ShellState.now.getMonth()
                            && root.calendarMonth.getFullYear()
                                === ShellState.now.getFullYear()
                        property bool selected: valid
                            && dayNumber === root.selectedDate.getDate()
                            && root.calendarMonth.getMonth()
                                === root.selectedDate.getMonth()
                            && root.calendarMonth.getFullYear()
                                === root.selectedDate.getFullYear()

                        Layout.fillWidth: true
                        implicitHeight: 34
                        enabled: valid
                        scale: down && !ShellState.reducedMotion ? 0.92 : 1
                        onClicked: root.selectedDate = new Date(
                            root.calendarMonth.getFullYear(),
                            root.calendarMonth.getMonth(),
                            dayNumber
                        )

                        contentItem: Text {
                            text: dayCell.valid ? dayCell.dayNumber : ""
                            color: dayCell.selected ? Theme.ivory : Theme.ink
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            font.pixelSize: 13
                            font.weight: dayCell.today || dayCell.selected
                                ? Font.Bold : Font.Normal
                        }
                        background: Rectangle {
                            color: dayCell.selected ? Theme.ink
                                : dayCell.today ? Theme.cactus
                                : dayCell.hovered ? Theme.paper : "transparent"
                            radius: 12
                            border.width: dayCell.activeFocus ? 2 : 0
                            border.color: Theme.ink
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                ActionButton {
                    text: "Today"
                    selected: Qt.formatDate(root.selectedDate, "yyyyMMdd")
                        === Qt.formatDate(ShellState.now, "yyyyMMdd")
                    onClicked: root.showToday()
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: Qt.formatTime(ShellState.now, "hh:mm")
                    color: Theme.ink
                    font.pixelSize: 29
                    font.weight: Font.Bold
                    font.letterSpacing: -0.5
                }
            }
        }

        Item {
            visible: ShellState.route === "notifications"
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                visible: root.notificationServer.trackedNotifications.values.length > 0
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        Layout.fillWidth: true
                        text: root.notificationServer.trackedNotifications.values.length
                            + " recent"
                        color: Theme.muted
                        font.pixelSize: 12
                    }
                    ActionButton {
                        text: "Clear all"
                        onClicked: root.dismissAllNotifications()
                    }
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 8
                    model: root.notificationServer.trackedNotifications

                    delegate: Rectangle {
                        required property var modelData
                        width: ListView.view.width
                        height: 86
                        color: Theme.paper
                        radius: 19
                        border.width: 1
                        border.color: Theme.separator

                        Column {
                            anchors.left: parent.left
                            anchors.right: dismissButton.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 16
                            anchors.rightMargin: 12
                            spacing: 4
                            Text {
                                width: parent.width
                                text: modelData.summary || modelData.appName
                                color: Theme.ink
                                elide: Text.ElideRight
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                            }
                            Text {
                                width: parent.width
                                text: modelData.body
                                textFormat: Text.PlainText
                                color: Theme.muted
                                maximumLineCount: 2
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                                font.pixelSize: 12
                            }
                        }

                        ActionButton {
                            id: dismissButton
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 12
                            glyph: "×"
                            text: "Dismiss"
                            compact: true
                            onClicked: modelData.dismiss()
                        }
                    }
                }
            }

            Column {
                anchors.centerIn: parent
                visible: root.notificationServer.trackedNotifications.values.length === 0
                spacing: 10

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 180
                    height: 138
                    radius: 38
                    color: Theme.cactus
                    BrandMark { anchors.fill: parent }
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Quiet by design"
                    color: Theme.ink
                    font.pixelSize: 17
                    font.weight: Font.Bold
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "New notices will gather here."
                    color: Theme.muted
                    font.pixelSize: 12
                }
            }
        }

        ColumnLayout {
            visible: ShellState.route === "wallpaper"
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 16

            GridLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                columns: 2
                columnSpacing: 16

                WallpaperPreview {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: "Cactus daylight"
                    imageSource:
                        "/usr/share/weyriva/wallpapers/light/weyriva-cactus.png"
                    darkAppearance: false
                }
                WallpaperPreview {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: "Cactus after dark"
                    imageSource:
                        "/usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png"
                    darkAppearance: true
                }
            }

            Text {
                Layout.fillWidth: true
                text: "Selecting a field also sets the matching light or dark appearance."
                color: Theme.muted
                wrapMode: Text.Wrap
                font.pixelSize: 12
            }
        }

        ColumnLayout {
            visible: ShellState.route === "settings"
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            Text {
                text: "Appearance"
                color: Theme.ink
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            RowLayout {
                Layout.fillWidth: true
                ActionButton {
                    Layout.fillWidth: true
                    selected: !ShellState.dark
                    glyph: "○"
                    text: "Light"
                    subtitle: "Cactus and ivory"
                    onClicked: ShellState.setDark(false)
                }
                ActionButton {
                    Layout.fillWidth: true
                    selected: ShellState.dark
                    glyph: "●"
                    text: "Dark"
                    subtitle: "Ink and cactus"
                    onClicked: ShellState.setDark(true)
                }
            }

            Text {
                text: "Accessibility"
                color: Theme.ink
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            ActionButton {
                Layout.fillWidth: true
                selected: ShellState.reducedMotion
                text: ShellState.reducedMotion
                    ? "Reduced motion enabled" : "Reduced motion disabled"
                subtitle: ShellState.reducedMotion
                    ? "Panels cross-fade without travel"
                    : "Panels follow their source"
                onClicked: ShellState.reducedMotion = !ShellState.reducedMotion
            }

            Text {
                text: "Planned"
                color: Theme.ink
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            ActionButton {
                Layout.fillWidth: true
                enabled: false
                text: "Plugin compatibility"
                subtitle: "Unavailable in this milestone"
            }
            ActionButton {
                Layout.fillWidth: true
                enabled: false
                text: "Automatic day and night schedule"
                subtitle: "Unavailable in this milestone"
            }
            Item { Layout.fillHeight: true }
        }
    }
}
