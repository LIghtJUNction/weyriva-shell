from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from weyriva_test_support import ROOT


SHARE_FILES = (
    "config/niri/config.kdl",
    "greetd/config.toml",
    "wallpapers/light/weyriva-cactus.png",
    "wallpapers/dark/weyriva-cactus-dark.png",
    "shell/shell.qml",
    "shell/Weyriva/Panel.qml",
    "greeter/shell.qml",
    "config/weyriva/defaults.json",
)
SYSTEM_UNITS = (
    "weyriva-ipc.service",
    "weyriva-session-failsafe.service",
    "weyriva-shell.service",
)
WANTS = (
    ("weyriva-ipc.service", "../weyriva-ipc.service"),
    ("weyriva-shell.service", "../weyriva-shell.service"),
)


def make_update_harness(temporary: Path) -> tuple[Path, Path, Path, dict[str, str]]:
    project = temporary / "project"
    scripts = project / "scripts"
    fake_bin = temporary / "fake-bin"
    trace = temporary / "trace"
    scripts.mkdir(parents=True)
    fake_bin.mkdir()
    (project / ".git").mkdir()
    shutil.copy2(ROOT / "scripts/update.sh", scripts / "update.sh")

    source_files = (
        "config/niri/config.kdl",
        "config/greetd/config.toml",
        "assets/wallpapers/weyriva-cactus.png",
        "assets/wallpapers/weyriva-cactus-dark.png",
        "user-share/wayland-sessions/weyriva.desktop",
        "shell/shell.qml",
        "shell/Weyriva/Panel.qml",
        "greeter/shell.qml",
        "config/weyriva/defaults.json",
    )
    for relative in source_files:
        path = project / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"{relative}\n")
    for unit in SYSTEM_UNITS:
        path = project / "systemd" / unit
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"{unit}\n")

    executables = {
        project / "install.sh": (
            "#!/usr/bin/env bash\n"
            "printf 'system-install %s\\n' \"$*\" >>\"$TRACE\"\n"
        ),
        scripts / "install.sh": (
            "#!/usr/bin/env bash\n"
            "printf 'user-install %s\\n' \"$*\" >>\"$TRACE\"\n"
        ),
        fake_bin / "git": (
            "#!/usr/bin/env bash\n"
            "printf 'git %s\\n' \"$*\" >>\"$TRACE\"\n"
        ),
        fake_bin / "pacman": (
            "#!/usr/bin/env bash\n"
            "if [[ ${FAKE_AUR:-0} == 1 && ${1:-} == -Q && "
            "${2:-} == weyriva-shell-git ]]; then exit 0; fi\n"
            "exit 1\n"
        ),
    }
    for path, content in executables.items():
        path.write_text(content)
        path.chmod(0o755)
    (scripts / "libinstall.sh").write_text(
        "build_release_binaries() {\n"
        "    printf '%s\\n' build-release >>\"$TRACE\"\n"
        "}\n"
    )

    environment = {
        **os.environ,
        "HOME": str(temporary / "home"),
        "XDG_STATE_HOME": str(temporary / "state"),
        "PATH": f"{fake_bin}:/usr/bin:/bin",
        "TRACE": str(trace),
    }
    return scripts / "update.sh", temporary / "system-root", trace, environment


def run_sourced_update(
    script: Path,
    system_root: Path,
    environment: dict[str, str],
    *arguments: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "/usr/bin/bash",
            "-c",
            'source "$1"; shift; run_update "$@"',
            "bash",
            str(script),
            str(system_root),
            str(script.parents[1]),
            *arguments,
        ],
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )


def create_complete_system_footprint(system_root: Path) -> None:
    for relative in ("usr/bin/weyriva", "usr/bin/weyriva-luau-host"):
        path = system_root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("binary\n")
        path.chmod(0o755)
    for relative in SHARE_FILES:
        path = system_root / "usr/share/weyriva" / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"{relative}\n")
    desktop = system_root / "usr/share/wayland-sessions/weyriva.desktop"
    desktop.parent.mkdir(parents=True, exist_ok=True)
    desktop.write_text("desktop\n")
    for unit in SYSTEM_UNITS:
        path = system_root / "usr/lib/systemd/user" / unit
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"{unit}\n")
    wants_root = system_root / "usr/lib/systemd/user/niri.service.wants"
    wants_root.mkdir()
    for unit, target in WANTS:
        (wants_root / unit).symlink_to(target)
