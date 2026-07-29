#!/usr/bin/env bash
set -euo pipefail

fail() {
    printf 'weyriva install: %s\n' "$*" >&2
    exit 1
}

[[ $# -eq 0 ]] || fail 'this installer takes no options'
[[ $(uname -s) == Linux ]] || fail 'Weyriva requires Linux and Niri/Wayland.'

SCRIPT_DIR=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
for required_path in \
    "$SCRIPT_DIR/scripts/install-system.sh" \
    "$SCRIPT_DIR/Cargo.toml" \
    "$SCRIPT_DIR/Cargo.lock" \
    "$SCRIPT_DIR/crates/weyriva/Cargo.toml" \
    "$SCRIPT_DIR/crates/weyriva-luau-host/Cargo.toml" \
    "$SCRIPT_DIR/config/niri/config.kdl" \
    "$SCRIPT_DIR/shell/shell.qml" \
    "$SCRIPT_DIR/greeter/shell.qml" \
    "$SCRIPT_DIR/config/greetd/config.toml"; do
    [[ -f $required_path && ! -L $required_path ]] ||
        fail "incomplete or unsafe Weyriva checkout: $required_path"
done
[[ -x $SCRIPT_DIR/scripts/install-system.sh ]] ||
    fail "system installer is not executable: $SCRIPT_DIR/scripts/install-system.sh"
os_release=$(readlink -f -- /etc/os-release) ||
    fail 'cannot resolve /etc/os-release'
case "$os_release" in
    /etc/os-release|/usr/lib/os-release) ;;
    *) fail "untrusted /etc/os-release target: $os_release" ;;
esac
[[ -f $os_release && ! -L $os_release ]] ||
    fail 'the resolved os-release target must be a regular file'
[[ $(stat -c %u "$os_release") == 0 ]] ||
    fail 'the resolved os-release target must be owned by root'
os_release_mode=$(stat -c %A "$os_release")
[[ ${os_release_mode:5:1} != w && ${os_release_mode:8:1} != w ]] ||
    fail 'the resolved os-release target must not be group- or world-writable'

# shellcheck source=/dev/null
source "$os_release"
distro_tokens=" ${ID,,} ${ID_LIKE:-} "
distro_tokens=${distro_tokens,,}
case "$distro_tokens" in
    *" arch "*) distro=arch; manager=pacman ;;
    *" fedora "*|*" rhel "*) distro=fedora; manager=dnf ;;
    *" debian "*|*" ubuntu "*) distro=debian; manager=apt-get ;;
    *" opensuse "*|*" suse "*) distro=opensuse; manager=zypper ;;
    *) fail "unsupported distribution: ${ID:-unknown} (ID_LIKE=${ID_LIKE:-})" ;;
esac
command -v "$manager" >/dev/null 2>&1 ||
    fail "$distro requires its matching package manager: $manager"

desktop_user=${SUDO_USER:-${USER:-}}
[[ -n $desktop_user && $desktop_user != root ]] ||
    fail 'cannot identify the ordinary desktop user; run this script as that user (with sudo available).'
id -u "$desktop_user" >/dev/null 2>&1 ||
    fail "cannot identify the ordinary desktop user: $desktop_user"
if [[ $EUID -ne 0 ]]; then
    command -v sudo >/dev/null 2>&1 ||
        fail 'sudo is required to install desktop packages.'
fi
for command_name in systemctl getent runuser; do
    command -v "$command_name" >/dev/null 2>&1 ||
        fail "$command_name is required"
done
systemd_version=$(systemctl --version | awk 'NR == 1 { print $2 }')
[[ $systemd_version =~ ^[0-9]+$ && $systemd_version -ge 254 ]] ||
    fail 'systemd 254 or newer is required.'

