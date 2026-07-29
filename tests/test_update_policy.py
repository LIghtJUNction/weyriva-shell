from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from update_policy_support import (
    create_complete_system_footprint,
    make_update_harness,
    run_sourced_update,
)


class UpdatePolicyTests(unittest.TestCase):
    def test_complete_system_install_routes_to_root_installer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            create_complete_system_footprint(system_root)

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                trace.read_text().splitlines(),
                [
                    f"git -C {temporary / 'project'} pull --ff-only",
                    "system-install ",
                ],
            )

    def test_absent_system_install_routes_to_user_installer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                trace.read_text().splitlines(),
                [
                    f"git -C {temporary / 'project'} pull --ff-only",
                    "build-release",
                    "user-install --apply",
                ],
            )

    def test_partial_system_install_fails_before_pull(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            binary = system_root / "usr/bin/weyriva"
            binary.parent.mkdir(parents=True)
            binary.write_text("partial\n")

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Partial or unsafe Weyriva system footprint", result.stderr)
            self.assertFalse(trace.exists())

    def test_incomplete_inventory_fails_before_pull(self) -> None:
        installed_paths = (
            "usr/lib/systemd/user/weyriva-session-failsafe.service",
            "usr/lib/systemd/user/niri.service.wants/weyriva-ipc.service",
            "usr/share/weyriva/shell/Weyriva/Panel.qml",
            "usr/share/weyriva/greeter/shell.qml",
            "usr/share/weyriva/config/weyriva/defaults.json",
            "usr/share/weyriva/greetd/config.toml",
            "usr/share/weyriva/wallpapers/light/weyriva-cactus.png",
            "usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png",
        )
        for relative in installed_paths:
            with self.subTest(relative=relative):
                with tempfile.TemporaryDirectory() as directory:
                    temporary = Path(directory)
                    script, system_root, trace, environment = make_update_harness(
                        temporary
                    )
                    create_complete_system_footprint(system_root)
                    (system_root / relative).unlink()

                    result = run_sourced_update(
                        script, system_root, environment, "--apply"
                    )

                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(trace.exists())

    def test_wrong_wants_target_fails_before_pull(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            create_complete_system_footprint(system_root)
            link = (
                system_root
                / "usr/lib/systemd/user/niri.service.wants/weyriva-shell.service"
            )
            link.unlink()
            link.symlink_to("../wrong.service")

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(trace.exists())

    def test_empty_share_tree_fails_before_pull(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            (system_root / "usr/share/weyriva").mkdir(parents=True)

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(trace.exists())

    def test_unsafe_system_type_fails_before_pull(self) -> None:
        for unsafe_type in ("unit-directory", "symlinked-wants-directory"):
            with self.subTest(unsafe_type=unsafe_type):
                with tempfile.TemporaryDirectory() as directory:
                    temporary = Path(directory)
                    script, system_root, trace, environment = make_update_harness(
                        temporary
                    )
                    create_complete_system_footprint(system_root)
                    if unsafe_type == "unit-directory":
                        unit = (
                            system_root
                            / "usr/lib/systemd/user/weyriva-shell.service"
                        )
                        unit.unlink()
                        unit.mkdir()
                    else:
                        wants = (
                            system_root
                            / "usr/lib/systemd/user/niri.service.wants"
                        )
                        redirected = wants.with_name("redirected-wants")
                        wants.rename(redirected)
                        wants.symlink_to(redirected, target_is_directory=True)

                    result = run_sourced_update(
                        script, system_root, environment, "--apply"
                    )

                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(trace.exists())

    def test_source_inventory_anomalies_fail_before_pull(self) -> None:
        for anomaly in ("new-file", "empty-systemd", "symlinked-parent"):
            with self.subTest(anomaly=anomaly):
                with tempfile.TemporaryDirectory() as directory:
                    temporary = Path(directory)
                    script, system_root, trace, environment = make_update_harness(
                        temporary
                    )
                    create_complete_system_footprint(system_root)
                    project = temporary / "project"
                    if anomaly == "new-file":
                        (project / "shell/Weyriva/Future.qml").write_text("new\n")
                    elif anomaly == "empty-systemd":
                        for unit in (project / "systemd").glob("*.service"):
                            unit.unlink()
                    else:
                        parent = project / "config/greetd"
                        redirected = parent.with_name("greetd-real")
                        parent.rename(redirected)
                        parent.symlink_to(redirected, target_is_directory=True)

                    result = run_sourced_update(
                        script, system_root, environment, "--apply"
                    )

                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(trace.exists())

    def test_system_update_synchronizes_existing_user_overrides(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            create_complete_system_footprint(system_root)
            manifest = (
                Path(environment["XDG_STATE_HOME"])
                / "weyriva/installed-files.tsv"
            )
            manifest.parent.mkdir(parents=True)
            manifest.write_text("digest\tmanaged-path\n")

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                trace.read_text().splitlines(),
                [
                    f"git -C {temporary / 'project'} pull --ff-only",
                    "system-install ",
                    "user-install --apply",
                ],
            )

    def test_dry_run_reports_selected_path_without_commands(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            create_complete_system_footprint(system_root)

            result = run_sourced_update(script, system_root, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("complete system installation", result.stdout)
            self.assertIn(f"{temporary / 'project/install.sh'}", result.stdout)
            self.assertFalse(trace.exists())

    def test_aur_detection_precedes_footprint_classification(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            script, system_root, trace, environment = make_update_harness(temporary)
            environment["FAKE_AUR"] = "1"

            result = run_sourced_update(script, system_root, environment, "--apply")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("paru -Syu weyriva-shell-git", result.stdout)
            self.assertFalse(trace.exists())
