pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    property date calendarMonth: new Date(
        ShellState.now.getFullYear(),
        ShellState.now.getMonth(),
        1
    )
    property date selectedDate: ShellState.now
    readonly property int firstWeekday: calendarMonth.getDay()
    readonly property int daysInMonth: new Date(
        calendarMonth.getFullYear(),
        calendarMonth.getMonth() + 1,
        0
    ).getDate()

    spacing: 12

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

    RowLayout {
        Layout.fillWidth: true

        Text {
            Layout.fillWidth: true
            text: Qt.formatDate(root.calendarMonth, "MMMM yyyy")
            color: Theme.foreground
            font.pixelSize: 18
            font.weight: Font.DemiBold
            font.letterSpacing: -0.25
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
        rowSpacing: 3
        columnSpacing: 3

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
                implicitHeight: 36
                enabled: valid
                scale: down && !ShellState.reducedMotion ? 0.9 : 1
                opacity: enabled ? (down ? 0.76 : 1) : 0

                Behavior on scale {
                    enabled: !dayCell.down && !ShellState.reducedMotion
                    NumberAnimation {
                        duration: Theme.motionFast
                        easing.type: Easing.OutCubic
                    }
                }

                onClicked: root.selectedDate = new Date(
                    root.calendarMonth.getFullYear(),
                    root.calendarMonth.getMonth(),
                    dayNumber
                )

                contentItem: Text {
                    text: dayCell.valid ? dayCell.dayNumber : ""
                    color: dayCell.selected ? Theme.onSelection
                        : dayCell.today ? Theme.accent : Theme.foreground
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    font.pixelSize: 13
                    font.weight: dayCell.today || dayCell.selected
                        ? Font.Bold : Font.Normal
                }

                background: Rectangle {
                    color: dayCell.selected ? Theme.selection
                        : dayCell.down ? Theme.pressed
                        : dayCell.hovered ? Theme.hover : "transparent"
                    radius: 18
                    border.width: dayCell.activeFocus || dayCell.today ? 2 : 0
                    border.color: dayCell.activeFocus
                        ? Theme.focusRing : Theme.accent
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
            color: Theme.foreground
            font.pixelSize: 30
            font.weight: Font.Bold
            font.letterSpacing: -0.7
        }
    }
}
