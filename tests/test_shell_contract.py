from __future__ import annotations

import unittest

from weyriva_test_support import ROOT, qml_sources


class IndependentShellTests(unittest.TestCase):
    def test_runtime_sources_have_no_legacy_shell_dependency(self) -> None:
        runtime_roots = (
            ROOT / "bin",
            ROOT / "config",
            ROOT / "greeter",
            ROOT / "packaging",
            ROOT / "shell",
            ROOT / "systemd",
            ROOT / "user-share",
        )
        runtime_paths = [
            path
            for root in runtime_roots
            for path in root.rglob("*")
            if path.is_file()
        ]
        runtime_paths.extend((ROOT / "scripts/install-system.sh", ROOT / "scripts/install.sh"))
        for path in runtime_paths:
            with self.subTest(path=path.relative_to(ROOT)):
                try:
                    content = path.read_text()
                except UnicodeDecodeError:
                    continue
                self.assertNotIn("noctalia", content.lower())
        self.assertFalse((ROOT / "config/noctalia").exists())

        canonical_installer = (ROOT / "install.sh").read_text().lower()
        self.assertIn("cachyos-niri-noctalia", canonical_installer)
        self.assertIn("noctalia-shell", canonical_installer)
        self.assertNotIn("noctalia-qs", canonical_installer)

    def test_shell_owns_required_routes_and_ipc_contract(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        surfaces = "\n".join(qml_sources().values())
        for route in (
            "launcher",
            "control-center",
            "calendar",
            "notifications",
            "wallpaper",
            "settings",
        ):
            self.assertIn(f'"{route}"', shell + surfaces)
        for function in (
            "route",
            "lock",
            "clearNotifications",
            "toggleDnd",
            "setDnd",
            "toggleBar",
            "reload",
        ):
            self.assertIn(f"function {function}(", shell)

    def test_shell_interaction_and_lock_are_real(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        state = (ROOT / "shell/Weyriva/ShellState.qml").read_text()
        surfaces = "\n".join(qml_sources().values())
        self.assertIn("DesktopEntries.applications", surfaces)
        self.assertIn("modelData.execute()", surfaces)
        self.assertIn("NotificationServer", shell)
        self.assertIn("modelData.dismiss()", surfaces)
        self.assertIn("WlSessionLock", shell)
        self.assertIn("PamContext", shell)
        self.assertIn("signal requestLock()", state)
        self.assertIn("function onRequestLock()", shell)
        self.assertIn("ShellState.requestLock()", shell + surfaces)
        self.assertIn("sessionLock.locked = true", shell)
        self.assertIn("WlrKeyboardFocus.OnDemand", shell)

    def test_bar_and_calendar_clocks_share_the_live_clock(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        surfaces = "\n".join(qml_sources().values())
        state = (ROOT / "shell/Weyriva/ShellState.qml").read_text()
        self.assertIn("property date now: new Date()", state)
        self.assertIn("interval: 1000", shell)
        self.assertIn("onTriggered: ShellState.now = new Date()", shell)
        self.assertIn(
            'Qt.formatDateTime(ShellState.now, "ddd  MMM d  hh:mm")',
            shell + surfaces,
        )
        self.assertIn("property date calendarMonth: new Date(", surfaces)
        self.assertIn("ShellState.now.getFullYear()", surfaces)
        self.assertIn("ShellState.now.getMonth()", surfaces)
        self.assertIn('Qt.formatTime(ShellState.now, "hh:mm")', surfaces)

    def test_dark_theme_tokens_drive_every_functional_surface(self) -> None:
        theme = (ROOT / "shell/Weyriva/Theme.qml").read_text()
        shell = (ROOT / "shell/shell.qml").read_text()
        button = (ROOT / "shell/Weyriva/ActionButton.qml").read_text()
        for token in (
            "background",
            "surface",
            "surfaceAlt",
            "foreground",
            "muted",
            "separator",
            "accent",
            "selection",
            "chrome",
        ):
            self.assertIn(f"property color {token}", theme)
        for token in ("surface", "surfaceAlt", "foreground", "muted", "accent"):
            declaration = theme.split(f"property color {token}:", 1)[1].split(
                "\n", 1
            )[0]
            self.assertIn("ShellState.dark", declaration)
        separator = theme.split("property color separator:", 1)[1].split(
            "property color accent:", 1
        )[0]
        self.assertIn("ShellState.dark", separator)
        self.assertIn("ShellState.reducedMotion", theme)
        self.assertIn("Theme.background", shell)
        functional_files = (
            "ActionButton.qml",
            "CalendarSurface.qml",
            "ControlCenterSurface.qml",
            "LauncherSurface.qml",
            "LockSurface.qml",
            "NotificationsSurface.qml",
            "SettingsSurface.qml",
            "SurfaceHeader.qml",
            "SurfacePanel.qml",
            "TopBar.qml",
            "UtilityRow.qml",
            "WallpaperPreview.qml",
            "WallpaperSurface.qml",
        )
        functional = "\n".join(
            (ROOT / "shell/Weyriva" / name).read_text()
            for name in functional_files
        )
        self.assertNotRegex(
            functional,
            r"Theme\.(?:ink|ivory|paper|cactus|carrier)\b",
        )
        for consumer in (
            "color: Theme.surface",
            "color: Theme.foreground",
            "color: Theme.surfaceAlt",
        ):
            self.assertIn(consumer, functional)
        self.assertIn("Theme.selection", functional)
        lock = (ROOT / "shell/Weyriva/LockSurface.qml").read_text()
        self.assertIn("color: Theme.surface", lock)
        self.assertIn("color: Theme.foreground", lock)
        self.assertIn("control.selected", button)
        self.assertIn("control.down", button)
        self.assertIn("Behavior on scale", button)

    def test_background_cannot_capture_input(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        self.assertIn("WlrLayershell.layer: WlrLayer.Background", shell)
        self.assertIn("WlrLayershell.keyboardFocus: WlrKeyboardFocus.None", shell)
        self.assertIn("mask: Region { item: null }", shell)

    def test_greeter_is_weyriva_owned_and_launches_fixed_session(self) -> None:
        greeter = (ROOT / "greeter/shell.qml").read_text()
        greetd = (ROOT / "config/greetd/config.toml").read_text()
        self.assertIn("Quickshell.Services.Greetd", greeter)
        self.assertIn('Greetd.launch(["/usr/bin/weyriva", "session", "start"])', greeter)
        self.assertIn("/usr/bin/cage -s -- /usr/bin/quickshell --path", greetd)
        self.assertIn("/usr/share/weyriva/greeter", greetd)
