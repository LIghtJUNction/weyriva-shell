from __future__ import annotations

import re
import unittest

from weyriva_test_support import ROOT, qml_blocks, qml_sources


class VisualInteractionContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.sources = qml_sources()
        self.qml = "\n".join(self.sources.values())

    def test_flat_anthropic_brand_grammar_has_no_gradient_or_legacy_runtime(self) -> None:
        for color in ("#BCD1CA", "#FAF9F5", "#141413"):
            self.assertIn(color, self.qml)
        self.assertIn("bezierCurveTo", self.qml)
        self.assertIn('lineCap = "round"', self.qml)
        self.assertNotRegex(self.qml, r"\bGradient\s*\{")
        self.assertNotIn("Noctalia", self.qml)
        self.assertNotIn("noctalia", self.qml)

    def test_rejected_visual_patterns_and_dead_controls_do_not_return(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        launcher = (ROOT / "shell/Weyriva/LauncherSurface.qml").read_text()
        controls = (ROOT / "shell/Weyriva/ControlCenterSurface.qml").read_text()
        notifications = (
            ROOT / "shell/Weyriva/NotificationsSurface.qml"
        ).read_text()
        settings = (ROOT / "shell/Weyriva/SettingsSurface.qml").read_text()
        top_bar = (ROOT / "shell/Weyriva/TopBar.qml").read_text()
        lock = (ROOT / "shell/Weyriva/LockSurface.qml").read_text()
        greeter = (ROOT / "greeter/shell.qml").read_text()

        for rejected in (
            "A QUIET PLACE TO BEGIN",
            "WELCOME BACK",
            "Your desktop is covered and waiting",
            "Find your next move",
            "Applications, without the noise",
            "Choose a world",
            "One field. One carrier. One clear mood",
            "Quiet by design",
        ):
            self.assertNotIn(rejected, self.qml)

        self.assertIn("TopBar {", shell)
        self.assertIn("LockSurface {", shell)
        self.assertIn("border.width: 1", top_bar)
        self.assertNotRegex(self.qml, r"border\.width:\s*[3-9]")
        self.assertNotIn("AnthropicCarrier", panel)
        self.assertNotIn("Canvas {", panel)
        self.assertNotIn("GridLayout", controls)
        self.assertIn("UtilityRow", controls)
        self.assertNotIn("subtitle:", controls)
        self.assertNotIn("enabled: false", settings)
        self.assertNotIn("Unavailable", settings)
        self.assertNotIn("Planned", settings)
        self.assertNotIn("radius: 19", notifications)
        self.assertIn("No notifications", notifications)
        self.assertNotIn("Weyriva Plugins ·", launcher)
        self.assertIn("id: search", launcher)
        self.assertIn("implicitHeight: 48", launcher)
        self.assertIn("id: credentialRegion", lock)
        self.assertIn("id: credentialRegion", greeter)
        self.assertIn('context.fillStyle = "#FAF9F5"', greeter)
        self.assertIn("context.bezierCurveTo(", greeter)
        self.assertIn('text: "Password"', lock)
        self.assertIn('text: "Username"', greeter)

    def test_routes_are_owned_by_exactly_one_source_screen(self) -> None:
        state = (ROOT / "shell/Weyriva/ShellState.qml").read_text()
        shell = (ROOT / "shell/shell.qml").read_text()
        top_bar = (ROOT / "shell/Weyriva/TopBar.qml").read_text()

        self.assertIn("property var routeScreen: null", state)
        self.assertIn('property string presentationRoute: ""', state)
        self.assertIn("function openRoute(nextRoute, sourceScreen)", state)
        self.assertIn("routeScreen = sourceScreen", state)
        open_body = state.split("function openRoute(", 1)[1].split("}", 1)[0]
        self.assertLess(
            open_body.index("presentationRoute = nextRoute"),
            open_body.index("route = nextRoute"),
        )
        self.assertIn("function closeRoute()", state)
        close_body = state.split("function closeRoute()", 1)[1].split("}", 1)[0]
        self.assertIn('route = ""', close_body)
        self.assertIn("routeScreen = null", close_body)
        self.assertNotIn("presentationRoute =", close_body)

        self.assertIn("required property var sourceScreen", top_bar)
        self.assertGreaterEqual(
            top_bar.count("ShellState.routeScreen === root.sourceScreen"),
            6,
        )
        self.assertGreaterEqual(top_bar.count("root.sourceScreen"), 12)
        self.assertEqual(
            shell.count("&& ShellState.routeScreen === modelData"),
            2,
            "only the owning utility/centered host may become active",
        )
        self.assertIn("focusable: active", shell)
        self.assertIn("WlrKeyboardFocus.OnDemand", shell)
        self.assertIn(
            "return Quickshell.screens.length > 0 ? Quickshell.screens[0] : null",
            shell,
        )
        self.assertIn(
            "ShellState.toggleRoute(name, root.defaultRouteScreen())",
            shell,
        )
        self.assertIn(
            'ShellState.openRoute("notifications", root.defaultRouteScreen())',
            shell,
        )
        for name, source in self.sources.items():
            if name.name == "ShellState.qml":
                continue
            self.assertNotRegex(source, r"ShellState\.route\s*=(?!=)")

    def test_utility_popovers_use_distinct_bounded_trigger_geometry(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        for property_name in (
            "barWidth",
            "barLeft",
            "barRight",
            "controlX",
            "calendarX",
            "notificationsX",
            "routeX",
        ):
            self.assertIn(f"property real {property_name}", shell)
        self.assertIn("function boundedX(value)", shell)
        self.assertIn("Math.max(10, Math.min(", shell)
        self.assertIn("value, width - utilitySurface.width - 10", shell)
        self.assertIn("x: utilityHost.routeX", shell)
        utility_host = next(
            block
            for block in qml_blocks(shell, "PanelWindow")
            if 'presentation: "utility"' in block
        )
        self.assertGreaterEqual(
            utility_host.count("ShellState.presentationRoute"),
            8,
        )
        for binding in ("routeX", "height:", "sourceOffsetX:", "sourceOrigin:"):
            binding_body = utility_host.split(binding, 1)[1]
            self.assertIn("ShellState.presentationRoute", binding_body)
        self.assertNotIn("anchors.right: parent.right", shell)
        self.assertIn("? -14", shell)
        self.assertIn("? 0 : 14", shell)
        for origin in ("Item.TopLeft", "Item.Top", "Item.TopRight"):
            self.assertIn(origin, shell)
        self.assertIn("required property real sourceOffsetX", panel)
        self.assertIn("required property int sourceOrigin", panel)
        self.assertIn("transformOrigin: sourceOrigin", panel)
        self.assertIn(": root.sourceOffsetX", panel)
        self.assertIn("anchors.centerIn: parent", shell)

    def test_utility_routes_retain_exit_origin_and_interrupt_switch_motion(self) -> None:
        state = (ROOT / "shell/Weyriva/ShellState.qml").read_text()
        shell = (ROOT / "shell/shell.qml").read_text()
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        open_body = state.split("function openRoute(", 1)[1].split("}", 1)[0]
        close_body = state.split("function closeRoute()", 1)[1].split("}", 1)[0]

        self.assertLess(
            open_body.index("presentationRoute = nextRoute"),
            open_body.index("routeScreen = sourceScreen"),
        )
        self.assertLess(
            open_body.index("presentationRoute = nextRoute"),
            open_body.index("route = nextRoute"),
        )
        self.assertNotIn("presentationRoute =", close_body)
        for route in ("control-center", "calendar", "notifications"):
            with self.subTest(transition=f"closed->{route}->closed"):
                self.assertIn(f'"{route}"', shell)
                self.assertIn("presentationRoute", shell)

        self.assertIn("Behavior on x", panel)
        self.assertIn("Behavior on height", panel)
        self.assertGreaterEqual(
            panel.count(
                "root.utility && root.active && !ShellState.reducedMotion"
            ),
            2,
        )
        self.assertIn("SmoothedAnimation", panel)
        self.assertIn("onPresentationRouteChanged:", panel)
        self.assertIn("routeCrossFade.restart()", panel)
        self.assertIn('property: "routeFade"', panel)
        self.assertIn("opacity: root.routeFade", panel)
        self.assertIn("presentationRoute: ShellState.route", shell)

    def test_every_compact_action_button_has_a_delayed_tooltip(self) -> None:
        button = (ROOT / "shell/Weyriva/ActionButton.qml").read_text()
        self.assertIn("ToolTip.visible: compact && hovered", button)
        self.assertIn("ToolTip.text: text", button)
        self.assertRegex(button, r"ToolTip\.delay:\s*[5-9]\d{2}")
        compact_instances = [
            block
            for source in self.sources.values()
            for block in qml_blocks(source, "ActionButton")
            if "compact: true" in block
        ]
        self.assertGreater(len(compact_instances), 8)
        for block in compact_instances:
            with self.subTest(block=block[:80]):
                self.assertRegex(block, r'\btext\s*:\s*"[^"]+"')

    def test_every_enabled_action_button_has_an_action(self) -> None:
        blocks = [
            block
            for source in self.sources.values()
            for block in qml_blocks(source, "ActionButton")
        ]
        self.assertGreater(len(blocks), 12)
        for block in blocks:
            if re.search(r"\benabled\s*:\s*false\b", block):
                continue
            with self.subTest(label=re.search(r'\btext\s*:\s*"([^"]+)', block)):
                self.assertRegex(
                    block,
                    r"\bonClicked\s*:|\bfunction launch\(\)",
                    "an enabled ActionButton must map to an action",
                )

    def test_routes_use_distinct_centered_and_top_right_surface_families(self) -> None:
        windows = [
            block
            for source in self.sources.values()
            for block in qml_blocks(source, "PanelWindow")
        ]
        utility = next(block for block in windows if 'presentation: "utility"' in block)
        centered = next(block for block in windows if 'presentation: "centered"' in block)
        for route in ("control-center", "calendar", "notifications"):
            self.assertIn(f'"{route}"', utility)
        self.assertIn("anchors.top: parent.top", utility)
        self.assertIn("x: utilityHost.routeX", utility)
        for route in ("launcher", "wallpaper", "settings"):
            self.assertIn(f'"{route}"', centered)
        self.assertIn("anchors.centerIn: parent", centered)

    def test_surface_router_and_route_components_remain_bounded(self) -> None:
        component_root = ROOT / "shell/Weyriva"
        panel = (component_root / "SurfacePanel.qml").read_text()
        route_components = (
            "LauncherSurface",
            "ControlCenterSurface",
            "CalendarSurface",
            "NotificationsSurface",
            "WallpaperSurface",
            "SettingsSurface",
        )
        self.assertLessEqual(len((ROOT / "shell/shell.qml").read_text().splitlines()), 260)
        self.assertLessEqual(len((ROOT / "greeter/shell.qml").read_text().splitlines()), 260)
        self.assertLessEqual(len(panel.splitlines()), 180)
        for component in route_components:
            with self.subTest(component=component):
                path = component_root / f"{component}.qml"
                self.assertTrue(path.is_file())
                self.assertLessEqual(len(path.read_text().splitlines()), 320)
                self.assertIn(f"{component} 1.0 {component}.qml", (
                    component_root / "qmldir"
                ).read_text())
                self.assertIn(f"{component} {{", panel)
        self.assertNotIn("DesktopEntries.applications", panel)
        self.assertNotIn("trackedNotifications", panel)
        for component in ("TopBar", "LockSurface", "UtilityRow"):
            path = component_root / f"{component}.qml"
            self.assertTrue(path.is_file())
            self.assertLessEqual(len(path.read_text().splitlines()), 180)
            self.assertIn(
                f"{component} 1.0 {component}.qml",
                (component_root / "qmldir").read_text(),
            )
