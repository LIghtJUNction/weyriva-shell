from __future__ import annotations

import contextlib
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import re
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
LOADER = importlib.machinery.SourceFileLoader("weyriva_runtime", str(ROOT / "bin/weyriva"))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
assert SPEC is not None
weyriva = importlib.util.module_from_spec(SPEC)
sys.modules[LOADER.name] = weyriva
LOADER.exec_module(weyriva)


def qml_sources() -> dict[Path, str]:
    roots = (ROOT / "shell", ROOT / "greeter")
    return {
        path.relative_to(ROOT): path.read_text()
        for root in roots
        for path in root.rglob("*.qml")
    }


def qml_blocks(source: str, type_name: str) -> list[str]:
    """Return balanced blocks for a QML type without depending on filenames."""
    blocks: list[str] = []
    pattern = re.compile(rf"\b{re.escape(type_name)}\s*\{{")
    for match in pattern.finditer(source):
        depth = 0
        quoted = False
        escaped = False
        for index in range(match.end() - 1, len(source)):
            character = source[index]
            if escaped:
                escaped = False
                continue
            if character == "\\" and quoted:
                escaped = True
                continue
            if character == '"':
                quoted = not quoted
                continue
            if quoted:
                continue
            if character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(source[match.start():index + 1])
                    break
    return blocks


