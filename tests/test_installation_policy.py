from __future__ import annotations

import hashlib
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from weyriva_test_support import ROOT


class InstallationPolicyTests(unittest.TestCase):
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
            content.index('niri validate -c "$ROOT/config/niri/config.kdl"'),
            content.index(apply_loop),
        )
        self.assertLess(
            content.index('niri validate -c "$user_niri_config"'),
            content.index(apply_loop),
        )
        guard = "if [[ $PREFLIGHT == true ]]; then"
        self.assertIn(guard, content)
        self.assertLess(content.index(guard), content.index("backup_existing()"))
        self.assertLess(content.index(guard), content.index(apply_loop))
        self.assertIn("--preflight", content.split(guard, 1)[0])
        startup_ensure = '/usr/bin/weyriva startup ensure --user "$TARGET_USER"'
        daemon_reload = '"${user_systemctl[@]}" daemon-reload'
        reload_guard = (
            "if [[ -d $user_runtime_dir && ! -L $user_runtime_dir &&\n"
            '    $(stat -c %u "$user_runtime_dir") == "$target_uid" &&\n'
            "    -S $user_bus && ! -L $user_bus &&\n"
            '    $(stat -c %u "$user_bus") == "$target_uid" ]]; then'
        )
        self.assertIn(startup_ensure, content)
        self.assertIn('user_runtime_dir="/run/user/$target_uid"', content)
        self.assertIn('user_bus="$user_runtime_dir/bus"', content)
        self.assertIn(reload_guard, content)
        self.assertIn('env HOME="$target_home"', content)
        self.assertIn('XDG_RUNTIME_DIR="$user_runtime_dir"', content)
        self.assertIn(
            'DBUS_SESSION_BUS_ADDRESS="unix:path=$user_bus"',
            content,
        )
        self.assertIn('if "${user_systemctl[@]}" show-environment', content)
        self.assertIn(daemon_reload, content)
        startup_index = content.index(startup_ensure)
        guard_index = content.index(reload_guard)
        manager_guard_index = content.index("if [[ $user_manager_active == true ]]")
        daemon_reload_index = content.index(daemon_reload)
        self.assertLess(guard_index, startup_index)
        self.assertLess(startup_index, manager_guard_index)
        self.assertLess(manager_guard_index, daemon_reload_index)
        self.assertIn('if [[ $active_ipc == true ]]', content)
        self.assertIn('if [[ $active_shell == true ]]', content)
        self.assertIn('stop weyriva-ipc.service', content)
        self.assertIn('start weyriva-ipc.service', content)
        self.assertIn('stop weyriva-shell.service', content)
        self.assertIn('start weyriva-shell.service', content)
        self.assertNotIn("systemctl restart", content)
        self.assertNotIn("systemctl --user restart", content)

    def test_aur_package_uses_only_independent_runtime(self) -> None:
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        srcinfo = (ROOT / "packaging/aur/.SRCINFO").read_text()
        for dependency in (
            "niri",
            "greetd",
            "quickshell>=0.3",
            "cage",
            "foot",
            "wl-clipboard",
            "libnotify",
        ):
            self.assertIn(f"'{dependency}'", package)
        for dependency in (
            "niri",
            "greetd",
            "quickshell>=0.3",
            "cage",
            "foot",
            "wl-clipboard",
            "libnotify",
        ):
            self.assertIn(f"\tdepends = {dependency}\n", srcinfo)
        self.assertIn("cargo build --release --locked", package)
        self.assertIn("-p weyriva", package)
        self.assertIn("-p weyriva-luau-host", package)
        self.assertIn(
            'install -Dm755 target/release/weyriva "$pkgdir/usr/bin/weyriva"',
            package,
        )
        self.assertIn(
            'install -Dm755 target/release/weyriva-luau-host',
            package,
        )
        self.assertNotIn("python", package.lower())
        self.assertNotIn("weyriva_plugins_v5", package)
        self.assertNotIn("\tdepends = python\n", srcinfo)
        self.assertIn('cp -a shell "$pkgdir/usr/share/weyriva/"', package)
        self.assertIn('cp -a greeter "$pkgdir/usr/share/weyriva/"', package)
        self.assertIn('cp -a config/weyriva "$pkgdir/usr/share/weyriva/config/"', package)
        self.assertNotIn("noctalia", package.lower())
        self.assertNotIn("noctalia", srcinfo.lower())

    def test_installed_runtime_is_all_rust_and_has_no_python_product_surface(self) -> None:
        self.assertFalse((ROOT / "bin/weyriva").exists())
        self.assertFalse((ROOT / "lib/weyriva_plugins_v5.py").exists())
        installer = (ROOT / "scripts/install-system.sh").read_text()
        package = (ROOT / "packaging/aur/PKGBUILD").read_text()
        for source in (installer, package):
            self.assertIn("target/release/weyriva", source)
            self.assertIn("target/release/weyriva-luau-host", source)
        self.assertNotRegex(installer, r"\bpython(?:3)?\b")
        self.assertNotRegex(package.lower(), r"\bpython(?:3)?\b")
        self.assertNotIn("weyriva_plugins_v5", package)
        self.assertEqual(installer.count("weyriva_plugins_v5.py"), 1)
        self.assertIn(
            "legacy_runtime=/usr/lib/weyriva/weyriva_plugins_v5.py",
            installer,
        )
        self.assertIn(
            'cp -a --no-dereference -- "$legacy_runtime" "$legacy_backup"',
            installer,
        )
        self.assertIn('rm -f -- "$legacy_runtime"', installer)

    def test_system_package_plans_complete_shell_greeter_and_config_trees(self) -> None:
        installer = (ROOT / "scripts/install-system.sh").read_text()
        required_trees = {
            "shell": (
                ROOT / "shell/shell.qml",
                ROOT / "shell/Weyriva/qmldir",
                ROOT / "shell/Weyriva/ActionButton.qml",
                ROOT / "shell/Weyriva/BrandMark.qml",
                ROOT / "shell/Weyriva/CalendarSurface.qml",
                ROOT / "shell/Weyriva/ControlCenterSurface.qml",
                ROOT / "shell/Weyriva/LauncherSurface.qml",
                ROOT / "shell/Weyriva/LockSurface.qml",
                ROOT / "shell/Weyriva/NotificationsSurface.qml",
                ROOT / "shell/Weyriva/PluginLauncherBridge.qml",
                ROOT / "shell/Weyriva/SettingsSurface.qml",
                ROOT / "shell/Weyriva/ShellState.qml",
                ROOT / "shell/Weyriva/SurfaceHeader.qml",
                ROOT / "shell/Weyriva/SurfacePanel.qml",
                ROOT / "shell/Weyriva/Theme.qml",
                ROOT / "shell/Weyriva/TopBar.qml",
                ROOT / "shell/Weyriva/UtilityRow.qml",
                ROOT / "shell/Weyriva/WallpaperPreview.qml",
                ROOT / "shell/Weyriva/WallpaperSurface.qml",
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
            self.assertEqual(
                hashlib.sha256(destination.read_bytes()).digest(),
                hashlib.sha256(payload).digest(),
            )
