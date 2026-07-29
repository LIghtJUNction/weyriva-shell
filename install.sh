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

SCRIPT_DIR=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
for required_path in \
    "$SCRIPT_DIR/scripts/install-system.sh" \
    "$SCRIPT_DIR/bin/weyriva" \
    "$SCRIPT_DIR/config/niri/config.kdl" \
    "$SCRIPT_DIR/config/noctalia/config.toml" \
    "$SCRIPT_DIR/config/greetd/config.toml"; do
    [[ -f $required_path && ! -L $required_path ]] ||
        fail "incomplete or unsafe Weyriva checkout: $required_path"
done
[[ -x $SCRIPT_DIR/scripts/install-system.sh ]] ||
    fail "system installer is not executable: $SCRIPT_DIR/scripts/install-system.sh"

managers=()
for candidate in pacman dnf apt-get zypper; do
    if command -v "$candidate" >/dev/null 2>&1; then
        managers+=("$candidate")
    fi
done

if [[ ${#managers[@]} -ne 1 ]]; then
    fail 'expected exactly one supported package manager: pacman, dnf, apt-get, or zypper.'
fi

desktop_user=${SUDO_USER:-${USER:-}}
[[ -n $desktop_user && $desktop_user != root ]] ||
    fail 'cannot identify the ordinary desktop user; run this script as that user (with sudo available).'
id -u "$desktop_user" >/dev/null 2>&1 ||
    fail "cannot identify the ordinary desktop user: $desktop_user"
if [[ ${EUID} -ne 0 ]]; then
    command -v sudo >/dev/null 2>&1 ||
        fail 'sudo is required to install desktop packages.'
fi
command -v systemctl >/dev/null 2>&1 || fail 'systemd is required.'
systemd_version=$(systemctl --version | awk 'NR == 1 { print $2 }')
[[ $systemd_version =~ ^[0-9]+$ && $systemd_version -ge 254 ]] ||
    fail 'systemd 254 or newer is required.'

run_as_root() {
    if [[ ${EUID} -eq 0 ]]; then
        "$@"
        return
    fi
    sudo "$@"
}

run_as_invoking_user() {
    if [[ ${EUID} -ne 0 ]]; then
        "$@"
        return
    fi
    if command -v sudo >/dev/null 2>&1; then
        sudo -u "$desktop_user" -- "$@"
    elif command -v runuser >/dev/null 2>&1; then
        runuser -u "$desktop_user" -- "$@"
    else
        fail 'sudo or runuser is required to execute the AUR helper as the invoking ordinary user.'
    fi
}

case ${managers[0]} in
    pacman)
        aur_packages=()
        repository_packages=(niri greetd foot noto-fonts)
        aur_helper=
        for package_name in "${repository_packages[@]}"; do
            pacman -Si "$package_name" >/dev/null 2>&1 ||
                fail "repository package is unavailable: $package_name"
        done
        for specification in "noctalia:noctalia" "noctalia-greeter:noctalia-greeter-session"; do
            package_name=${specification%%:*}
            command_name=${specification#*:}
            if command -v "$command_name" >/dev/null 2>&1; then
                continue
            fi
            if pacman -Si "$package_name" >/dev/null 2>&1; then
                repository_packages+=("$package_name")
            else
                aur_packages+=("$package_name")
            fi
        done
        if [[ ${#aur_packages[@]} -gt 0 ]]; then
            for candidate in paru yay; do
                if command -v "$candidate" >/dev/null 2>&1; then
                    aur_helper=$(command -v "$candidate")
                    break
                fi
            done
            [[ -n $aur_helper ]] ||
                fail 'an installed AUR helper (paru or yay) is required for the resolved package plan.'
            for package_name in "${aur_packages[@]}"; do
                run_as_invoking_user "$aur_helper" -Si "$package_name" >/dev/null 2>&1 ||
                    fail "AUR package is unavailable: $package_name"
            done
        fi
        run_as_root pacman -S --noconfirm --needed "${repository_packages[@]}"
        if [[ ${#aur_packages[@]} -gt 0 ]]; then
            run_as_invoking_user "$aur_helper" -S --noconfirm --needed "${aur_packages[@]}"
        fi
        ;;
    dnf)
        run_as_root dnf install -y niri greetd noctalia noctalia-greeter foot google-noto-sans-fonts
        ;;
    apt-get)
        run_as_root apt-get install -y niri greetd noctalia noctalia-greeter foot fonts-noto-core
        ;;
    zypper)
        run_as_root zypper --non-interactive install niri greetd noctalia noctalia-greeter foot google-noto-sans-fonts
        ;;
esac

for command_name in niri niri-session noctalia noctalia-greeter-session foot; do
    command -v "$command_name" >/dev/null 2>&1 || fail "required command is unavailable after package installation: $command_name"
done

run_as_root "$SCRIPT_DIR/scripts/install-system.sh" --user "$desktop_user"

printf '%s\n' 'Weyriva is installed. Reboot or log out, then choose Weyriva.'