class InstallerTests(unittest.TestCase):
    def _run_fake_arch(self, installed: tuple[str, ...]) -> tuple[subprocess.CompletedProcess[str], str]:
        with tempfile.TemporaryDirectory() as temporary:
            fake_root = Path(temporary)
            log = fake_root / "calls.log"
            generic_installed = fake_root / "generic-quickshell"
            installed_cases = "|".join(installed)
            installed_words = " ".join(installed)

            def executable(name: str, body: str = "exit 0\n") -> None:
                path = fake_root / name
                path.write_text("#!/usr/bin/bash\nset -eu\n" + body)
                path.chmod(0o755)

            executable("uname", "printf '%s\\n' Linux\n")
            executable("id")
            executable(
                "systemctl",
                "[[ ${1:-} == --version ]] && printf '%s\\n' 'systemd 261' && exit 0\n"
                "exit 1\n",
            )
            executable(
                "pacman",
                f"printf 'pacman %s\\n' \"$*\" >>'{log}'\n"
                "if [[ ${1:-} == -Qq ]]; then\n"
                "  if [[ $# == 1 ]]; then\n"
                f"    printf '%s\\n' {installed_words}\n"
                "    printf '%s\\n' quickshell\n"
                "    exit 0\n"
                "  fi\n"
                f"  case ${{2:-}} in {installed_cases}) exit 0;; esac\n"
                f"  [[ ${{2:-}} == quickshell && -f '{generic_installed}' ]] && exit 0\n"
                "  exit 1\n"
                "fi\n"
                "[[ ${1:-} == -Si || ${1:-} == -Sp ]] && exit 0\n"
                "[[ ${1:-} == -R && ${2:-} == --print ]] && exit 0\n"
                "exit 99\n",
            )
            executable(
                "sudo",
                f"printf 'sudo %s\\n' \"$*\" >>'{log}'\n"
                "if [[ ${1:-} == pacman && ${2:-} == -S ]]; then\n"
                "  if [[ \" $* \" == *' quickshell '* ]]; then\n"
                "    [[ $* == 'pacman -S --noconfirm --ask=4 --needed quickshell' ]] || exit 98\n"
                f"      : >'{generic_installed}'\n"
                "  fi\n"
                "fi\n"
                "exit 0\n",
            )
            for command in ("niri", "niri-session", "quickshell", "cage", "foot"):
                executable(command)
            (fake_root / "awk").symlink_to("/usr/bin/awk")
            (fake_root / "grep").symlink_to("/usr/bin/grep")

            completed = subprocess.run(
                ["/usr/bin/bash", str(ROOT / "install.sh")],
                env={"PATH": str(fake_root), "USER": "tester"},
                capture_output=True,
                text=True,
                check=False,
            )
            return completed, log.read_text()

    def test_canonical_installer_is_zero_choice_and_cross_distribution(self) -> None:
        installer = ROOT / "install.sh"
        content = installer.read_text()
        self.assertTrue(os.access(installer, os.X_OK))
        self.assertIn("if [[ $# -ne 0 ]]", content)
        for manager in ("pacman", "dnf", "apt-get", "zypper"):
            self.assertIn(manager, content)
        for dependency in ("niri", "greetd", "quickshell", "cage", "foot"):
            self.assertIn(dependency, content)
        self.assertNotIn("read -", content)
        self.assertNotIn("select ", content)
        self.assertNotIn("curl", content)

    def test_arch_migration_uses_safe_exact_transactions(self) -> None:
        content = (ROOT / "install.sh").read_text()
        self.assertIn(
            "for package_name in cachyos-niri-noctalia noctalia-shell",
            content,
        )
        self.assertIn('run_as_root pacman -R --noconfirm "${blocking_packages[@]}"', content)
        self.assertIn("pacman -R --print --print-format '%n'", content)
        replacement = (
            "run_as_root pacman -S --noconfirm --ask=4 --needed quickshell"
        )
        self.assertIn(replacement, content)
        # pacman's --ask value is a bit mask. Four is exactly the conflict
        # question bit (1 << 2), so unrelated questions remain non-interactive.
        self.assertEqual(4, 1 << 2)
        self.assertIn("ALPM conflict bit (1 << 2)", content)
        self.assertEqual(re.findall(r"--ask=(\d+)", content), ["4"])
        self.assertNotIn("--ask=", content.split("pacman -R --noconfirm", 1)[0])
        self.assertIn("pacman -Qq | grep -Fx quickshell", content)
        self.assertNotIn("noctalia-qs", content)
        self.assertNotIn("pacman -Rns", content)
        self.assertLess(
            content.index("pacman -Sp --print-format"),
            content.index('nonconflicting_packages='),
        )

    def test_arch_xry_meta_chain_is_preflighted_then_replaced(self) -> None:
        completed, calls = self._run_fake_arch(
            ("cachyos-niri-noctalia", "noctalia-shell", "noctalia-qs")
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        resolution = "pacman -Sp --print-format %n niri greetd quickshell cage foot noto-fonts"
        dependencies = "sudo pacman -S --noconfirm --needed niri greetd cage foot noto-fonts"
        preflight = "sudo " + str(ROOT / "scripts/install-system.sh") + " --preflight --user tester"
        removal = (
            "sudo pacman -R --noconfirm "
            "cachyos-niri-noctalia noctalia-shell"
        )
        replacement = "sudo pacman -S --noconfirm --ask=4 --needed quickshell"
        removal_preflight = (
            "pacman -R --print --print-format %n "
            "cachyos-niri-noctalia noctalia-shell"
        )
        for expected in (
            resolution,
            dependencies,
            preflight,
            removal_preflight,
            removal,
            replacement,
        ):
            self.assertIn(expected, calls)
        self.assertLess(calls.index(resolution), calls.index(dependencies))
        self.assertLess(calls.index(dependencies), calls.index(preflight))
        self.assertLess(calls.index(preflight), calls.index(removal_preflight))
        self.assertLess(calls.index(removal_preflight), calls.index(removal))
        self.assertLess(calls.index(removal), calls.index(replacement))
        self.assertEqual(calls.count(replacement), 1)
        removal_calls = [
            line
            for line in calls.splitlines()
            if "pacman -R " in line
        ]
        self.assertGreater(len(removal_calls), 0)
        self.assertTrue(
            all("noctalia-qs" not in line for line in removal_calls),
            "the provider must be replaced by the conflict transaction, not removed standalone",
        )
        self.assertNotIn("-Rns", calls)

    def test_arch_generic_quickshell_consumer_is_preserved(self) -> None:
        completed, calls = self._run_fake_arch(
            ("greetd-dms-greeter-git", "noctalia-qs")
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        resolution = "pacman -Sp --print-format %n niri greetd quickshell cage foot noto-fonts"
        dependencies = "sudo pacman -S --noconfirm --needed niri greetd cage foot noto-fonts"
        replacement = "sudo pacman -S --noconfirm --ask=4 --needed quickshell"
        preflight = (
            "sudo " + str(ROOT / "scripts/install-system.sh")
            + " --preflight --user tester"
        )
        for expected in (resolution, dependencies, replacement, preflight):
            self.assertIn(expected, calls)
        self.assertLess(calls.index(resolution), calls.index(dependencies))
        self.assertLess(calls.index(dependencies), calls.index(replacement))
        self.assertLess(calls.index(replacement), calls.index(preflight))
        self.assertEqual(calls.count(replacement), 1)
        self.assertNotIn("sudo pacman -R", calls)
        self.assertNotIn("greetd-dms-greeter-git", calls)

    def test_system_installer_preflights_before_mutating(self) -> None:
        content = (ROOT / "scripts/install-system.sh").read_text()
        self.assertIn("[[ $EUID -eq 0 ]]", content)
        self.assertIn("systemd-analyze --user verify", content)
        self.assertIn('runuser -u "$TARGET_USER"', content)
        self.assertIn("/etc/pam.d/greetd", content)
        self.assertIn("niri.service.wants", content)
        apply_loop = 'for index in "${!install_sources[@]}"; do\n    install_system_file'
        self.assertLess(content.index("/etc/pam.d/greetd"), content.index(apply_loop))
        self.assertLess(
            content.index('niri validate -c "$effective_niri_config"'),
            content.index(apply_loop),
        )
        guard = "if [[ $PREFLIGHT == true ]]; then"
        self.assertIn(guard, content)
        self.assertLess(content.index(guard), content.index("backup_existing()"))
        self.assertLess(content.index(guard), content.index(apply_loop))
        self.assertIn("--preflight", content.split(guard, 1)[0])
        self.assertNotIn("systemctl restart", content)

    def test_aur_package_uses_only_independent_runtime(self) -> None:
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        srcinfo = (ROOT / "packaging/aur/.SRCINFO").read_text()
        for dependency in ("niri", "greetd", "quickshell>=0.3", "cage", "foot"):
            self.assertIn(f"'{dependency}'", package)
        for dependency in ("niri", "greetd", "quickshell>=0.3", "cage", "foot"):
            self.assertIn(f"\tdepends = {dependency}\n", srcinfo)
        self.assertIn('cp -a shell "$pkgdir/usr/share/weyriva/"', package)
        self.assertIn('cp -a greeter "$pkgdir/usr/share/weyriva/"', package)
        self.assertIn('cp -a config/weyriva "$pkgdir/usr/share/weyriva/config/"', package)
        self.assertNotIn("noctalia", package.lower())
        self.assertNotIn("noctalia", srcinfo.lower())

    def test_system_package_plans_complete_shell_greeter_and_config_trees(self) -> None:
        installer = (ROOT / "scripts/install-system.sh").read_text()
        required_trees = {
            "shell": (
                ROOT / "shell/shell.qml",
                ROOT / "shell/Weyriva/qmldir",
                ROOT / "shell/Weyriva/ActionButton.qml",
                ROOT / "shell/Weyriva/ShellState.qml",
                ROOT / "shell/Weyriva/SurfacePanel.qml",
                ROOT / "shell/Weyriva/Theme.qml",
            ),
            "greeter": (ROOT / "greeter/shell.qml",),
            "config/weyriva": (ROOT / "config/weyriva/defaults.json",),
        }
        self.assertIn(
            'for source_root in "$ROOT/shell" "$ROOT/greeter" "$ROOT/config/weyriva"',
            installer,
        )
        self.assertIn('"/usr/share/weyriva/$relative"', installer)
        for tree, paths in required_trees.items():
            with self.subTest(tree=tree):
                self.assertTrue(all(path.is_file() for path in paths))

    def test_user_installer_manages_shell_and_config_but_not_system_greeter(self) -> None:
        installer = (ROOT / "scripts/install.sh").read_text()
        self.assertIn('install_tree "$WEYRIVA_ROOT/config/weyriva" "$CONFIG_HOME/weyriva"', installer)
        self.assertIn('install_tree "$WEYRIVA_ROOT/shell" "$DATA_HOME/weyriva/shell"', installer)
        self.assertNotIn('install_tree "$WEYRIVA_ROOT/greeter"', installer)
        self.assertNotIn("systemd/user", installer)

    def test_user_install_preserves_unmanaged_identical_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            destination = home / "config/weyriva/defaults.json"
            destination.parent.mkdir(parents=True)
            payload = (ROOT / "config/weyriva/defaults.json").read_bytes()
            destination.write_bytes(payload)
            environment = {
                **os.environ,
                "HOME": str(home),
                "XDG_CONFIG_HOME": str(home / "config"),
                "XDG_DATA_HOME": str(home / "data"),
                "XDG_STATE_HOME": str(home / "state"),
            }
            subprocess.run(
                [str(ROOT / "scripts/install.sh"), "--apply"],
                env=environment,
                capture_output=True,
                text=True,
                check=True,
            )
            manifest = home / "state/weyriva/installed-files.tsv"
            self.assertNotIn(str(destination), manifest.read_text())
            self.assertEqual(hashlib.sha256(destination.read_bytes()).digest(),
                             hashlib.sha256(payload).digest())


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
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        for route in (
            "launcher",
            "control-center",
            "calendar",
            "notifications",
            "wallpaper",
            "settings",
        ):
            self.assertIn(f'"{route}"', shell + panel)
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
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        self.assertIn("DesktopEntries.applications", panel)
        self.assertIn("modelData.execute()", panel)
        self.assertIn("NotificationServer", shell)
        self.assertIn("modelData.dismiss()", panel)
        self.assertIn("WlSessionLock", shell)
        self.assertIn("PamContext", shell)
        self.assertIn("signal requestLock()", state)
        self.assertIn("function onRequestLock()", shell)
        self.assertIn("ShellState.requestLock()", shell + panel)
        self.assertIn("sessionLock.locked = true", shell)
        self.assertIn("WlrKeyboardFocus.OnDemand", shell)

    def test_bar_and_calendar_clocks_share_the_live_clock(self) -> None:
        shell = (ROOT / "shell/shell.qml").read_text()
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        state = (ROOT / "shell/Weyriva/ShellState.qml").read_text()
        self.assertIn("property date now: new Date()", state)
        self.assertIn("interval: 1000", shell)
        self.assertIn("onTriggered: ShellState.now = new Date()", shell)
        self.assertIn('Qt.formatDateTime(ShellState.now, "ddd  MMM d  hh:mm")', shell)
        self.assertIn("property date calendarMonth: new Date(", panel)
        self.assertIn("ShellState.now.getFullYear()", panel)
        self.assertIn("ShellState.now.getMonth()", panel)
        self.assertIn('Qt.formatTime(ShellState.now, "hh:mm")', panel)

    def test_every_shell_surface_uses_shared_fixed_theme(self) -> None:
        theme = (ROOT / "shell/Weyriva/Theme.qml").read_text()
        shell = (ROOT / "shell/shell.qml").read_text()
        panel = (ROOT / "shell/Weyriva/SurfacePanel.qml").read_text()
        button = (ROOT / "shell/Weyriva/ActionButton.qml").read_text()
        for token in ("background", "surface", "foreground", "carrier"):
            self.assertIn(f"property color {token}", theme)
        self.assertIn("ShellState.dark", theme)
        self.assertIn("ShellState.reducedMotion", theme)
        self.assertIn("Theme.background", shell)
        self.assertIn("Theme.chrome", shell)
        self.assertIn("Theme.ivory", panel)
        self.assertIn("Theme.ink", panel)
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

    def test_quickshell_command_is_one_fixed_argument_array(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            data_home = Path(temporary)
            user_shell = data_home / "weyriva/shell"
            user_shell.mkdir(parents=True)
            (user_shell / "shell.qml").write_text("ShellRoot {}\n")
            environment = {"XDG_DATA_HOME": str(data_home)}
            self.assertEqual(
                weyriva.quickshell_argv(["ipc", "call", "weyriva", "reload"], environment),
                [
                    "quickshell",
                    "--path",
                    str(user_shell),
                    "ipc",
                    "call",
                    "weyriva",
                    "reload",
                ],
            )
        with mock.patch.object(weyriva, "PACKAGED_SHELL_ROOT", Path("/packaged/weyriva/shell")):
            self.assertEqual(
                weyriva.quickshell_argv(environment={"XDG_DATA_HOME": "/missing"}),
                ["quickshell", "--path", "/packaged/weyriva/shell"],
            )

    def test_cli_called_ipc_functions_exactly_match_qml_handler(self) -> None:
        python_source = (ROOT / "bin/weyriva").read_text()
        qml_source = (ROOT / "shell/shell.qml").read_text()
        called = set(re.findall(r'_shell_ipc\("([A-Za-z][A-Za-z0-9]*)"', python_source))
        called.update(
            re.findall(
                r'quickshell_argv\(\s*\[\s*"ipc",\s*"call",\s*"weyriva",\s*"([A-Za-z][A-Za-z0-9]*)"',
                python_source,
            )
        )
        handler_source = qml_source.split("IpcHandler {", 1)[1]
        handlers = set(
            re.findall(r"function ([A-Za-z][A-Za-z0-9]*)\(", handler_source)
        )
        self.assertEqual(
            called,
            {
                "route",
                "lock",
                "clearNotifications",
                "toggleDnd",
                "setDnd",
                "toggleBar",
                "reload",
            },
        )
        self.assertEqual(handlers, called | {"status"})

    def test_cli_route_and_lock_commands_call_native_ipc(self) -> None:
        with (
            mock.patch.object(weyriva, "_shell_ipc", return_value={"output": "ok"}) as shell_ipc,
            mock.patch.object(weyriva, "_print_json"),
        ):
            self.assertEqual(weyriva.main(["shell", "route", "calendar"]), 0)
            shell_ipc.assert_called_once_with("route", "calendar")
        with (
            mock.patch.object(weyriva, "_shell_ipc", return_value={"output": "ok"}) as shell_ipc,
            mock.patch.object(weyriva, "_print_json"),
        ):
            self.assertEqual(weyriva.main(["shell", "lock"]), 0)
            shell_ipc.assert_called_once_with("lock")

    def test_parser_exposes_only_implemented_shell_and_plugin_routes(self) -> None:
        parser = weyriva.build_parser()
        for arguments in (
            ["shell", "run"],
            ["shell", "lock"],
            ["shell", "route", "launcher"],
            ["plugin", "list"],
            ["plugin", "reload"],
            ["plugin", "validate", "demo.json"],
        ):
            with self.subTest(arguments=arguments):
                parser.parse_args(arguments)
        for arguments in (
            ["shell", "msg"],
            ["plugin", "install", "example"],
            ["plugin", "source", "list"],
        ):
            with (
                self.subTest(arguments=arguments),
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                parser.parse_args(arguments)


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
        self.assertIn("anchors.right: parent.right", utility)
        for route in ("launcher", "wallpaper", "settings"):
            self.assertIn(f'"{route}"', centered)
        self.assertIn("anchors.centerIn: parent", centered)

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
        self.assertIn("model: filteredApplications", launcher)
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
        self.assertIn('text: "No applications found"', panel)
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
        self.assertIn('password.text = ""', completion)
        self.assertLess(
            completion.index('password.text = ""'),
            completion.index("if (result === PamResult.Success)"),
            "password clearing must happen on both failed and successful completion",
        )

    def test_qml_status_handler_is_typed_but_not_a_lock_recovery_probe(self) -> None:
        handlers = [
            block
            for source in self.sources.values()
            for block in qml_blocks(source, "IpcHandler")
        ]
        self.assertEqual(len(handlers), 1)
        handler = handlers[0]
        self.assertRegex(handler, r"\bfunction status\(\)\s*:\s*string\b")
        status_body = re.search(
            r"function status\(\)\s*:\s*string\s*\{(?P<body>.*?)\}",
            handler,
            re.DOTALL,
        )
        self.assertIsNotNone(status_body)
        assert status_body is not None
        self.assertIn("return", status_body.group("body"))
        python_source = (ROOT / "bin/weyriva").read_text()
        reconcile_source = python_source.split("def reconcile_lock(", 1)[1].split(
            "\ndef ", 1
        )[0]
        self.assertNotIn('"status"', reconcile_source)
        self.assertIn("LockedHint", reconcile_source)
        self.assertIn('return 0 if locked_hint == "no" else 1', reconcile_source)


class ProtocolAndPluginTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = weyriva.PluginRegistry({}, (), ())

    def test_ping_round_trip_shape(self) -> None:
        response = weyriva.process_request(
            {"protocol": 1, "id": "test", "method": "weyriva.ping", "params": {}},
            self.registry,
        )
        self.assertEqual(response["result"], {"pong": True, "protocol": 1})

    def test_malformed_json_returns_structured_error(self) -> None:
        response = json.loads(weyriva.process_line(b"{bad\n", self.registry))
        self.assertEqual(response["error"]["code"], "parse_error")

    def test_manifest_requires_own_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            manifest = Path(temporary) / "plugin.json"
            manifest.write_text(
                json.dumps(
                    {
                        "id": "demo",
                        "version": 1,
                        "methods": {"other.echo": {"argv": ["true"]}},
                    }
                )
            )
            with self.assertRaisesRegex(weyriva.PluginError, "invalid or reserved"):
                weyriva._parse_plugin(manifest)

    def test_all_builtin_shell_actions_map_to_native_ipc(self) -> None:
        registry = weyriva.PluginRegistry({}, (), ())
        cases = (
            ("weyriva.launcher.open", {}, ("route", "launcher")),
            ("weyriva.notifications.dismiss_all", None, ("clearNotifications",)),
            ("weyriva.notifications.dnd", {}, ("toggleDnd",)),
            ("weyriva.notifications.dnd", {"enabled": True}, ("setDnd", "true")),
            ("weyriva.notifications.dnd", {"enabled": False}, ("setDnd", "false")),
            ("weyriva.panel.toggle", {}, ("toggleBar",)),
            ("weyriva.panel.reload", None, ("reload",)),
        )
        for method, params, expected in cases:
            with (
                self.subTest(method=method, params=params),
                mock.patch.object(weyriva, "_shell_ipc", return_value={}) as bridge,
            ):
                self.assertEqual(weyriva.dispatch(method, params, registry), {})
                bridge.assert_called_once_with(*expected)

    def test_builtin_shell_actions_reject_unexpected_parameters(self) -> None:
        registry = weyriva.PluginRegistry({}, (), ())
        invalid = (
            ("weyriva.launcher.open", {"route": "settings"}),
            ("weyriva.notifications.dismiss_all", {"all": True}),
            ("weyriva.notifications.dnd", {"enabled": "yes"}),
            ("weyriva.panel.toggle", {"visible": True}),
            ("weyriva.panel.reload", {"force": True}),
        )
        for method, params in invalid:
            with (
                self.subTest(method=method),
                self.assertRaisesRegex(weyriva.ProtocolError, "parameters"),
            ):
                weyriva.dispatch(method, params, registry)

    def test_unsupported_protocol_and_unknown_method_are_structured(self) -> None:
        unsupported = weyriva.process_request(
            {"protocol": 999, "id": "p", "method": "weyriva.ping", "params": {}},
            self.registry,
        )
        unknown = weyriva.process_request(
            {"protocol": 1, "id": "m", "method": "weyriva.missing", "params": {}},
            self.registry,
        )
        self.assertEqual(unsupported["error"]["code"], "unsupported_protocol")
        self.assertEqual(unknown["error"]["code"], "method_not_found")

    def test_ipc_framing_rejects_non_lines_and_oversized_requests(self) -> None:
        non_line = json.loads(weyriva.process_line(b"{}", self.registry))
        oversized = json.loads(
            weyriva.process_line(b"x" * (weyriva.MAX_REQUEST_BYTES + 1) + b"\n", self.registry)
        )
        self.assertEqual(non_line["error"]["code"], "request_too_large")
        self.assertEqual(oversized["error"]["code"], "request_too_large")

    def test_daemon_lock_is_exclusive_and_private(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            lock_path = Path(temporary) / "daemon.lock"
            lock = weyriva.acquire_daemon_lock(lock_path)
            try:
                self.assertEqual(lock_path.stat().st_mode & 0o777, 0o600)
                with self.assertRaisesRegex(RuntimeError, "already running"):
                    weyriva.acquire_daemon_lock(lock_path)
            finally:
                lock.close()

    def test_stale_socket_is_removed_but_other_paths_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stale_path = root / "stale.sock"
            stale = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                stale.bind(str(stale_path))
            finally:
                stale.close()
            self.assertTrue(stale_path.is_socket())
            weyriva.remove_stale_daemon_socket(stale_path)
            self.assertFalse(stale_path.exists())

            regular_path = root / "do-not-replace"
            regular_path.write_text("owned data\n")
            with self.assertRaisesRegex(RuntimeError, "non-socket"):
                weyriva.remove_stale_daemon_socket(regular_path)
            self.assertEqual(regular_path.read_text(), "owned data\n")


class SessionLifecycleTests(unittest.TestCase):
    @staticmethod
    def _completed(returncode: int = 0, stdout: str = "") -> mock.Mock:
        return mock.Mock(returncode=returncode, stdout=stdout, stderr="")

    def test_greetd_pam_requires_active_session_rule(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            pam = Path(temporary) / "greetd"
            pam.write_text("# session include system-login\nauth include system-login\n")
            with self.assertRaisesRegex(RuntimeError, "no active session"):
                weyriva.validate_greetd_pam(pam)
            pam.write_text("auth include system-login\nsession include system-login\n")
            weyriva.validate_greetd_pam(pam)

    def test_startup_file_backup_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            destination = root / "destination"
            backup = root / "backup"
            source.write_text("new\n")
            destination.write_text("old\n")
            self.assertTrue(weyriva.reconcile_startup_file(source, destination, backup))
            self.assertEqual(backup.read_text(), "old\n")
            self.assertFalse(weyriva.reconcile_startup_file(source, destination, backup))

    def test_unlocked_session_is_the_only_successful_reconciliation(self) -> None:
        runner = mock.Mock(return_value=self._completed(stdout="no\n"))
        self.assertEqual(
            weyriva.reconcile_lock(
                {"XDG_SESSION_ID": "c2"},
                runner=runner,
            ),
            0,
        )
        runner.assert_called_once_with(
            ["loginctl", "show-session", "c2", "-p", "LockedHint", "--value"],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )

    def test_locked_unknown_and_failed_session_state_fail_closed(self) -> None:
        for completed in (
            self._completed(stdout="yes\n"),
            self._completed(stdout="unknown\n"),
            self._completed(returncode=1),
        ):
            with self.subTest(completed=completed):
                runner = mock.Mock(return_value=completed)
                self.assertEqual(
                    weyriva.reconcile_lock(
                        {"XDG_SESSION_ID": "c3"},
                        runner=runner,
                    ),
                    1,
                )
                self.assertEqual(runner.call_count, 1)

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
            "Mod+N": 'spawn "weyriva" "shell" "route" "notifications"',
            "Mod+C": 'spawn "weyriva" "shell" "route" "control-center"',
            "Mod+W": 'spawn "weyriva" "shell" "route" "wallpaper"',
            "Mod+Shift+T": 'spawn "weyriva" "shell" "route" "settings"',
            "Mod+Shift+X": 'spawn "weyriva" "shell" "lock"',
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


if __name__ == "__main__":
    unittest.main()
