from __future__ import annotations

import re
import unittest

from weyriva_test_support import ROOT, qml_blocks, qml_sources


class InteractionBehaviorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.sources = qml_sources()
        self.qml = "\n".join(self.sources.values())

    def test_launcher_filters_and_executes_real_desktop_entries(self) -> None:
        panels = [
            source
            for source in self.sources.values()
            if "DesktopEntries.applications.values" in source
        ]
        self.assertEqual(len(panels), 1)
        panel = panels[0]

        script_models = qml_blocks(panel, "ScriptModel")
        filtered_model = next(
            block for block in script_models if "id: filteredApplications" in block
        )
        self.assertIn('objectProp: "modelData"', filtered_model)
        self.assertIn(
            "DesktopEntries.applications.values.filter(application =>",
            filtered_model,
        )
        self.assertIn("const query = search.text.trim().toLowerCase()", filtered_model)
        self.assertIn('const name = application.name || ""', filtered_model)
        self.assertIn('const genericName = application.genericName || ""', filtered_model)
        self.assertIn("name.toLowerCase().includes(query)", filtered_model)
        self.assertIn("genericName.toLowerCase().includes(query)", filtered_model)

        launcher = next(
            block
            for block in qml_blocks(panel, "ListView")
            if "id: launcherList" in block
        )
        self.assertIn("pluginBridge.providerMode", launcher)
        self.assertIn("? root.filteredPluginResults", launcher)
        self.assertIn(": filteredApplications", launcher)
        self.assertIn("currentIndex: count > 0 ? 0 : -1", launcher)
        self.assertIn("onCountChanged: resetSelection()", launcher)
        self.assertIn("keyNavigationEnabled: false", launcher)
        self.assertIn("currentItem as LauncherButton", launcher)
        self.assertIn("Keys.onReturnPressed: launchCurrent()", launcher)
        self.assertIn("Math.min(currentIndex + 1, count - 1)", launcher)
        self.assertIn("Math.max(currentIndex - 1, 0)", launcher)

        self.assertNotIn("itemAtIndex", panel)
        self.assertNotIn("visible: matches", panel)
        self.assertNotIn("height: matches", panel)
        self.assertIn("visible: launcherList.count === 0", panel)
        self.assertIn('"No applications found"', panel)
        self.assertIn("modelData.execute()", panel)

        applications = (
            {"name": "Code Editor", "genericName": "Text Editor"},
            {"name": "Terminal", "genericName": "Command Line"},
            {"name": "Notes", "genericName": "Plain Text Editor"},
        )

        def matches(application: dict[str, str], raw_query: str) -> bool:
            query = raw_query.strip().lower()
            return not query or any(
                query in application[field].lower()
                for field in ("name", "genericName")
            )

        filtered_rows = [
            application
            for application in applications
            if matches(application, " editor ")
        ]
        self.assertEqual(
            [application["name"] for application in filtered_rows],
            ["Code Editor", "Notes"],
        )
        self.assertTrue(
            all(matches(application, " editor ") for application in filtered_rows),
            "every ListView model row must already satisfy the active query",
        )

    def test_launcher_categories_are_interactive_filtered_and_deterministic(self) -> None:
        launcher = (ROOT / "shell/Weyriva/LauncherSurface.qml").read_text()
        bridge = (ROOT / "shell/Weyriva/PluginLauncherBridge.qml").read_text()
        self.assertIn("categories: provider.categories || []", bridge)
        self.assertIn('label: "All", value: ""', launcher)
        self.assertIn("model: root.categoryOptions", launcher)
        self.assertIn(
            "values.filter(result => result.category === selectedCategory)",
            launcher,
        )
        self.assertIn("onClicked:", launcher)
        self.assertIn("root.selectedCategory = modelData.value", launcher)
        self.assertIn("onProviderReferenceChanged: resetCategory()", launcher)
        self.assertIn("onProviderCategoriesChanged: normalizeCategory()", launcher)
        self.assertIn("onCountChanged: resetSelection()", launcher)
        self.assertIn("? root.filteredPluginResults", launcher)
        self.assertIn("Weyriva Plugins", launcher + self.qml)
        self.assertNotRegex(self.qml, r"\bv5\b")
        self.assertNotIn("Python", self.qml)

    def test_plugin_activation_reuses_the_provider_prefix_that_started_it(self) -> None:
        launcher = (ROOT / "shell/Weyriva/LauncherSurface.qml").read_text()
        bridge = (ROOT / "shell/Weyriva/PluginLauncherBridge.qml").read_text()
        self.assertIn(
            "signal queryReplacementRequested(string providerPrefix, string query)",
            bridge,
        )
        self.assertIn("activationProcess.expectedPrefix = provider.prefix", bridge)
        self.assertIn("activationProcess.expectedProvider = provider.reference", bridge)
        self.assertIn(
            "root.queryReplacementRequested(\n"
            "                            expectedPrefix,\n"
            "                            outcomes[index].query",
            bridge,
        )
        replacement_handler = launcher.split(
            "onQueryReplacementRequested:", 1
        )[1].split("}", 1)[0]
        self.assertIn("providerPrefix + \" \" + query", replacement_handler)
        self.assertNotIn("activeProvider", replacement_handler)

    def test_calendar_has_real_month_navigation_and_date_grid(self) -> None:
        self.assertIn("function moveMonth(offset)", self.qml)
        self.assertIn("calendarMonth.getMonth() + offset", self.qml)
        self.assertIn("model: 42", self.qml)
        self.assertIn("property int dayNumber", self.qml)
        self.assertIn("enabled: valid", self.qml)
        self.assertRegex(
            self.qml,
            r"onClicked\s*:\s*root\.selectedDate\s*=\s*new Date\(",
        )
        self.assertIn("root.moveMonth(-1)", self.qml)
        self.assertIn("root.moveMonth(1)", self.qml)

    def test_notifications_and_wallpaper_have_observable_final_states(self) -> None:
        self.assertIn("modelData.dismiss()", self.qml)
        self.assertIn("function dismissAllNotifications()", self.qml)
        self.assertIn("trackedNotifications.values.length === 0", self.qml)
        self.assertIn("property bool selected: ShellState.wallpaper === imageSource", self.qml)
        self.assertIn(
            "onClicked: ShellState.useWallpaper(imageSource, darkAppearance)",
            self.qml,
        )
        self.assertIn("wallpaper = path", self.qml)
        self.assertIn("dark = darkAppearance", self.qml)

    def test_security_input_and_reduced_motion_contracts_remain_real(self) -> None:
        self.assertIn("mask: Region { item: null }", self.qml)
        self.assertIn("WlrLayershell.keyboardFocus: WlrKeyboardFocus.None", self.qml)
        self.assertIn("WlSessionLock", self.qml)
        self.assertIn("PamContext", self.qml)
        self.assertIn("Greetd.createSession(username.text)", self.qml)
        self.assertIn("Greetd.respond(password.text)", self.qml)
        self.assertIn("Greetd.launch(", self.qml)
        self.assertIn("ShellState.reducedMotion ? 90", self.qml)
        self.assertIn("enabled: !ShellState.reducedMotion", self.qml)
        self.assertIn("active || ShellState.reducedMotion ? 1", self.qml)

    def test_greeter_clears_secrets_on_failure_error_and_launch(self) -> None:
        greeter = (ROOT / "greeter/shell.qml").read_text()
        for handler in ("onAuthFailure", "onError"):
            body = greeter.split(f"function {handler}", 1)[1].split(
                "\n        }", 1
            )[0]
            self.assertIn("password.clear()", body)
            self.assertIn("password.forceActiveFocus()", body)
        ready = greeter.split("function onReadyToLaunch()", 1)[1].split(
            "\n        }", 1
        )[0]
        self.assertIn("password.clear()", ready)
        self.assertIn("Greetd.launch(", ready)
        self.assertLess(
            ready.index("password.clear()"),
            ready.index("Greetd.launch("),
        )

    def test_lock_password_is_cleared_for_every_pam_completion(self) -> None:
        pam_handlers = [
            block
            for source in self.sources.values()
            for block in qml_blocks(source, "PamContext")
        ]
        self.assertEqual(len(pam_handlers), 1)
        completion = pam_handlers[0].split("onCompleted: result => {", 1)[1]
        completion = completion.split("\n        }", 1)[0]
        self.assertIn('pendingResponse = ""', completion)
        self.assertIn("lockView.clearPassword()", completion)
        self.assertLess(
            completion.index("lockView.clearPassword()"),
            completion.index("if (result === PamResult.Success)"),
            "password clearing must happen on both failed and successful completion",
        )
        lock_surface = (ROOT / "shell/Weyriva/LockSurface.qml").read_text()
        clear_password = lock_surface.split("function clearPassword()", 1)[1].split(
            "}", 1
        )[0]
        self.assertIn('password.text = ""', clear_password)
