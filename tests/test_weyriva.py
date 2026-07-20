from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import os
import sys
import tempfile
import threading
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
    def test_niri_starts_graphical_session_bound_services(self) -> None:
        config = (ROOT / "config/niri/config.kdl").read_text()
        self.assertIn('spawn-at-startup "systemctl" "--user" "start"', config)
        self.assertNotIn("weyriva-session.target", config)
        self.assertFalse((ROOT / "systemd/weyriva-session.target").exists())
        for unit in sorted((ROOT / "systemd").glob("*.service")):
            content = unit.read_text()
            self.assertIn("PartOf=graphical-session.target", content)
            self.assertIn("After=graphical-session.target", content)
            self.assertIn("Requisite=graphical-session.target", content)

    def test_system_session_uses_absolute_system_cli(self) -> None:
        desktop = (ROOT / "user-share/wayland-sessions/weyriva.desktop").read_text()
        self.assertIn("Exec=/usr/bin/weyriva session start", desktop)
        self.assertNotIn("DesktopNames", desktop)

    def test_session_exec_inherits_path_with_running_cli_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "bin/weyriva"
            executable.parent.mkdir()
            executable.touch()
            with mock.patch.object(weyriva.os, "execvpe", side_effect=OSError("exec boundary")) as execute:
                with self.assertRaisesRegex(OSError, "exec boundary"):
                    weyriva.start_session(str(executable), {"PATH": "/usr/bin", "TEST_VALUE": "kept"})
            program, arguments, environment = execute.call_args.args
            self.assertEqual(program, "niri-session")
            self.assertEqual(arguments, ["niri-session"])
            self.assertEqual(environment["PATH"].split(os.pathsep)[0], str(executable.parent.resolve()))
            self.assertEqual(environment["TEST_VALUE"], "kept")


if __name__ == "__main__":
    unittest.main()
