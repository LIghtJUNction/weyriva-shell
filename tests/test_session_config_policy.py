from __future__ import annotations

import re
import unittest

from weyriva_test_support import ROOT


class SessionConfigPolicyTests(unittest.TestCase):
    def test_shell_restart_policy_is_bounded(self) -> None:
        shell_unit = (ROOT / "systemd/weyriva-shell.service").read_text()
        failsafe_unit = (ROOT / "systemd/weyriva-session-failsafe.service").read_text()
        self.assertIn("Restart=on-failure", shell_unit)
        self.assertIn("StartLimitBurst=3", shell_unit)
        self.assertIn("StartLimitIntervalSec=30", shell_unit)
        self.assertIn("OnFailure=weyriva-session-failsafe.service", shell_unit)
        self.assertIn("niri msg action quit --skip-confirmation", failsafe_unit)
        self.assertNotIn("WatchdogSec", shell_unit)

    def test_niri_binds_every_shipped_desktop_action_exactly(self) -> None:
        config = (ROOT / "config/niri/config.kdl").read_text()
        expected = {
            "Mod+Space": 'spawn "weyriva" "shell" "route" "launcher"',
            "Mod+A": 'spawn "weyriva" "shell" "route" "tasks"',
            "Mod+Tab": 'spawn "weyriva" "shell" "route" "overview"',
            "Mod+V": 'spawn "weyriva" "shell" "route" "clipboard"',
            "Mod+N": 'spawn "weyriva" "shell" "route" "notifications"',
            "Mod+C": 'spawn "weyriva" "shell" "route" "control-center"',
            "Mod+W": 'spawn "weyriva" "shell" "route" "wallpaper"',
            "Mod+Shift+T": 'spawn "weyriva" "shell" "route" "settings"',
            "Mod+Shift+E": 'spawn "weyriva" "shell" "route" "session"',
            "Mod+Shift+X": 'spawn "weyriva" "shell" "lock"',
            "XF86AudioRaiseVolume": 'spawn "weyriva" "shell" "osd" "volume" "up"',
            "XF86AudioLowerVolume": 'spawn "weyriva" "shell" "osd" "volume" "down"',
            "XF86AudioMute": 'spawn "weyriva" "shell" "osd" "volume" "toggle"',
            "XF86MonBrightnessUp": 'spawn "weyriva" "shell" "osd" "brightness" "up"',
            "XF86MonBrightnessDown": 'spawn "weyriva" "shell" "osd" "brightness" "down"',
            "XF86AudioPlay": 'spawn "playerctl" "play-pause"',
            "XF86AudioNext": 'spawn "playerctl" "next"',
            "XF86AudioPrev": 'spawn "playerctl" "previous"',
            "Mod+Return": 'spawn "foot"',
            "Mod+Q": "close-window",
            "Mod+H": "focus-column-left",
            "Mod+L": "focus-column-right",
            "Mod+J": "focus-window-down",
            "Mod+K": "focus-window-up",
            "Mod+Shift+H": "move-column-left",
            "Mod+Shift+L": "move-column-right",
            "Mod+1": "focus-workspace 1",
            "Mod+2": "focus-workspace 2",
            "Mod+3": "focus-workspace 3",
            "Mod+Shift+1": "move-column-to-workspace 1",
            "Mod+Shift+2": "move-column-to-workspace 2",
            "Mod+Shift+3": "move-column-to-workspace 3",
            "Print": "screenshot",
        }
        binds = config.split("binds {", 1)[1].split("\n}", 1)[0]
        actual = dict(
            re.findall(r"^\s*([^\s{]+)\s*\{\s*([^;\n]+);\s*\}$", binds, re.MULTILINE)
        )
        self.assertEqual(actual, expected)
        self.assertNotIn("spawn-at-startup", config)