desktop_home=$(getent passwd "$desktop_user" | cut -d: -f6)
[[ $desktop_home == /* && -d $desktop_home ]] ||
    fail "cannot determine the ordinary desktop user's home: $desktop_user"

run_as_root() {
    if [[ $EUID -eq 0 ]]; then
        "$@"
    else
        sudo "$@"
    fi
}

resolve_arch() {
    local package_name
    for package_name in "${packages[@]}"; do
        pacman -Si "$package_name" >/dev/null 2>&1 ||
            fail "repository package is unavailable: $package_name"
    done
    pacman -Sp --noconfirm --ask=4 --print-format '%n' \
        "${packages[@]}" >/dev/null 2>&1 ||
        fail 'the complete Arch package transaction cannot be resolved'

    blocking_packages=()
    for package_name in cachyos-niri-noctalia noctalia-shell; do
        if pacman -Qq "$package_name" >/dev/null 2>&1; then
            blocking_packages+=("$package_name")
        fi
    done
    if [[ ${#blocking_packages[@]} -gt 0 ]]; then
        pacman -R --print --print-format '%n' \
            "${blocking_packages[@]}" >/dev/null 2>&1 ||
            fail 'the legacy shell package removal cannot be resolved'
    fi
}

resolve_fedora() {
    local package_name
    dnf repoquery --help >/dev/null 2>&1 ||
        fail 'dnf repoquery support is required for deterministic preflight'
    for package_name in "${packages[@]}"; do
        dnf repoquery --available "$package_name" 2>/dev/null |
            grep -q . || fail "repository package is unavailable: $package_name"
    done
}

resolve_debian() {
    local package_name
    command -v apt-cache >/dev/null 2>&1 ||
        fail 'apt-cache is required for deterministic preflight'
    for package_name in "${packages[@]}"; do
        apt-cache show "$package_name" >/dev/null 2>&1 ||
            fail "repository package is unavailable: $package_name"
    done
    apt-get --simulate install "${packages[@]}" >/dev/null ||
        fail 'the complete apt package transaction cannot be resolved'
}

resolve_opensuse() {
    local package_name
    for package_name in "${packages[@]}"; do
        zypper --non-interactive info "$package_name" >/dev/null 2>&1 ||
            fail "repository package is unavailable: $package_name"
    done
    zypper --non-interactive --dry-run install "${packages[@]}" >/dev/null ||
        fail 'the complete zypper package transaction cannot be resolved'
}

case "$distro" in
    arch)
        packages=(
            niri greetd quickshell cage foot noto-fonts
            rust cmake gcc wl-clipboard libnotify
        )
        resolve_arch
        if [[ ${#blocking_packages[@]} -gt 0 ]]; then
            nonconflicting_packages=(
                niri greetd cage foot noto-fonts
                rust cmake gcc wl-clipboard libnotify
            )
            run_as_root pacman -S --noconfirm --needed \
                "${nonconflicting_packages[@]}"
            run_as_root pacman -Sw --noconfirm --needed quickshell
            arch_finalize=true
        else
            run_as_root pacman -S --noconfirm --ask=4 --needed "${packages[@]}"
            arch_finalize=false
        fi
        ;;
    fedora)
        packages=(
            niri greetd quickshell cage foot google-noto-sans-fonts
            rust cargo cmake gcc-c++ wl-clipboard libnotify
        )
        resolve_fedora
        run_as_root dnf install -y "${packages[@]}"
        ;;
    debian)
        packages=(
            niri greetd quickshell cage foot fonts-noto-core
            rustc cargo cmake g++ wl-clipboard libnotify-bin
        )
        resolve_debian
        run_as_root apt-get install -y "${packages[@]}"
        ;;
    opensuse)
        packages=(
            niri greetd quickshell cage foot google-noto-sans-fonts
            rust cargo cmake gcc-c++ wl-clipboard libnotify-tools
        )
        resolve_opensuse
        run_as_root zypper --non-interactive install "${packages[@]}"
        ;;
esac

for command_name in cargo rustc; do
    command -v "$command_name" >/dev/null 2>&1 ||
        fail "$command_name is unavailable after package installation"
done
rust_version=$(rustc --version | awk '{ print $2 }')
rust_major=${rust_version%%.*}
rust_minor_patch=${rust_version#*.}
rust_minor=${rust_minor_patch%%.*}
[[ $rust_major =~ ^[0-9]+$ && $rust_minor =~ ^[0-9]+$ ]] ||
    fail "cannot parse rustc version: $rust_version"
((rust_major > 1 || rust_major == 1 && rust_minor >= 88)) ||
    fail "rustc 1.88 or newer is required; found $rust_version"

build_command=(
    cargo build
    --manifest-path "$SCRIPT_DIR/Cargo.toml"
    --release
    --locked
    -p weyriva
    -p weyriva-luau-host
)
if [[ $EUID -eq 0 ]]; then
    runuser -u "$desktop_user" -- env HOME="$desktop_home" "${build_command[@]}"
else
    "${build_command[@]}"
fi

verify_elf() {
    local executable=$1
    local magic
    [[ -f $executable && ! -L $executable && -x $executable ]] ||
        fail "build did not produce a safe executable: $executable"
    magic=$(od -An -tx1 -N4 "$executable" | tr -d '[:space:]')
    [[ $magic == 7f454c46 ]] || fail "build output is not ELF: $executable"
}
verify_elf "$SCRIPT_DIR/target/release/weyriva"
verify_elf "$SCRIPT_DIR/target/release/weyriva-luau-host"

for command_name in niri niri-session quickshell cage foot wl-copy notify-send; do
    command -v "$command_name" >/dev/null 2>&1 ||
        fail "required command is unavailable after package installation: $command_name"
done

run_as_root "$SCRIPT_DIR/scripts/install-system.sh" \
    --preflight --user "$desktop_user"
if [[ ${arch_finalize:-false} == true ]]; then
    run_as_root pacman -R --noconfirm "${blocking_packages[@]}"
    run_as_root pacman -S --noconfirm --ask=4 --needed quickshell
    pacman -Qq quickshell >/dev/null 2>&1 ||
        fail 'generic quickshell package was not installed'
fi
run_as_root "$SCRIPT_DIR/scripts/install-system.sh" --user "$desktop_user"

printf '%s\n' 'Weyriva is installed. Reboot or log out, then choose Weyriva.'
