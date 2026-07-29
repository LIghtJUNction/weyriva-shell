#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf '%s\n' 'Usage: ./install.sh' >&2
}

fail() {
    printf 'weyriva install: %s\n' "$*" >&2
    exit 1
}

if [[ $# -ne 0 ]]; then
    usage
    exit 2
fi

[[ $(uname -s) == Linux ]] || fail 'Weyriva requires Linux and Niri/Wayland.'

managers=()
for candidate in pacman dnf apt-get zypper; do
    if command -v "$candidate" >/dev/null 2>&1; then
        managers+=("$candidate")
    fi
done

if [[ ${#managers[@]} -ne 1 ]]; then
    fail 'expected exactly one supported package manager: pacman, dnf, apt-get, or zypper.'
fi

run_as_root() {
    if [[ ${EUID} -eq 0 ]]; then
        "$@"
        return
    fi
    command -v sudo >/dev/null 2>&1 || fail 'sudo is required to install desktop packages.'
    sudo "$@"
}

case ${managers[0]} in
    pacman)
        run_as_root pacman -S --noconfirm --needed niri waybar fuzzel mako swaybg foot noto-fonts gsimplecal pavucontrol
        ;;
    dnf)
        run_as_root dnf install -y niri waybar fuzzel mako swaybg foot google-noto-sans-fonts pavucontrol
        ;;
    apt-get)
        run_as_root apt-get install -y niri waybar fuzzel mako swaybg foot fonts-noto-core pavucontrol
        ;;
    zypper)
        run_as_root zypper --non-interactive install niri waybar fuzzel mako swaybg foot google-noto-sans-fonts pavucontrol
        ;;
esac

for command_name in niri waybar fuzzel mako swaybg foot; do
    command -v "$command_name" >/dev/null 2>&1 || fail "required command is unavailable after package installation: $command_name"
done

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
"$SCRIPT_DIR/scripts/install.sh" --apply --force

if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user daemon-reload || true
fi

printf '%s\n' 'Weyriva is installed. Start a Niri session to use it.'
