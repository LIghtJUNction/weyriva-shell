pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

Item {
    id: root
    required property bool active
    property string selectedCategory: ""
    readonly property string providerReference: pluginBridge.activeProvider
        ? pluginBridge.activeProvider.reference : ""
    readonly property var providerCategories: pluginBridge.activeProvider
        ? (pluginBridge.activeProvider.categories || []) : []
    readonly property var categoryOptions: {
        const options = [{label: "All", value: "", glyph: ""}]
        for (let index = 0; index < providerCategories.length; ++index) {
            const category = providerCategories[index]
            if (!category || typeof category.label !== "string"
                    || category.label.length === 0)
                continue
            options.push({
                label: category.label,
                value: category.label,
                glyph: category.glyph || ""
            })
        }
        return options
    }
    readonly property var filteredPluginResults: {
        const values = pluginBridge.results || []
        if (selectedCategory.length === 0)
            return values
        return values.filter(result => result.category === selectedCategory)
    }
    focus: active
    component LauncherButton: Button {
        id: row
        required property var modelData
        property bool pluginResult: false
        property string subtitle: pluginResult
            ? (modelData.subtitle || modelData.category || "")
            : (modelData.genericName || "")
        property bool selected: ListView.isCurrentItem
            || (
                pluginBridge.activationPending
                && pluginBridge.pendingResultId === modelData.id
            )
        implicitHeight: 48
        leftPadding: 12
        rightPadding: 12
        scale: down && !ShellState.reducedMotion ? 0.985 : 1
        function launch() {
            if (pluginResult) {
                pluginBridge.activate(modelData.id)
                return
            }
            modelData.execute()
            ShellState.closeRoute()
        }
        onClicked: launch()
        contentItem: Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                width: parent.width
                text: row.modelData.name || row.modelData.title
                color: Theme.foreground
                elide: Text.ElideRight
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            Text {
                visible: row.subtitle.length > 0
                width: parent.width
                text: row.subtitle
                color: Theme.muted
                elide: Text.ElideRight
                font.pixelSize: 11
            }
        }
        background: Rectangle {
            color: row.down ? Theme.selection
                : row.selected || row.hovered ? Theme.surfaceAlt
                : "transparent"
            radius: 8
            border.width: row.activeFocus ? 2 : 0
            border.color: Theme.foreground
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: 12
                height: 1
                color: Theme.separator
            }
        }
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
    PluginLauncherBridge {
        id: pluginBridge
        input: search.text

        onQueryReplacementRequested: function(providerPrefix, query) {
            search.text = providerPrefix + " " + query
            search.forceActiveFocus()
        }
    }
    function activateSurface() {
        pluginBridge.refreshProviders()
        Qt.callLater(function() { search.forceActiveFocus() })
    }
    function resetCategory() {
        selectedCategory = ""
        Qt.callLater(launcherList.resetSelection)
    }
    function normalizeCategory() {
        const stillDeclared = providerCategories.some(
            category => category && category.label === selectedCategory
        )
        if (selectedCategory.length > 0 && !stillDeclared)
            selectedCategory = ""
        Qt.callLater(launcherList.resetSelection)
    }
    onProviderReferenceChanged: resetCategory()
    onProviderCategoriesChanged: normalizeCategory()
    onActiveChanged: {
        if (active && visible)
            activateSurface()
    }
    onVisibleChanged: {
        if (visible && active)
            activateSurface()
    }
    ColumnLayout {
        anchors.fill: parent
        spacing: 8
        TextField {
            id: search
            Layout.fillWidth: true
            implicitHeight: 52
            placeholderText: "Search applications"
            color: Theme.foreground
            placeholderTextColor: Theme.muted
            font.pixelSize: 17
            leftPadding: 16
            rightPadding: 16
            selectByMouse: true
            onTextChanged: Qt.callLater(launcherList.resetSelection)
            onAccepted: launcherList.launchCurrent()
            Keys.onDownPressed: {
                launcherList.resetSelection()
                launcherList.forceActiveFocus()
            }
            background: Rectangle {
                color: Theme.surfaceAlt
                radius: 12
                border.width: search.activeFocus ? 2 : 1
                border.color: search.activeFocus
                    ? Theme.foreground : Theme.separator
            }
        }
        Text {
            Layout.fillWidth: true
            visible: pluginBridge.providerMode
            text: "Weyriva Plugins"
            color: Theme.muted
            font.pixelSize: 11
        }
        Flickable {
            Layout.fillWidth: true
            implicitHeight: 30
            visible: pluginBridge.providerMode
                && root.categoryOptions.length > 1
            contentWidth: categoryRow.width
            contentHeight: height
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Row {
                id: categoryRow
                height: parent.height
                spacing: 5
                Repeater {
                    model: root.categoryOptions
                    delegate: Button {
                        id: categoryButton
                        required property var modelData
                        height: 28
                        text: modelData.label
                        leftPadding: 10
                        rightPadding: 10
                        scale: down && !ShellState.reducedMotion ? 0.98 : 1
                        Behavior on scale {
                            enabled: !ShellState.reducedMotion
                            NumberAnimation { duration: Theme.motionFast }
                        }
                        onClicked: {
                            root.selectedCategory = modelData.value
                            launcherList.resetSelection()
                            search.forceActiveFocus()
                        }
                        contentItem: Text {
                            text: categoryButton.text
                            color: Theme.foreground
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        background: Rectangle {
                            color: categoryButton.down
                                ? Theme.selection
                                : root.selectedCategory === modelData.value
                                ? Theme.selection
                                : categoryButton.hovered ? Theme.surfaceAlt
                                : "transparent"
                            radius: 7
                            border.width: categoryButton.activeFocus ? 2 : 0
                            border.color: Theme.foreground
                        }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: launcherList
                anchors.fill: parent
                clip: true
                spacing: 0
                currentIndex: count > 0 ? 0 : -1
                keyNavigationEnabled: false
                model: pluginBridge.providerMode
                    ? root.filteredPluginResults : filteredApplications

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
                Keys.onEscapePressed: ShellState.closeRoute()

                delegate: LauncherButton {
                    width: ListView.view.width
                    pluginResult: pluginBridge.providerMode
                }
            }

            Column {
                anchors.centerIn: parent
                visible: launcherList.count === 0
                spacing: 8

                BrandMark {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 84
                    height: 62
                    quiet: true
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: pluginBridge.providerMode && pluginBridge.loading
                        ? "Loading…"
                        : pluginBridge.error.length > 0
                            ? "Plugin unavailable"
                            : pluginBridge.providerMode
                                ? "No plugin results"
                                : "No applications found"
                    color: Theme.foreground
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: pluginBridge.error.length > 0
                    text: pluginBridge.error
                    color: Theme.muted
                    font.pixelSize: 11
                }
            }
        }
    }
}
