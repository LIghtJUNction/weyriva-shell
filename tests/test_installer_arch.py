from __future__ import annotations

import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from weyriva_test_support import ROOT


class InstallerTests(unittest.TestCase):
    def _run_fake_arch(self, installed: tuple[str, ...]) -> tuple[subprocess.CompletedProcess[str], str]:
        with tempfile.TemporaryDirectory() as temporary:
            fake_root = Path(temporary)
            log = fake_root / "calls.log"
            generic_installed = fake_root / "generic-quickshell"
            binaries = (
                ROOT / "target/release/weyriva",
                ROOT / "target/release/weyriva-luau-host",
            )
            preexisting_binaries = {path: path.exists() for path in binaries}
            installed_cases = "|".join(installed)
            installed_words = " ".join(installed)

            def executable(name: str, body: str = "exit 0\n") -> None:
                path = fake_root / name
                path.write_text("#!/usr/bin/bash\nset -eu\n" + body)
                path.chmod(0o755)

            executable("uname", "printf '%s\\n' Linux\n")
            executable("id")
            executable(
                "getent",
                f"[[ ${{1:-}} == passwd && ${{2:-}} == tester ]] && "
                f"printf '%s\\n' 'tester:x:1000:1000::{fake_root}:/bin/bash' && exit 0\n"
                "exit 1\n",
            )
            executable("runuser")
            executable("rustc", "printf '%s\\n' 'rustc 1.88.0'\n")
            executable("od", "printf '%s\\n' ' 7f 45 4c 46'\n")
            executable(
                "cargo",
                f"printf 'cargo %s\\n' \"$*\" >>'{log}'\n"
                f"mkdir -p '{binaries[0].parent}'\n"
                f"[[ -x '{binaries[0]}' ]] || {{ "
                f"printf '%s\\n' '#!/usr/bin/bash' 'exit 0' >'{binaries[0]}'; "
                f"chmod 0755 '{binaries[0]}'; }}\n"
                f"[[ -x '{binaries[1]}' ]] || {{ "
                f"printf '%s\\n' '#!/usr/bin/bash' "
                f"'printf \"%s\\\\n\" \"usage: weyriva-luau-host --plugin-dir\" >&2' "
                f"'exit 0' >'{binaries[1]}'; "
                f"chmod 0755 '{binaries[1]}'; }}\n",
            )
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
                "    [[ $* == pacman\\ -S\\ --noconfirm\\ --ask=4\\ --needed* ]] || exit 98\n"
                f"      : >'{generic_installed}'\n"
                "  fi\n"
                "fi\n"
                "exit 0\n",
            )
            for command in (
                "niri",
                "niri-session",
                "quickshell",
                "cage",
                "foot",
                "wl-copy",
                "notify-send",
            ):
                executable(command)
            (fake_root / "awk").symlink_to("/usr/bin/awk")
            (fake_root / "chmod").symlink_to("/usr/bin/chmod")
            (fake_root / "cut").symlink_to("/usr/bin/cut")
            (fake_root / "grep").symlink_to("/usr/bin/grep")
            (fake_root / "mkdir").symlink_to("/usr/bin/mkdir")
            (fake_root / "readlink").symlink_to("/usr/bin/readlink")
            (fake_root / "stat").symlink_to("/usr/bin/stat")
            (fake_root / "tr").symlink_to("/usr/bin/tr")

            try:
                completed = subprocess.run(
                    ["/usr/bin/bash", str(ROOT / "install.sh")],
                    env={"PATH": str(fake_root), "USER": "tester"},
                    capture_output=True,
                    text=True,
                    check=False,
                )
                return completed, log.read_text() if log.exists() else ""
            finally:
                for binary in binaries:
                    if not preexisting_binaries[binary]:
                        binary.unlink(missing_ok=True)

    def test_canonical_installer_is_zero_choice_and_cross_distribution(self) -> None:
        installer = ROOT / "install.sh"
        content = installer.read_text()
        self.assertTrue(os.access(installer, os.X_OK))
        self.assertIn("[[ $# -eq 0 ]]", content)
        for manager in ("pacman", "dnf", "apt-get", "zypper"):
            self.assertIn(manager, content)
        for dependency in (
            "niri",
            "greetd",
            "quickshell",
            "cage",
            "foot",
            "rust",
            "wl-clipboard",
            "libnotify",
        ):
            self.assertIn(dependency, content)
        self.assertIn("cargo build", content)
        self.assertIn("--release", content)
        self.assertIn("--locked", content)
        self.assertNotIn("python", content.lower())
        self.assertNotIn("weyriva_plugins_v5", content)
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
        self.assertEqual(re.findall(r"--ask=(\d+)", content), ["4", "4", "4"])
        self.assertIn("pacman -Qq quickshell", content)
        self.assertNotIn("noctalia-qs", content)
        self.assertNotIn("pacman -Rns", content)
        self.assertLess(
            content.index("pacman -Sp --noconfirm --ask=4 --print-format"),
            content.index('nonconflicting_packages='),
        )

    def test_arch_xry_meta_chain_is_preflighted_then_replaced(self) -> None:
        completed, calls = self._run_fake_arch(
            ("cachyos-niri-noctalia", "noctalia-shell", "noctalia-qs")
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        package_tail = "rust cmake gcc wl-clipboard libnotify"
        resolution = (
            "pacman -Sp --noconfirm --ask=4 --print-format %n "
            f"niri greetd quickshell cage foot noto-fonts {package_tail}"
        )
        dependencies = (
            "sudo pacman -S --noconfirm --needed "
            f"niri greetd cage foot noto-fonts {package_tail}"
        )
        build = (
            "cargo build --manifest-path "
            f"{ROOT / 'Cargo.toml'} --release --locked "
            "-p weyriva -p weyriva-luau-host"
        )
        preflight = "sudo " + str(ROOT / "scripts/install-system.sh") + " --preflight --user tester"
        cache = "sudo pacman -Sw --noconfirm --needed quickshell"
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
            removal_preflight,
            dependencies,
            cache,
            build,
            preflight,
            removal,
            replacement,
        ):
            self.assertIn(expected, calls)
        self.assertLess(calls.index(resolution), calls.index(removal_preflight))
        self.assertLess(calls.index(removal_preflight), calls.index(dependencies))
        self.assertLess(calls.index(dependencies), calls.index(cache))
        self.assertLess(calls.index(cache), calls.index(build))
        self.assertLess(calls.index(build), calls.index(preflight))
        self.assertLess(calls.index(preflight), calls.index(removal))
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
        package_tail = "rust cmake gcc wl-clipboard libnotify"
        resolution = (
            "pacman -Sp --noconfirm --ask=4 --print-format %n "
            f"niri greetd quickshell cage foot noto-fonts {package_tail}"
        )
        installation = (
            "sudo pacman -S --noconfirm --ask=4 --needed "
            f"niri greetd quickshell cage foot noto-fonts {package_tail}"
        )
        build = (
            "cargo build --manifest-path "
            f"{ROOT / 'Cargo.toml'} --release --locked "
            "-p weyriva -p weyriva-luau-host"
        )
        preflight = (
            "sudo " + str(ROOT / "scripts/install-system.sh")
            + " --preflight --user tester"
        )
        for expected in (resolution, installation, build, preflight):
            self.assertIn(expected, calls)
        self.assertLess(calls.index(resolution), calls.index(installation))
        self.assertLess(calls.index(installation), calls.index(build))
        self.assertLess(calls.index(build), calls.index(preflight))
        self.assertEqual(calls.count(installation), 1)
        self.assertNotIn("sudo pacman -R", calls)
        self.assertNotIn("greetd-dms-greeter-git", calls)
