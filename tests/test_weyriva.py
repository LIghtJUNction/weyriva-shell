from __future__ import annotations

import contextlib
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import threading
import tomllib
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


class InstallerTests(unittest.TestCase):
    def test_canonical_installer_is_executable_and_uses_all_supported_managers(self) -> None:
        installer = ROOT / "install.sh"
        content = installer.read_text()
        self.assertTrue(installer.is_file())
        self.assertTrue(os.access(installer, os.X_OK))
        for manager in ("pacman", "dnf", "apt-get", "zypper"):
            self.assertIn(manager, content)
        self.assertIn("pacman -S --noconfirm --needed", content)
        self.assertIn("noctalia", content)
        self.assertIn("noctalia-greeter", content)
        self.assertIn('pacman -Si "$package_name"', content)
        self.assertIn("run_as_invoking_user", content)
        self.assertIn("SUDO_USER", content)
        for helper in ("paru", "yay"):
            self.assertIn(helper, content)
        self.assertNotIn("curl", content)
        for retired in ("waybar", "fuzzel", "mako", "swaybg", "swaylock", "swayidle"):
            self.assertNotRegex(content, rf"pacman -S [^\n]*\b{retired}\b")

    def test_canonical_installer_applies_system_install_after_runtime_gating(self) -> None:
        content = (ROOT / "install.sh").read_text()
        self.assertIn('scripts/install-system.sh" --user "$desktop_user"', content)
        self.assertLess(
            content.index('command -v "$command_name"'),
            content.index('scripts/install-system.sh" --user "$desktop_user"'),
        )
        self.assertLess(content.index("systemd_version="), content.index("case ${managers[0]}"))
        self.assertLess(
            content.index('SCRIPT_DIR=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)'),
            content.index("case ${managers[0]}"),
        )
        self.assertLess(
            content.index('"$SCRIPT_DIR/scripts/install-system.sh"'),
            content.index("case ${managers[0]}"),
        )
        self.assertLess(
            content.index('run_as_invoking_user "$aur_helper" -Si "$package_name"'),
            content.index("run_as_root pacman -S --noconfirm --needed"),
        )
        self.assertIn("repository_packages=(niri greetd foot noto-fonts)", content)
        for manager in ("dnf install", "apt-get install", "zypper --non-interactive install"):
            line = next(line for line in content.splitlines() if manager in line)
            self.assertIn("greetd", line)
        self.assertNotIn("systemctl --user stop", content)

    def test_arch_aur_plan_failure_happens_before_pacman_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            log = root / "calls.log"

            def executable(name: str, body: str) -> None:
                path = root / name
                path.write_text("#!/usr/bin/bash\nset -eu\n" + body)
                path.chmod(0o755)

            executable("uname", "printf '%s\\n' Linux\n")
            executable("id", "exit 0\n")
            executable(
                "systemctl",
                "[[ ${1:-} == --version ]] && printf '%s\\n' 'systemd 261' && exit 0\nexit 1\n",
            )
            executable(
                "pacman",
                f"printf 'pacman %s\\n' \"$*\" >>'{log}'\n"
                "if [[ ${1:-} == -Si ]]; then\n"
                "  case ${2:-} in niri|greetd|foot|noto-fonts) exit 0;; *) exit 1;; esac\n"
                "fi\n"
                "exit 99\n",
            )
            executable(
                "paru",
                f"printf 'paru %s\\n' \"$*\" >>'{log}'\n"
                "[[ ${1:-} == -Si && ${2:-} == noctalia ]] && exit 0\n"
                "exit 1\n",
            )
            executable("sudo", "exit 99\n")
            (root / "awk").symlink_to("/usr/bin/awk")
            completed = subprocess.run(
                ["/usr/bin/bash", str(ROOT / "install.sh")],
                env={"PATH": str(root), "USER": "tester"},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(completed.returncode, 0)
            calls = log.read_text()
            self.assertIn("paru -Si noctalia", calls)
            self.assertIn("paru -Si noctalia-greeter", calls)
            self.assertNotIn("pacman -S --", calls)

    def test_system_installer_is_root_owned_bounded_and_never_restarts_login(self) -> None:
        installer = ROOT / "scripts/install-system.sh"
        content = installer.read_text()
        self.assertTrue(os.access(installer, os.X_OK))
        self.assertIn("[[ $EUID -eq 0 ]]", content)
        self.assertIn("systemd_version", content)
        self.assertIn("systemd-analyze --user verify", content)
        self.assertIn("systemctl cat greetd.service", content)
        self.assertIn("noctalia config validate", content)
        self.assertIn("/usr/bin/weyriva startup ensure --user", content)
        self.assertIn("niri.service.wants", content)
        self.assertIn("/etc/pam.d/greetd", content)
        self.assertIn("getent group greeter", content)
        apply_loop = 'for index in "${!install_sources[@]}"; do\n    install_system_file'
        self.assertLess(
            content.index("/etc/pam.d/greetd"),
            content.index(apply_loop),
        )
        self.assertLess(
            content.index('fail "refusing to replace unexpected niri wants entry'),
            content.index(apply_loop),
        )
        self.assertLess(content.index("startup_backup_root="), content.index(apply_loop))
        self.assertLess(
            content.index('niri validate -c "$effective_niri_config"'),
            content.index(apply_loop),
        )
        self.assertNotIn("systemctl restart", content)
        self.assertNotIn("systemctl --user start", content)

    def test_user_installer_never_installs_system_service_overrides(self) -> None:
        content = (ROOT / "scripts/install.sh").read_text()
        self.assertNotIn('install_tree "$WEYRIVA_ROOT/systemd"', content)
        self.assertNotIn("SYSTEMD_HOME", content)
        self.assertIn("remove_obsolete_managed", content)

    def test_user_installer_retires_a_previously_managed_service_override(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            override = home / "config/systemd/user/weyriva-shell.service"
            override.parent.mkdir(parents=True)
            payload = (ROOT / "systemd/weyriva-shell.service").read_bytes()
            override.write_bytes(payload)
            state = home / "state/weyriva/installed-files.tsv"
            state.parent.mkdir(parents=True)
            digest = hashlib.sha256(payload).hexdigest()
            state.write_text(f"{digest}\t{override}\n")
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
            self.assertFalse(override.exists())
            self.assertNotIn(str(override), state.read_text())

    def test_aur_package_uses_noctalia_as_the_only_shell_engine(self) -> None:
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        self.assertNotIn("ttf-nerd-fonts-symbols-mono", package)
        self.assertIn("'noctalia'", package)
        self.assertIn("'noctalia-greeter'", package)
        self.assertIn("'systemd>=254'", package)
        self.assertIn("'greetd'", package)
        self.assertNotIn("'greetd-tuigreet'", package)
        srcinfo = (ROOT / "packaging/aur/.SRCINFO").read_text()
        self.assertIn("\tdepends = greetd\n", srcinfo)
        for retired in ("waybar", "fuzzel", "mako", "swaybg", "swaylock", "swayidle"):
            self.assertNotIn(f"'{retired}'", package)

    def test_retired_component_configs_are_absent_from_source_and_installers(self) -> None:
        retired = ("waybar", "fuzzel", "mako", "swaylock")
        installer = (ROOT / "scripts/install.sh").read_text()
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        for component in retired:
            with self.subTest(component=component):
                component_root = ROOT / "config" / component
                self.assertFalse(component_root.is_dir() and any(component_root.rglob("*")))
                self.assertNotIn(f"config/{component}", installer)
                self.assertNotIn(f"config/{component}", package)
        self.assertIn('config/noctalia', installer)
        self.assertIn('config/noctalia', package)


class ProtocolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = weyriva.PluginRegistry({}, (), ())

    def test_ping_round_trip_shape(self) -> None:
        response = weyriva.process_request(
            {"protocol": 1, "id": "test", "method": "weyriva.ping", "params": {}}, self.registry
        )
        self.assertEqual(response["id"], "test")
        self.assertEqual(response["result"], {"pong": True, "protocol": 1})

    def test_unsupported_protocol_is_structured(self) -> None:
        response = weyriva.process_request(
            {"protocol": 99, "id": 7, "method": "weyriva.ping"}, self.registry
        )
        self.assertEqual(response["error"]["code"], "unsupported_protocol")

    def test_unknown_method_is_structured(self) -> None:
        response = weyriva.process_request(
            {"protocol": 1, "id": 8, "method": "unknown.call"}, self.registry
        )
        self.assertEqual(response["error"]["code"], "method_not_found")


class FramingTests(unittest.TestCase):
    def _exchange(self, payload: bytes) -> dict[str, object]:
        encoded = weyriva.process_line(payload, weyriva.PluginRegistry({}, (), ()))
        return json.loads(encoded)

    def test_unix_socket_accepts_one_json_line(self) -> None:
        response = self._exchange(b'{"protocol":1,"id":2,"method":"weyriva.ping"}\n')
        self.assertTrue(response["result"]["pong"])

    def test_malformed_json_returns_parse_error(self) -> None:
        response = self._exchange(b"not-json\n")
        self.assertEqual(response["error"]["code"], "parse_error")


class PluginTests(unittest.TestCase):
    def _environment(self, root: Path) -> dict[str, str]:
        return {
            "HOME": str(root),
            "XDG_CONFIG_HOME": str(root / "config"),
            "XDG_DATA_HOME": str(root / "data"),
            "XDG_DATA_DIRS": str(root / "system-data"),
        }

    def test_discovers_and_calls_relative_executable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plugins = root / "config/weyriva/plugins"
            plugins.mkdir(parents=True)
            executable = plugins / "echo.py"
            executable.write_text("#!/usr/bin/env python3\nimport json,sys\njson.dump(json.load(sys.stdin),sys.stdout)\n")
            executable.chmod(0o755)
            (plugins / "echo.json").write_text(
                json.dumps({"id": "test", "version": 1, "methods": {"test.echo": {"argv": ["./echo.py"]}}})
            )
            with mock.patch.dict(os.environ, self._environment(root), clear=False):
                registry = weyriva.discover_plugins()
            self.assertEqual(registry.errors, ())
            self.assertEqual(weyriva.run_plugin(registry.methods["test.echo"], {"value": 4}), {"value": 4})

    def test_rejects_reserved_and_duplicate_methods(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plugins = root / "config/weyriva/plugins"
            plugins.mkdir(parents=True)
            base = {"id": "test", "version": 1, "methods": {"test.echo": {"argv": ["true"]}}}
            duplicate = {"id": "test", "version": 1, "methods": {"test.echo": {"argv": ["true"]}}}
            reserved = {"id": "bad", "version": 1, "methods": {"weyriva.steal": {"argv": ["true"]}}}
            (plugins / "1.json").write_text(json.dumps(base))
            (plugins / "2.json").write_text(json.dumps(duplicate))
            (plugins / "3.json").write_text(json.dumps(reserved))
            with mock.patch.dict(os.environ, self._environment(root), clear=False):
                registry = weyriva.discover_plugins()
            self.assertEqual(sorted(registry.methods), ["test.echo"])
            self.assertEqual(len(registry.errors), 2)

    def test_plugin_timeout_becomes_clear_error(self) -> None:
        method = weyriva.PluginMethod("slow", "test.slow", ("sleep", "1"), 0.1, Path("slow.json"))
        with self.assertRaisesRegex(weyriva.PluginError, "timed out"):
            weyriva.run_plugin(method, {})

    def test_plugin_output_limit_is_enforced_incrementally(self) -> None:
        method = weyriva.PluginMethod(
            "large",
            "large.output",
            (sys.executable, "-c", "import sys; sys.stdout.write('x' * (1024 * 1024 + 1))"),
            2,
            Path("large.json"),
        )
        with self.assertRaisesRegex(weyriva.PluginError, "stdout exceeds"):
            weyriva.run_plugin(method, {})

    def test_manifest_requires_version_and_own_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plugins = root / "config/weyriva/plugins"
            plugins.mkdir(parents=True)
            (plugins / "missing-version.json").write_text(
                json.dumps({"id": "demo", "methods": {"demo.call": {"argv": ["true"]}}})
            )
            (plugins / "wrong-namespace.json").write_text(
                json.dumps({"id": "demo", "version": 1, "methods": {"other.call": {"argv": ["true"]}}})
            )
            with mock.patch.dict(os.environ, self._environment(root), clear=False):
                registry = weyriva.discover_plugins()
            self.assertEqual(len(registry.errors), 2)


class DaemonSafetyTests(unittest.TestCase):
    def test_second_lock_attempt_does_not_touch_socket_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            lock_path = root / "daemon.lock"
            socket_marker = root / "weyriva.sock"
            socket_marker.write_text("live")
            first = weyriva.acquire_daemon_lock(lock_path)
            try:
                with self.assertRaisesRegex(RuntimeError, "already running"):
                    weyriva.acquire_daemon_lock(lock_path)
                self.assertEqual(socket_marker.read_text(), "live")
            finally:
                first.close()

    def test_handler_slots_reject_overload(self) -> None:
        server = object.__new__(weyriva.IpcServer)
        server._handler_slots = threading.BoundedSemaphore(1)
        first = mock.Mock()
        overload = mock.Mock()
        self.assertTrue(server.verify_request(first, None))
        self.assertFalse(server.verify_request(overload, None))
        overload.close.assert_called_once_with()
        server._handler_slots.release()


class SessionLifecycleTests(unittest.TestCase):
    def test_startup_ensure_parser(self) -> None:
        arguments = weyriva.build_parser().parse_args(["startup", "ensure"])
        self.assertEqual(arguments.command, "startup")
        self.assertEqual(arguments.startup_command, "ensure")

    def test_reconcile_startup_file_backs_up_existing_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "template.toml"
            destination = root / "config.toml"
            backup = root / "backups/config.toml"
            source.write_text("new\n")
            destination.write_text("old\n")
            self.assertTrue(weyriva.reconcile_startup_file(source, destination, backup))
            self.assertEqual(destination.read_text(), "new\n")
            self.assertEqual(backup.read_text(), "old\n")
            self.assertFalse(weyriva.reconcile_startup_file(source, destination, backup))

    def test_recognized_user_units_shadow_only_matching_packaged_units(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            unit_root = root / ".config/systemd/user"
            unit_root.mkdir(parents=True)
            for name, markers in weyriva.LEGACY_UNIT_MARKERS.items():
                (unit_root / name).write_text(f"[Service]\n{markers[0]}\n")
            packaged = root / "usr/lib/systemd/user"
            with mock.patch.object(weyriva, "PACKAGED_UNIT_ROOT", packaged):
                self.assertEqual(weyriva.legacy_override_units(unit_root), ())
                packaged.mkdir(parents=True)
                (packaged / "weyriva-ipc.service").write_text("[Service]\nExecStart=/usr/bin/weyriva daemon\n")
                self.assertEqual(
                    weyriva.legacy_override_units(unit_root),
                    ("weyriva-ipc.service",),
                )

    def test_recognized_legacy_and_current_units_are_backed_up_but_custom_units_are_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            unit_root = root / ".config/systemd/user"
            backup_root = root / "backups"
            unit_root.mkdir(parents=True)
            legacy = unit_root / "weyriva-waybar.service"
            custom = unit_root / "weyriva-mako.service"
            legacy.write_text("[Service]\nExecStart=/usr/bin/waybar\n")
            custom.write_text("[Service]\nExecStart=/opt/custom-mako\n")
            current_units = {
                "weyriva-ipc.service": "ExecStart=/usr/bin/weyriva daemon",
                "weyriva-shell.service": "ExecStart=/usr/bin/weyriva shell run",
                "weyriva-session-failsafe.service": (
                    "ExecStart=/usr/bin/niri msg action quit --skip-confirmation"
                ),
            }
            for name, marker in current_units.items():
                (unit_root / name).write_text(f"[Service]\n{marker}\n")
            moved = weyriva.back_up_legacy_user_units(unit_root, backup_root)
            self.assertEqual(
                moved,
                (
                    "weyriva-waybar.service",
                    "weyriva-ipc.service",
                    "weyriva-shell.service",
                    "weyriva-session-failsafe.service",
                ),
            )
            self.assertFalse(legacy.exists())
            self.assertTrue((backup_root / legacy.name).is_file())
            for name in current_units:
                self.assertFalse((unit_root / name).exists())
                self.assertTrue((backup_root / name).is_file())
            self.assertTrue(custom.is_file())
            self.assertEqual(weyriva.back_up_legacy_user_units(unit_root, backup_root), ())

    def test_greetd_pam_requires_an_active_session_rule(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            pam = Path(temporary) / "greetd"
            pam.write_text("# session required pam_systemd.so\nauth include system-login\n")
            with self.assertRaisesRegex(RuntimeError, "no active session"):
                weyriva.validate_greetd_pam(pam)
            pam.write_text("auth include system-login\nsession include system-login\n")
            weyriva.validate_greetd_pam(pam)

    def test_greeter_state_directory_repairs_mode_and_refuses_unsafe_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = root / "noctalia-greeter"
            state.mkdir(mode=0o777)
            weyriva.ensure_greeter_state_directory(
                state,
                os.getuid(),
                os.getgid(),
            )
            self.assertEqual(state.stat().st_mode & 0o777, 0o750)

            unsafe = root / "unsafe"
            unsafe.write_text("not a directory\n")
            with self.assertRaisesRegex(RuntimeError, "not a directory"):
                weyriva.ensure_greeter_state_directory(
                    unsafe,
                    os.getuid(),
                    os.getgid(),
                )
            link = root / "state-link"
            link.symlink_to(state, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, "symlinked"):
                weyriva.ensure_greeter_state_directory(
                    link,
                    os.getuid(),
                    os.getgid(),
                )

    def test_startup_ensure_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            packaged_config = root / "share/config"
            packaged_units = root / "share/units"
            greetd_template = root / "share/greetd/config.toml"
            greetd_config = root / "etc/greetd/config.toml"
            greeter_session = root / "bin/noctalia-greeter-session"
            greetd_pam = root / "etc/pam.d/greetd"
            greeter_state = root / "var/lib/noctalia-greeter"
            session_entry = root / "share/wayland-sessions/weyriva.desktop"
            user_home = root / "home/tester"

            (packaged_config / "niri").mkdir(parents=True)
            (packaged_config / "niri/config.kdl").write_text("layout {}\n")
            packaged_units.mkdir(parents=True)
            for name in weyriva.WEYRIVA_UNITS:
                (packaged_units / name).write_text("[Service]\nExecStart=/usr/bin/true\n")
            wants = packaged_units / "niri.service.wants"
            wants.mkdir()
            for name in weyriva.NIRI_WANTED_UNITS:
                (wants / name).symlink_to(f"../{name}")
            greetd_template.parent.mkdir(parents=True)
            greetd_template.write_text(
                "[terminal]\n"
                "vt = 1\n\n"
                "[default_session]\n"
                'command = "/usr/bin/noctalia-greeter-session -- --session Weyriva"\n'
                'user = "greeter"\n'
            )
            greetd_config.parent.mkdir(parents=True)
            greetd_config.write_text("old greetd\n")
            greetd_pam.parent.mkdir(parents=True)
            greetd_pam.write_text("session include system-login\n")
            greeter_state.parent.mkdir(parents=True)
            greeter_session.parent.mkdir(parents=True)
            greeter_session.write_text("#!/bin/sh\n")
            session_entry.parent.mkdir(parents=True)
            session_entry.write_text(
                "Name=Weyriva\nExec=/usr/bin/weyriva session start\n"
            )
            for relative in weyriva.REQUIRED_WALLPAPERS:
                wallpaper = root / "share/wallpapers" / relative
                wallpaper.parent.mkdir(parents=True, exist_ok=True)
                wallpaper.write_text("image\n")
            unit_root = user_home / ".config/systemd/user"
            unit_root.mkdir(parents=True)
            (unit_root / "weyriva-waybar.service").write_text(
                "[Service]\nExecStart=/usr/bin/waybar\n"
            )
            current_unit_markers = {
                "weyriva-ipc.service": "ExecStart=/usr/bin/weyriva daemon",
                "weyriva-shell.service": "ExecStart=/usr/bin/weyriva shell run",
                "weyriva-session-failsafe.service": (
                    "ExecStart=/usr/bin/niri msg action quit --skip-confirmation"
                ),
            }
            for name, marker in current_unit_markers.items():
                (unit_root / name).write_text(f"[Service]\n{marker}\n")

            account = mock.Mock(pw_dir=str(user_home), pw_uid=1000, pw_gid=1000)
            greeter_account = mock.Mock(
                pw_dir=str(greeter_state),
                pw_uid=os.getuid(),
                pw_gid=os.getgid(),
            )
            greeter_group = mock.Mock(gr_gid=os.getgid())

            def lookup_account(name: str) -> mock.Mock:
                return greeter_account if name == "greeter" else account

            with (
                mock.patch.object(weyriva, "PACKAGED_CONFIG_ROOT", packaged_config),
                mock.patch.object(weyriva, "PACKAGED_UNIT_ROOT", packaged_units),
                mock.patch.object(weyriva, "GREETD_TEMPLATE", greetd_template),
                mock.patch.object(weyriva, "GREETD_CONFIG", greetd_config),
                mock.patch.object(weyriva, "GREETD_PAM", greetd_pam),
                mock.patch.object(weyriva, "GREETER_SESSION", greeter_session),
                mock.patch.object(weyriva, "GREETER_STATE_DIR", greeter_state),
                mock.patch.object(weyriva, "PACKAGED_DATA_ROOT", root / "share"),
                mock.patch.object(weyriva, "SESSION_ENTRY", session_entry),
                mock.patch.object(weyriva.os, "geteuid", return_value=0),
                mock.patch.object(weyriva.pwd, "getpwnam", side_effect=lookup_account),
                mock.patch.object(weyriva.grp, "getgrnam", return_value=greeter_group),
                mock.patch.object(weyriva, "_diagnostic_command", return_value="/usr/bin/runtime"),
                mock.patch.object(weyriva, "_run_diagnostic_command", return_value=(0, "valid")),
                mock.patch.object(weyriva, "_run_checked"),
                mock.patch.object(weyriva, "_chown_tree"),
                mock.patch.object(weyriva.time, "strftime", return_value="20260720-120000"),
            ):
                self.assertEqual(weyriva.ensure_startup_chain("tester"), 0)
                self.assertEqual(weyriva.ensure_startup_chain("tester"), 0)

            backup_root = user_home / ".local/state/weyriva/startup-backups/20260720-120000"
            self.assertEqual(greetd_config.read_text(), greetd_template.read_text())
            self.assertEqual((backup_root / "greetd/config.toml").read_text(), "old greetd\n")
            self.assertTrue((backup_root / "systemd/user/weyriva-waybar.service").is_file())
            self.assertFalse((unit_root / "weyriva-waybar.service").exists())
            for name in current_unit_markers:
                self.assertTrue((backup_root / "systemd/user" / name).is_file())
                self.assertFalse((unit_root / name).exists())
            self.assertEqual(greeter_state.stat().st_mode & 0o777, 0o750)

    def test_startup_preflight_failure_causes_no_apply_mutation(self) -> None:
        with (
            mock.patch.object(
                weyriva,
                "preflight_startup_chain",
                side_effect=RuntimeError("known conflict"),
            ),
            mock.patch.object(weyriva, "ensure_greeter_state_directory") as state_write,
            mock.patch.object(weyriva, "reconcile_startup_file") as config_write,
            mock.patch.object(weyriva, "back_up_legacy_user_units") as unit_write,
            self.assertRaisesRegex(RuntimeError, "known conflict"),
        ):
            weyriva.ensure_startup_chain("tester")
        state_write.assert_not_called()
        config_write.assert_not_called()
        unit_write.assert_not_called()

    def test_aur_package_uses_private_default_config_paths(self) -> None:
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        self.assertNotIn('"$pkgdir/etc/xdg', package)
        self.assertNotIn("backup=(", package)
        self.assertIn("usr/share/weyriva/config/niri", package)
        self.assertIn("usr/share/weyriva/config/noctalia", package)
        self.assertIn("usr/share/weyriva/wallpapers/light/weyriva-cactus.png", package)
        self.assertIn(
            "usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png",
            package,
        )
        self.assertIn("usr/lib/systemd/user/niri.service.wants", package)
        for retired in ("waybar", "fuzzel", "mako", "swaylock"):
            self.assertNotIn(f"usr/share/weyriva/config/{retired}", package)

    def test_noctalia_profile_environment_is_isolated_by_default(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            environment = {
                "HOME": str(home),
                "PATH": "/usr/bin",
                "NOCTALIA_CONFIG_HOME": "/standalone/config",
                "NOCTALIA_STATE_HOME": "/standalone/state",
                "NOCTALIA_DATA_HOME": "/standalone/data",
            }
            with mock.patch.object(Path, "home", return_value=home):
                child = weyriva.noctalia_profile_environment(environment)
            self.assertEqual(child["NOCTALIA_CONFIG_HOME"], str(home / ".config/weyriva"))
            self.assertEqual(child["NOCTALIA_STATE_HOME"], str(home / ".local/state/weyriva"))
            self.assertEqual(child["NOCTALIA_DATA_HOME"], str(home / ".local/share/weyriva"))
            self.assertEqual(child["PATH"], "/usr/bin")

    def test_noctalia_profile_environment_honors_custom_xdg_bases(self) -> None:
        root = Path("/tmp/weyriva-profile-test")
        environment = {
            "XDG_CONFIG_HOME": str(root / "config"),
            "XDG_STATE_HOME": str(root / "state"),
            "XDG_DATA_HOME": str(root / "data"),
            "UNRELATED": "preserved",
        }
        child = weyriva.noctalia_profile_environment(environment)
        self.assertEqual(
            {
                key: child[key]
                for key in ("NOCTALIA_CONFIG_HOME", "NOCTALIA_STATE_HOME", "NOCTALIA_DATA_HOME")
            },
            {
                "NOCTALIA_CONFIG_HOME": str(root / "config/weyriva"),
                "NOCTALIA_STATE_HOME": str(root / "state/weyriva"),
                "NOCTALIA_DATA_HOME": str(root / "data/weyriva"),
            },
        )
        self.assertEqual(child["UNRELATED"], "preserved")

    def test_noctalia_delegation_execs_one_fixed_argument_array(self) -> None:
        environment = {
            "XDG_CONFIG_HOME": "/tmp/config",
            "XDG_STATE_HOME": "/tmp/state",
            "XDG_DATA_HOME": "/tmp/data",
        }
        with mock.patch.object(
            weyriva.os, "execvpe", side_effect=OSError("exec boundary")
        ) as execute:
            with self.assertRaisesRegex(OSError, "exec boundary"):
                weyriva.run_noctalia(["msg", "panel-toggle", "launcher;touch /tmp/bad"], environment)
        program, arguments, child = execute.call_args.args
        self.assertEqual(program, "noctalia")
        self.assertEqual(arguments, ["noctalia", "msg", "panel-toggle", "launcher;touch /tmp/bad"])
        self.assertEqual(child["NOCTALIA_CONFIG_HOME"], "/tmp/config/weyriva")

    def test_first_run_seeds_only_missing_profile_and_wallpaper_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            packaged_config = root / "packaged/config/noctalia"
            packaged_data = root / "packaged/data"
            (packaged_config / "palettes").mkdir(parents=True)
            (packaged_data / "wallpapers").mkdir(parents=True)
            (packaged_config / "config.toml").write_text("packaged\n")
            (packaged_config / "palettes/Weyriva.json").write_text("{}\n")
            (packaged_data / "wallpapers/cactus.png").write_text("image\n")

            environment = {
                "HOME": str(root / "user/home"),
                "XDG_CONFIG_HOME": str(root / "user/config"),
                "XDG_DATA_HOME": str(root / "user/data"),
            }
            user_config = root / "user/config/weyriva/noctalia/config.toml"
            user_config.parent.mkdir(parents=True)
            user_config.write_text("user-owned\n")
            with (
                mock.patch.object(weyriva, "PACKAGED_CONFIG_ROOT", packaged_config.parent),
                mock.patch.object(weyriva, "PACKAGED_DATA_ROOT", packaged_data),
            ):
                seeded = weyriva.seed_noctalia_profile(environment)
                second_seed = weyriva.seed_noctalia_profile(environment)

            self.assertEqual(user_config.read_text(), "user-owned\n")
            self.assertEqual(
                (root / "user/config/weyriva/noctalia/palettes/Weyriva.json").read_text(),
                "{}\n",
            )
            self.assertEqual(
                (root / "user/home/.local/share/weyriva/wallpapers/cactus.png").read_text(),
                "image\n",
            )
            self.assertFalse((root / "user/data/weyriva/wallpapers/cactus.png").exists())
            self.assertEqual(len(seeded), 2)
            self.assertEqual(second_seed, ())

    def test_first_run_seed_refuses_symlinked_destination_root_or_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            packaged_config = root / "packaged/config/noctalia"
            packaged_data = root / "packaged/data"
            packaged_config.mkdir(parents=True)
            (packaged_data / "wallpapers").mkdir(parents=True)
            (packaged_config / "config.toml").write_text("packaged\n")
            (packaged_data / "wallpapers/cactus.png").write_text("image\n")
            environment = {
                "HOME": str(root / "user/home"),
                "XDG_CONFIG_HOME": str(root / "user/config"),
                "XDG_DATA_HOME": str(root / "user/data"),
            }

            external = root / "external"
            external.mkdir()
            profile_base = root / "user/config/weyriva"
            profile_base.parent.mkdir(parents=True)
            profile_base.symlink_to(external, target_is_directory=True)
            with (
                mock.patch.object(weyriva, "PACKAGED_CONFIG_ROOT", packaged_config.parent),
                mock.patch.object(weyriva, "PACKAGED_DATA_ROOT", packaged_data),
                self.assertRaisesRegex(RuntimeError, "symlink"),
            ):
                weyriva.seed_noctalia_profile(environment)
            self.assertEqual(list(external.iterdir()), [])

            profile_base.unlink()
            profile = profile_base / "noctalia"
            profile.mkdir(parents=True)
            external_file = external / "config.toml"
            external_file.write_text("external\n")
            (profile / "config.toml").symlink_to(external_file)
            with (
                mock.patch.object(weyriva, "PACKAGED_CONFIG_ROOT", packaged_config.parent),
                mock.patch.object(weyriva, "PACKAGED_DATA_ROOT", packaged_data),
                self.assertRaisesRegex(RuntimeError, "symlink"),
            ):
                weyriva.seed_noctalia_profile(environment)
            self.assertEqual(external_file.read_text(), "external\n")

    def test_first_run_seed_uses_no_follow_file_creation_when_available(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            destination = root / "destination"
            source.mkdir()
            (source / "file").write_text("content\n")
            original_open = os.open
            observed_flags: list[int] = []

            def recording_open(path: object, flags: int, mode: int = 0o777) -> int:
                observed_flags.append(flags)
                return original_open(path, flags, mode)

            with mock.patch.object(weyriva.os, "open", side_effect=recording_open):
                weyriva._seed_missing_regular_files(source, destination)

            self.assertTrue(observed_flags)
            if hasattr(os, "O_NOFOLLOW"):
                self.assertTrue(observed_flags[0] & os.O_NOFOLLOW)

    def test_shell_run_seeds_before_starting_noctalia(self) -> None:
        calls: list[str] = []

        def exec_boundary(_arguments: list[str], _environment: object) -> None:
            calls.append("run")
            raise OSError("exec boundary")

        with (
            mock.patch.object(
                weyriva, "seed_noctalia_profile", side_effect=lambda _environment: calls.append("seed")
            ),
            mock.patch.object(weyriva, "run_noctalia", side_effect=exec_boundary),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(weyriva.main(["shell", "run"]), 1)
        self.assertEqual(calls, ["seed", "run"])

    def test_shell_parser_bounds_run_reconcile_msg_and_config(self) -> None:
        parser = weyriva.build_parser()
        run = parser.parse_args(["shell", "run"])
        self.assertEqual((run.command, run.shell_command), ("shell", "run"))
        reconcile = parser.parse_args(["shell", "reconcile-lock"])
        self.assertEqual((reconcile.command, reconcile.shell_command), ("shell", "reconcile-lock"))
        for route in ("msg", "config"):
            with self.subTest(route=route):
                arguments = parser.parse_args(["shell", route, "validate"])
                self.assertEqual((arguments.command, arguments.shell_command), ("shell", route))
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    parser.parse_args(["shell", route])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            parser.parse_args(["shell", "run", "extra"])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            parser.parse_args(["shell", "arbitrary"])

    def test_niri_config_prefers_environment_then_user_then_packaged(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            packaged = root / "share/weyriva/config"
            packaged_config = packaged / "niri/config.kdl"
            packaged_config.parent.mkdir(parents=True)
            packaged_config.touch()
            environment = {"XDG_CONFIG_HOME": str(root / "config")}
            with mock.patch.object(weyriva, "PACKAGED_CONFIG_ROOT", packaged):
                self.assertEqual(weyriva.niri_config_path(environment), packaged_config)
                user_config = root / "config/niri/config.kdl"
                user_config.parent.mkdir(parents=True)
                user_config.touch()
                self.assertEqual(weyriva.niri_config_path(environment), user_config)
                selected = root / "custom.kdl"
                environment["NIRI_CONFIG"] = str(selected)
                self.assertEqual(weyriva.niri_config_path(environment), selected)

    def test_diagnose_parser_supports_json_output(self) -> None:
        arguments = weyriva.build_parser().parse_args(["diagnose", "--json"])
        self.assertEqual(arguments.command, "diagnose")
        self.assertTrue(arguments.json)

    def test_diagnostic_summary_fails_only_on_failures(self) -> None:
        checks = (
            weyriva.DiagnosticCheck("niri", "ok", "/usr/bin/niri"),
            weyriva.DiagnosticCheck("greetd", "warn", "not configured"),
        )
        summary = weyriva.diagnostic_summary(checks)
        self.assertTrue(summary["ok"])
        self.assertEqual(len(summary["checks"]), 2)

    def test_niri_service_owns_weyriva_units_without_manual_spawn(self) -> None:
        config = (ROOT / "config/niri/config.kdl").read_text()
        self.assertNotIn("spawn-at-startup", config)
        self.assertEqual(
            weyriva.WEYRIVA_UNITS,
            (
                "weyriva-ipc.service",
                "weyriva-shell.service",
                "weyriva-session-failsafe.service",
            ),
        )
        self.assertEqual(
            weyriva.NIRI_WANTED_UNITS,
            ("weyriva-ipc.service", "weyriva-shell.service"),
        )
        self.assertNotIn("weyriva-session.target", config)
        self.assertFalse((ROOT / "systemd/weyriva-session.target").exists())
        self.assertEqual(
            {unit.name for unit in (ROOT / "systemd").glob("*.service")},
            set(weyriva.WEYRIVA_UNITS),
        )
        for name in weyriva.WEYRIVA_UNITS:
            unit = ROOT / "systemd" / name
            content = unit.read_text()
            self.assertIn("PartOf=graphical-session.target", content)
            self.assertIn("After=graphical-session.target", content)
            self.assertIn("Requisite=graphical-session.target", content)
        self.assertIn(
            "ExecStart=/usr/bin/weyriva shell run",
            (ROOT / "systemd/weyriva-shell.service").read_text(),
        )
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        for name in weyriva.NIRI_WANTED_UNITS:
            self.assertIn(f'niri.service.wants/{name}"', package)
        for retired in ("waybar", "mako", "wallpaper", "idle"):
            self.assertNotIn(f"weyriva-{retired}.service", config)

    def test_shell_restart_policy_and_failsafe_are_bounded(self) -> None:
        shell = (ROOT / "systemd/weyriva-shell.service").read_text()
        for setting in (
            "Restart=on-failure",
            "RestartSec=2",
            "RestartMode=direct",
            "StartLimitIntervalSec=30",
            "StartLimitBurst=3",
            "OnFailure=weyriva-session-failsafe.service",
            "ExecStartPost=/usr/bin/weyriva shell reconcile-lock",
        ):
            self.assertIn(setting, shell)
        failsafe = (ROOT / "systemd/weyriva-session-failsafe.service").read_text()
        self.assertIn("Type=oneshot", failsafe)
        self.assertIn(
            "ExecStart=/usr/bin/niri msg action quit --skip-confirmation",
            failsafe,
        )
        self.assertIn("Requisite=graphical-session.target", failsafe)

    def test_greeter_template_uses_noctalia_and_fixed_weyriva_session(self) -> None:
        config = (ROOT / "config/greetd/config.toml").read_text()
        self.assertIn("vt = 1", config)
        self.assertIn(
            'command = "/usr/bin/noctalia-greeter-session -- --session Weyriva"',
            config,
        )
        self.assertIn('user = "greeter"', config)
        self.assertNotIn("tuigreet", config)
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        self.assertIn("config/greetd/config.toml", package)

    def test_niri_binds_all_noctalia_owned_shell_actions(self) -> None:
        config = (ROOT / "config/niri/config.kdl").read_text()
        bindings = (
            'Mod+Space { spawn "weyriva" "shell" "msg" "panel-toggle" "launcher"; }',
            'Mod+N { spawn "weyriva" "shell" "msg" "notification-dnd-toggle"; }',
            'Mod+B { spawn "weyriva" "shell" "msg" "bar-toggle"; }',
            'Mod+C { spawn "weyriva" "shell" "msg" "panel-toggle" "control-center"; }',
            'Mod+V { spawn "weyriva" "shell" "msg" "panel-toggle" "clipboard"; }',
            'Mod+W { spawn "weyriva" "shell" "msg" "panel-toggle" "wallpaper"; }',
            'Mod+Shift+T { spawn "weyriva" "shell" "msg" "theme-mode-toggle"; }',
            'Mod+Shift+E { spawn "weyriva" "shell" "msg" "panel-toggle" "session"; }',
            'Mod+Shift+X { spawn "weyriva" "shell" "msg" "session" "lock"; }',
            'Print { spawn "weyriva" "shell" "msg" "screenshot-region"; }',
        )
        for binding in bindings:
            self.assertIn(binding, config)
        for retired_command in (
            '"component"',
            '"desktop"',
            '"wallpaper"',
            '"idle"',
            '"session" "lock"',
        ):
            self.assertNotIn(f'spawn "weyriva" {retired_command}', config)

    def test_retired_surface_routes_are_absent_from_parser(self) -> None:
        parser = weyriva.build_parser()
        for arguments in (
            ["component", "waybar"],
            ["desktop", "calendar"],
            ["wallpaper"],
            ["idle"],
            ["session", "lock"],
        ):
            with (
                self.subTest(arguments=arguments),
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                parser.parse_args(arguments)

    def test_system_session_uses_absolute_system_cli(self) -> None:
        desktop = (ROOT / "user-share/wayland-sessions/weyriva.desktop").read_text()
        self.assertIn("Name=Weyriva", desktop)
        self.assertIn("Exec=/usr/bin/weyriva session start", desktop)
        self.assertIn("DesktopNames=Weyriva;niri;", desktop)

    def test_session_exec_inherits_path_with_running_cli_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "bin/weyriva"
            executable.parent.mkdir()
            executable.touch()
            with (
                mock.patch.object(weyriva, "_diagnostic_command", return_value="/usr/bin/niri"),
                mock.patch.object(weyriva, "niri_config_path", return_value=Path("/tmp/niri/config.kdl")),
                mock.patch.object(weyriva, "_run_diagnostic_command", return_value=(0, "valid")),
                mock.patch.object(weyriva.os, "execvpe", side_effect=OSError("exec boundary")) as execute,
            ):
                with self.assertRaisesRegex(OSError, "exec boundary"):
                    weyriva.start_session(str(executable), {"PATH": "/usr/bin", "TEST_VALUE": "kept"})
            program, arguments, environment = execute.call_args.args
            self.assertEqual(program, "niri-session")
            self.assertEqual(arguments, ["niri-session"])
            self.assertEqual(environment["PATH"].split(os.pathsep)[0], str(executable.parent.resolve()))
            self.assertEqual(environment["TEST_VALUE"], "kept")


class LockReconciliationTests(unittest.TestCase):
    def _completed(self, returncode: int = 0, stdout: str = "") -> mock.Mock:
        return mock.Mock(returncode=returncode, stdout=stdout, stderr="")

    def test_unlocked_session_returns_without_waiting_for_noctalia(self) -> None:
        runner = mock.Mock(return_value=self._completed(stdout="no\n"))
        sleeper = mock.Mock()
        self.assertEqual(
            weyriva.reconcile_lock(
                {"XDG_SESSION_ID": "c2"},
                runner=runner,
                sleeper=sleeper,
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
        sleeper.assert_not_called()

    def test_missing_environment_session_uses_user_display_session(self) -> None:
        runner = mock.Mock(
            side_effect=(
                self._completed(stdout="7\n"),
                self._completed(stdout="no\n"),
            )
        )
        with mock.patch.object(weyriva.os, "getuid", return_value=1000):
            self.assertEqual(weyriva.reconcile_lock({}, runner=runner), 0)
        self.assertEqual(
            [call.args[0] for call in runner.call_args_list],
            [
                ["loginctl", "show-user", "1000", "-p", "Display", "--value"],
                ["loginctl", "show-session", "7", "-p", "LockedHint", "--value"],
            ],
        )

    def test_locked_session_polls_then_reacquires_isolated_noctalia_lock(self) -> None:
        runner = mock.Mock(
            side_effect=(
                self._completed(stdout="yes\n"),
                self._completed(returncode=1),
                self._completed(),
                self._completed(),
            )
        )
        sleeper = mock.Mock()
        environment = {
            "XDG_SESSION_ID": "c3",
            "XDG_CONFIG_HOME": "/tmp/config",
            "XDG_STATE_HOME": "/tmp/state",
            "XDG_DATA_HOME": "/tmp/data",
        }
        self.assertEqual(
            weyriva.reconcile_lock(
                environment,
                attempts=3,
                interval=0.01,
                runner=runner,
                sleeper=sleeper,
            ),
            0,
        )
        self.assertEqual(
            runner.call_args_list[-1].args[0],
            ["noctalia", "msg", "session", "lock"],
        )
        child = runner.call_args_list[-1].kwargs["env"]
        self.assertEqual(child["NOCTALIA_CONFIG_HOME"], "/tmp/config/weyriva")
        sleeper.assert_called_once_with(0.01)

    def test_locked_session_failure_is_bounded_and_returns_nonzero(self) -> None:
        runner = mock.Mock(
            side_effect=(
                self._completed(stdout="yes\n"),
                self._completed(returncode=1),
                self._completed(returncode=1),
                self._completed(returncode=1),
            )
        )
        sleeper = mock.Mock()
        self.assertEqual(
            weyriva.reconcile_lock(
                {"XDG_SESSION_ID": "9"},
                attempts=3,
                interval=0.01,
                runner=runner,
                sleeper=sleeper,
            ),
            1,
        )
        self.assertEqual(sleeper.call_count, 2)
        self.assertNotIn(
            ["noctalia", "msg", "session", "lock"],
            [call.args[0] for call in runner.call_args_list],
        )

    def test_unknown_locked_hint_fails_closed(self) -> None:
        runner = mock.Mock(return_value=self._completed(stdout="unknown\n"))
        self.assertEqual(
            weyriva.reconcile_lock({"XDG_SESSION_ID": "c4"}, runner=runner),
            1,
        )

    def test_session_resolution_failure_blank_or_invalid_id_fails_closed(self) -> None:
        cases = (
            ({}, (self._completed(returncode=1),)),
            ({}, (self._completed(stdout="\n"),)),
            ({"XDG_SESSION_ID": "c7"}, (self._completed(returncode=1),)),
            ({"XDG_SESSION_ID": "bad id"}, ()),
            ({"XDG_SESSION_ID": "c5\ncontrol"}, ()),
            ({"XDG_SESSION_ID": " c6"}, ()),
        )
        for environment, results in cases:
            with self.subTest(environment=environment):
                runner = mock.Mock(side_effect=results)
                self.assertEqual(
                    weyriva.reconcile_lock(environment, runner=runner),
                    1,
                )
                for call in runner.call_args_list:
                    self.assertNotIn("self", call.args[0])


class ControlMethodTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = weyriva.PluginRegistry({"test.echo": mock.Mock()}, (), ())

    def _completed(self, returncode: int = 0, stdout: str = "", stderr: str = "") -> mock.Mock:
        return mock.Mock(returncode=returncode, stdout=stdout, stderr=stderr)

    def test_methods_lists_builtins_and_plugins(self) -> None:
        result = weyriva.dispatch("weyriva.methods", {}, self.registry)
        self.assertEqual(result["builtin"], list(weyriva.BUILTIN_METHODS))
        self.assertEqual(result["plugin"], ["test.echo"])
        self.assertIn("weyriva.notifications.dnd", result["builtin"])

    def test_dnd_routes_use_the_isolated_noctalia_ipc_bridge(self) -> None:
        with mock.patch.object(
            weyriva, "_noctalia_ipc", return_value={"command": "notification-dnd-toggle"}
        ) as bridge:
            result = weyriva.dispatch("weyriva.notifications.dnd", {}, self.registry)
        self.assertEqual(result, {"command": "notification-dnd-toggle"})
        self.assertEqual(bridge.call_args.args[0], ["notification-dnd-toggle"])
        with mock.patch.object(weyriva, "_noctalia_ipc", return_value={}) as bridge:
            weyriva.dispatch("weyriva.notifications.dnd", {"enabled": True}, self.registry)
        self.assertEqual(bridge.call_args.args[0], ["notification-dnd-set", "on"])
        with mock.patch.object(weyriva, "_noctalia_ipc", return_value={}) as bridge:
            weyriva.dispatch("weyriva.notifications.dnd", {"enabled": False}, self.registry)
        self.assertEqual(bridge.call_args.args[0], ["notification-dnd-set", "off"])

    def test_dnd_rejects_non_boolean_parameters(self) -> None:
        with self.assertRaises(weyriva.ProtocolError) as raised:
            weyriva.dispatch("weyriva.notifications.dnd", {"enabled": "yes"}, self.registry)
        self.assertEqual(raised.exception.code, "invalid_params")

    def test_panel_aliases_use_the_noctalia_bar(self) -> None:
        with mock.patch.object(weyriva, "_noctalia_ipc", return_value={}) as bridge:
            weyriva.dispatch("weyriva.panel.toggle", {}, self.registry)
        self.assertEqual(bridge.call_args.args[0], ["bar-toggle"])
        with mock.patch.object(weyriva, "_noctalia_ipc", return_value={}) as bridge:
            weyriva.dispatch("weyriva.panel.reload", {}, self.registry)
        self.assertEqual(bridge.call_args.args[0], ["config-reload"])

    def test_plugin_reload_swaps_registry_through_reloader(self) -> None:
        fresh = weyriva.PluginRegistry({}, (), ("broken.json: cannot read JSON",))
        result = weyriva.dispatch("weyriva.plugin.reload", {}, self.registry, reloader=lambda: fresh)
        self.assertEqual(result, weyriva.plugin_summary(fresh))

    def test_plugin_reload_requires_daemon_context(self) -> None:
        with self.assertRaises(weyriva.ProtocolError) as raised:
            weyriva.dispatch("weyriva.plugin.reload", {}, self.registry)
        self.assertEqual(raised.exception.code, "unavailable")

    def test_niri_queries_parse_json_and_report_missing_niri(self) -> None:
        payload = '[{"name": "eDP-1", "logical": {"width": 2256}}]'
        with mock.patch.object(weyriva.subprocess, "run", return_value=self._completed(stdout=payload)) as run:
            result = weyriva.dispatch("weyriva.niri.outputs", {}, self.registry)
        self.assertEqual(run.call_args.args[0], ["niri", "msg", "-j", "outputs"])
        self.assertEqual(result[0]["name"], "eDP-1")
        with mock.patch.object(weyriva.subprocess, "run", side_effect=FileNotFoundError("niri")):
            with self.assertRaises(weyriva.ProtocolError) as raised:
                weyriva.dispatch("weyriva.niri.windows", {}, self.registry)
        self.assertEqual(raised.exception.code, "unavailable")

    def test_validate_manifest_reports_methods_and_missing_executables(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "demo.json"
            manifest.write_text(
                json.dumps(
                    {
                        "id": "demo",
                        "version": 1,
                        "methods": {
                            "demo.here": {"argv": ["true"]},
                            "demo.gone": {"argv": ["./missing.py"]},
                        },
                    }
                )
            )
            summary = weyriva.validate_manifest(manifest)
            self.assertEqual(summary["id"], "demo")
            self.assertEqual(summary["methods"], ["demo.gone", "demo.here"])
            self.assertEqual(summary["missing_executables"], [str(root / "missing.py")])
            manifest.write_text("{not json")
            with self.assertRaises(weyriva.PluginError):
                weyriva.validate_manifest(manifest)
            with self.assertRaisesRegex(weyriva.PluginError, "does not exist"):
                weyriva.validate_manifest(root / "absent.json")

    def test_plugin_parser_keeps_python_plugins_under_flat_legacy_names(self) -> None:
        arguments = weyriva.build_parser().parse_args(["plugin", "legacy-list"])
        self.assertEqual(arguments.plugin_command, "legacy-list")
        arguments = weyriva.build_parser().parse_args(["plugin", "legacy-reload"])
        self.assertEqual(arguments.plugin_command, "legacy-reload")
        arguments = weyriva.build_parser().parse_args(["plugin", "legacy-validate", "demo.json"])
        self.assertEqual(arguments.plugin_command, "legacy-validate")
        self.assertEqual(arguments.path, "demo.json")
        for old_route in (["plugin", "reload"], ["plugin", "validate", "demo.json"]):
            with (
                self.subTest(old_route=old_route),
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                weyriva.build_parser().parse_args(old_route)


class NativePluginCliTests(unittest.TestCase):
    def assert_delegates(self, source: list[str], expected: list[str]) -> None:
        with mock.patch.object(
            weyriva, "run_noctalia", side_effect=OSError("exec boundary")
        ) as delegate:
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(weyriva.main(source), 1)
        self.assertEqual(delegate.call_args.args[0], expected)

    def test_native_plugin_lifecycle_maps_to_noctalia_without_translation(self) -> None:
        plugin_id = "noctalia/screen_recorder"
        cases = (
            (["plugin", "list"], ["msg", "plugins", "list"]),
            (["plugin", "install", plugin_id], ["msg", "plugins", "enable", plugin_id]),
            (["plugin", "enable", plugin_id], ["msg", "plugins", "enable", plugin_id]),
            (["plugin", "disable", plugin_id], ["msg", "plugins", "disable", plugin_id]),
            (["plugin", "update", "official"], ["msg", "plugins", "update", "official"]),
        )
        for source, expected in cases:
            with self.subTest(source=source):
                self.assert_delegates(source, expected)

    def test_native_plugin_parser_rejects_non_upstream_lifecycle_shapes(self) -> None:
        parser = weyriva.build_parser()
        for arguments in (
            ["plugin", "remove", "noctalia/screen_recorder"],
            ["plugin", "update"],
            ["plugin", "update", "official", "community"],
        ):
            with (
                self.subTest(arguments=arguments),
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                parser.parse_args(arguments)

    def test_native_plugin_sources_map_to_noctalia(self) -> None:
        cases = (
            (["plugin", "source", "list"], ["msg", "plugins", "source", "list"]),
            (
                ["plugin", "source", "add", "mine", "git", "https://example.test/plugins"],
                [
                    "msg",
                    "plugins",
                    "source",
                    "add",
                    "mine",
                    "git",
                    "https://example.test/plugins",
                ],
            ),
            (
                ["plugin", "source", "add", "dev", "path", "/tmp/plugins;literal"],
                ["msg", "plugins", "source", "add", "dev", "path", "/tmp/plugins;literal"],
            ),
            (["plugin", "source", "remove", "mine"], ["msg", "plugins", "source", "remove", "mine"]),
        )
        for source, expected in cases:
            with self.subTest(source=source):
                self.assert_delegates(source, expected)

    def test_native_plugin_lint_uses_offline_noctalia_tool(self) -> None:
        self.assert_delegates(["plugin", "lint"], ["plugins", "lint"])
        self.assert_delegates(
            ["plugin", "lint", "/tmp/one", "/tmp/two;literal"],
            ["plugins", "lint", "/tmp/one", "/tmp/two;literal"],
        )


class NoctaliaProfileTests(unittest.TestCase):
    @staticmethod
    def _contrast(left: str, right: str) -> float:
        def luminance(color: str) -> float:
            channels = [int(color[index : index + 2], 16) / 255 for index in (1, 3, 5)]
            linear = [
                channel / 12.92
                if channel <= 0.04045
                else ((channel + 0.055) / 1.055) ** 2.4
                for channel in channels
            ]
            return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]

        first, second = sorted((luminance(left), luminance(right)), reverse=True)
        return (first + 0.05) / (second + 0.05)

    def test_profile_declares_native_sources_and_weyriva_palette(self) -> None:
        config_path = ROOT / "config/noctalia/config.toml"
        with config_path.open("rb") as stream:
            document = tomllib.load(stream)
        self.assertIsInstance(document, dict)
        content = config_path.read_text()
        self.assertIn("official-plugins", content)
        self.assertIn("community-plugins", content)
        self.assertIn("Weyriva", content)
        self.assertTrue(document["dock"]["enabled"])
        self.assertTrue(document["osd"]["enabled"])
        self.assertTrue(document["shell"]["clipboard_enabled"])
        self.assertTrue(document["shell"]["polkit_agent"])
        self.assertTrue(document["shell"]["animation"]["enabled"])
        self.assertEqual(document["shell"]["animation"]["speed"], 1.1)
        self.assertEqual(document["shell"]["panel"]["transparency_mode"], "soft")
        self.assertTrue(document["shell"]["greeter_sync"]["auto_sync"])
        self.assertEqual(document["theme"]["mode"], "auto")
        self.assertEqual(document["theme"]["source"], "wallpaper")
        self.assertEqual(document["theme"]["wallpaper_scheme"], "soft")
        self.assertTrue(document["location"]["custom_schedule"])
        self.assertEqual(document["location"]["sunrise"], "07:00")
        self.assertEqual(document["location"]["sunset"], "19:00")
        self.assertEqual(
            document["wallpaper"]["directory"],
            "~/.local/share/weyriva/wallpapers",
        )
        self.assertEqual(document["wallpaper"]["transition"], ["fade"])
        self.assertEqual(document["wallpaper"]["transition_duration"], 400)
        self.assertTrue(document["wallpaper"]["transition_on_startup"])
        self.assertEqual(
            document["wallpaper"]["directory_light"],
            "~/.local/share/weyriva/wallpapers/light",
        )
        self.assertEqual(
            document["wallpaper"]["directory_dark"],
            "~/.local/share/weyriva/wallpapers/dark",
        )
        self.assertEqual(
            document["wallpaper"]["default"]["path"],
            "~/.local/share/weyriva/wallpapers/light/weyriva-cactus.png",
        )
        self.assertIn("theme_mode", document["bar"]["default"]["end"])
        self.assertIn("settings", document["bar"]["default"]["end"])

        palette = json.loads((ROOT / "config/noctalia/palettes/Weyriva.json").read_text())
        serialized = json.dumps(palette)
        for color in ("141413", "FAF9F5", "BCD1CA"):
            self.assertIn(color, serialized)
        roles = {
            "mPrimary",
            "mOnPrimary",
            "mSecondary",
            "mOnSecondary",
            "mTertiary",
            "mOnTertiary",
            "mError",
            "mOnError",
            "mSurface",
            "mOnSurface",
            "mSurfaceVariant",
            "mOnSurfaceVariant",
            "mOutline",
            "mShadow",
            "mHover",
            "mOnHover",
        }
        ansi = {"black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"}
        terminal_direct = {
            "foreground",
            "background",
            "cursor",
            "cursorText",
            "selectionFg",
            "selectionBg",
        }
        for mode in ("dark", "light"):
            with self.subTest(mode=mode):
                theme = palette[mode]
                self.assertTrue(roles.issubset(theme))
                self.assertEqual(set(theme["terminal"]["normal"]), ansi)
                self.assertEqual(set(theme["terminal"]["bright"]), ansi)
                self.assertTrue(terminal_direct.issubset(theme["terminal"]))
                for foreground, background in (
                    ("mOnPrimary", "mPrimary"),
                    ("mOnSecondary", "mSecondary"),
                    ("mOnTertiary", "mTertiary"),
                    ("mOnError", "mError"),
                    ("mOnSurface", "mSurface"),
                    ("mOnSurfaceVariant", "mSurfaceVariant"),
                    ("mOnHover", "mHover"),
                ):
                    self.assertGreaterEqual(
                        self._contrast(theme[foreground], theme[background]),
                        4.5,
                    )
                terminal = theme["terminal"]
                self.assertGreaterEqual(
                    self._contrast(terminal["foreground"], terminal["background"]),
                    4.5,
                )

    def test_light_and_dark_wallpaper_assets_are_seeded_and_packaged(self) -> None:
        self.assertTrue((ROOT / "assets/wallpapers/weyriva-cactus.png").is_file())
        self.assertTrue((ROOT / "assets/wallpapers/weyriva-cactus-dark.png").is_file())
        installer = (ROOT / "scripts/install.sh").read_text()
        system_installer = (ROOT / "scripts/install-system.sh").read_text()
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        for relative in (
            "wallpapers/light/weyriva-cactus.png",
            "wallpapers/dark/weyriva-cactus-dark.png",
        ):
            self.assertIn(relative, installer)
            self.assertIn(relative, system_installer)
            self.assertIn(relative, package)

    def test_v4_parity_rows_remain_explicitly_incomplete(self) -> None:
        rows = [
            line.lower()
            for line in (ROOT / "docs/NOCTALIA_PARITY.md").read_text().splitlines()
            if line.startswith("|") and "v4" in line.lower()
        ]
        self.assertTrue(rows)
        for row in rows:
            self.assertIn("pending", row)
            self.assertNotIn("complete", row)

    def test_roadmap_tracks_current_stack_and_keeps_acceptance_gates_pending(self) -> None:
        roadmap = (ROOT / "docs/ROADMAP.md").read_text()
        for current in (
            "Noctalia v5",
            "Noctalia Greeter",
            "systemd",
            "AUR",
            "user-only installer",
        ):
            self.assertIn(current, roadmap)
        for pending in ("v4 compatibility", "catalog matrix", "XRY acceptance"):
            self.assertIn(pending, roadmap)
        self.assertNotIn("Waybar, fuzzel, mako", roadmap)


if __name__ == "__main__":
    unittest.main()
