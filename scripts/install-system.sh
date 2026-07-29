#!/usr/bin/env bash
set -euo pipefail

fail() {
    printf 'weyriva system install: %s\n' "$*" >&2
    exit 1
}

PREFLIGHT=false
if [[ $# -eq 3 && $1 == --preflight && $2 == --user && -n $3 && $3 != root ]]; then
    PREFLIGHT=true
    TARGET_USER=$3
elif [[ $# -eq 2 && $1 == --user && -n $2 && $2 != root ]]; then
    TARGET_USER=$2
else
    fail 'usage: scripts/install-system.sh [--preflight] --user USER'
fi
[[ $EUID -eq 0 ]] || fail 'root privileges are required'
command -v systemctl >/dev/null 2>&1 || fail 'systemd is required'
command -v getent >/dev/null 2>&1 || fail 'getent is required'
command -v runuser >/dev/null 2>&1 || fail 'runuser is required'
[[ -x /usr/bin/env ]] || fail '/usr/bin/env is required for the isolated greeter environment'
for command_name in niri niri-session quickshell cage foot; do
    command -v "$command_name" >/dev/null 2>&1 ||
        fail "required command is unavailable: $command_name"
done
systemd_version=$(systemctl --version | awk 'NR == 1 { print $2 }')
[[ $systemd_version =~ ^[0-9]+$ && $systemd_version -ge 254 ]] ||
    fail 'systemd 254 or newer is required'

target_uid=$(id -u "$TARGET_USER") || fail "unknown desktop user: $TARGET_USER"
greeter_uid=$(id -u greeter 2>/dev/null) ||
    fail 'the greeter account is required'
greeter_gid=$(getent group greeter | cut -d: -f3)
[[ -n $greeter_gid && $(id -g greeter) == "$greeter_gid" ]] ||
    fail 'the greeter account must use the greeter group'
[[ -f /etc/pam.d/greetd && ! -L /etc/pam.d/greetd ]] ||
    fail 'a regular distro-provided /etc/pam.d/greetd stack is required'
[[ -d /etc/greetd && ! -L /etc/greetd ]] ||
    fail 'a regular distro-provided /etc/greetd directory is required'
display_manager_link=/etc/systemd/system/display-manager.service
[[ ! -e $display_manager_link && ! -L $display_manager_link ||
    -L $display_manager_link ]] ||
    fail "display-manager destination is not a symlink: $display_manager_link"
awk '
    /^[[:space:]]*#/ || NF == 0 { next }
    {
        type = $1
        sub(/^-/, "", type)
        if (type == "session" && NF >= 3) found = 1
    }
    END { exit(found ? 0 : 1) }
' /etc/pam.d/greetd ||
    fail 'the greetd PAM stack must contain an active session rule or include'
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
[[ -f $ROOT/bin/weyriva && -f $ROOT/config/greetd/config.toml ]] ||
    fail "incomplete Weyriva checkout: $ROOT"

target_home=$(getent passwd "$TARGET_USER" | cut -d: -f6)
[[ $target_home == /* && -d $target_home && ! -L $target_home ]] ||
    fail "the desktop user home is unavailable or unsafe: $target_home"
user_niri_config="$target_home/.config/niri/config.kdl"
[[ ! -L $user_niri_config ]] ||
    fail "refusing to replace symlink: $user_niri_config"
[[ ! -e $user_niri_config || -f $user_niri_config ]] ||
    fail "destination is not a regular file: $user_niri_config"
niri validate -c "$ROOT/config/niri/config.kdl"
if [[ -f $user_niri_config ]]; then
    niri validate -c "$user_niri_config"
fi
niri_config_changed=true
if [[ -f $user_niri_config ]] &&
    cmp -s "$ROOT/config/niri/config.kdl" "$user_niri_config"; then
    niri_config_changed=false
fi
unit_verify_dir=$(mktemp -d /tmp/weyriva-systemd-verify.XXXXXX)
chmod 0755 "$unit_verify_dir"
install -m 0755 "$ROOT/bin/weyriva" "$unit_verify_dir/weyriva"
for source in "$ROOT"/systemd/*.service; do
    sed "s#/usr/bin/weyriva#$unit_verify_dir/weyriva#g" \
        "$source" >"$unit_verify_dir/${source##*/}"
done
if ! runuser -u "$TARGET_USER" -- \
    env XDG_RUNTIME_DIR="/run/user/$target_uid" \
    systemd-analyze --user verify "$unit_verify_dir"/*.service; then
    rm -r -- "$unit_verify_dir"
    fail 'systemd user-unit verification failed'
fi
rm -r -- "$unit_verify_dir"
systemctl cat greetd.service >/dev/null ||
    fail 'greetd.service is unavailable'

INSTALL_TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_ROOT="/var/lib/weyriva/install-backups/$INSTALL_TIMESTAMP"

validate_safe_parent() {
    local destination=$1
    local parent
    parent=$(dirname "$destination")
    while [[ $parent != / ]]; do
        [[ ! -L $parent ]] || fail "refusing path containing a symlink: $parent"
        [[ ! -e $parent || -d $parent ]] ||
            fail "path component is not a directory: $parent"
        parent=$(dirname "$parent")
    done
}

validate_file_destination() {
    local source=$1
    local destination=$2
    [[ -f $source && ! -L $source ]] ||
        fail "required source file is unavailable or unsafe: $source"
    validate_safe_parent "$destination"
    [[ ! -L $destination ]] || fail "refusing to replace symlink: $destination"
    [[ ! -e $destination || -f $destination ]] ||
        fail "destination is not a regular file: $destination"
}

validate_directory_destination() {
    local destination=$1
    validate_safe_parent "$destination/placeholder"
    [[ ! -L $destination ]] || fail "refusing symlinked directory: $destination"
    [[ ! -e $destination || -d $destination ]] ||
        fail "destination is not a directory: $destination"
}

declare -a install_sources=()
declare -a install_destinations=()
declare -a install_modes=()
plan_file() {
    install_sources+=("$1")
    install_destinations+=("$2")
    install_modes+=("${3:-0644}")
}

plan_file "$ROOT/bin/weyriva" /usr/bin/weyriva 0755
plan_file "$ROOT/config/niri/config.kdl" /usr/share/weyriva/config/niri/config.kdl
for source_root in "$ROOT/shell" "$ROOT/greeter" "$ROOT/config/weyriva"; do
    while IFS= read -r -d '' source; do
        relative=${source#"$ROOT"/}
        plan_file "$source" "/usr/share/weyriva/$relative"
    done < <(find "$source_root" -type f -print0 | sort -z)
done
plan_file "$ROOT/config/greetd/config.toml" /usr/share/weyriva/greetd/config.toml
plan_file \
    "$ROOT/assets/wallpapers/weyriva-cactus.png" \
    /usr/share/weyriva/wallpapers/light/weyriva-cactus.png
plan_file \
    "$ROOT/assets/wallpapers/weyriva-cactus-dark.png" \
    /usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png
plan_file \
    "$ROOT/user-share/wayland-sessions/weyriva.desktop" \
    /usr/share/wayland-sessions/weyriva.desktop
for source in "$ROOT"/systemd/*.service; do
    plan_file "$source" "/usr/lib/systemd/user/${source##*/}"
done

for index in "${!install_sources[@]}"; do
    validate_file_destination \
        "${install_sources[$index]}" \
        "${install_destinations[$index]}"
done

validate_file_destination "$ROOT/config/greetd/config.toml" /etc/greetd/config.toml
validate_directory_destination /var/lib/weyriva-greeter
for directory in state cache config; do
    validate_directory_destination "/var/lib/weyriva-greeter/$directory"
done
validate_directory_destination "$target_home/.config/systemd/user"
validate_file_destination "$ROOT/config/niri/config.kdl" "$user_niri_config"
validate_safe_parent "$target_home/.local/state/weyriva/startup-backups/placeholder"
startup_backup_root="$target_home/.local/state/weyriva/startup-backups/$INSTALL_TIMESTAMP"
[[ ! -e $startup_backup_root && ! -L $startup_backup_root ]] ||
    fail "startup backup destination already exists: $startup_backup_root"
user_niri_backup="$startup_backup_root/niri/config.kdl"
if [[ $niri_config_changed == true && -e $user_niri_config ]]; then
    [[ ! -e $user_niri_backup && ! -L $user_niri_backup ]] ||
        fail "startup backup destination already exists: $user_niri_backup"
fi
validate_safe_parent "$BACKUP_ROOT/placeholder"
[[ ! -e $BACKUP_ROOT && ! -L $BACKUP_ROOT ]] ||
    fail "system backup destination already exists: $BACKUP_ROOT"

wants_dir=/usr/lib/systemd/user/niri.service.wants
[[ -f /usr/lib/systemd/user/niri.service && ! -L /usr/lib/systemd/user/niri.service ]] ||
    fail 'niri.service is unavailable or unsafe'
validate_directory_destination "$wants_dir"
for unit in weyriva-ipc.service weyriva-shell.service; do
    link="$wants_dir/$unit"
    if [[ -L $link && $(readlink "$link") == "../$unit" ]]; then
        continue
    fi
    [[ ! -e $link && ! -L $link ]] ||
        fail "refusing to replace unexpected niri wants entry: $link"
done

if [[ $PREFLIGHT == true ]]; then
    printf 'System install preflight passed for %s; no system files were changed.\n' \
        "$TARGET_USER"
    exit 0
fi

backup_existing() {
    local destination=$1
    [[ -e $destination || -L $destination ]] || return 0
    [[ ! -L $destination ]] || fail "refusing to replace symlink: $destination"
    local backup="$BACKUP_ROOT/${destination#/}"
    install -d -m 0700 "$(dirname "$backup")"
    cp -a "$destination" "$backup"
}

install_system_file() {
    local source=$1
    local destination=$2
    local mode=${3:-0644}
    [[ ! -L $destination ]] || fail "refusing to replace symlink: $destination"
    if [[ -f $destination ]] && cmp -s "$source" "$destination"; then
        chmod "$mode" "$destination"
        return
    fi
    backup_existing "$destination"
    install -D -m "$mode" "$source" "$destination"
}

for index in "${!install_sources[@]}"; do
    install_system_file \
        "${install_sources[$index]}" \
        "${install_destinations[$index]}" \
        "${install_modes[$index]}"
done

install -d -m 0755 "$wants_dir"
for unit in weyriva-ipc.service weyriva-shell.service; do
    link="$wants_dir/$unit"
    if [[ -L $link && $(readlink "$link") == "../$unit" ]]; then
        continue
    fi
    [[ ! -e $link && ! -L $link ]] ||
        fail "refusing to replace unexpected niri wants entry: $link"
    ln -s "../$unit" "$link"
done

WEYRIVA_STARTUP_TIMESTAMP="$INSTALL_TIMESTAMP" \
    /usr/bin/weyriva startup ensure --user "$TARGET_USER"
user_runtime_dir="/run/user/$target_uid"
user_bus="$user_runtime_dir/bus"
if [[ -d $user_runtime_dir && ! -L $user_runtime_dir &&
    -S $user_bus && ! -L $user_bus ]]; then
    if ! runuser -u "$TARGET_USER" -- \
        env XDG_RUNTIME_DIR="$user_runtime_dir" \
        systemctl --user daemon-reload; then
        fail "failed to reload the active user manager for $TARGET_USER"
    fi
fi
printf 'System files installed for %s; greeter uid %s gid %s; greetd enabled for next boot and not restarted.\n' \
    "$TARGET_USER" "$greeter_uid" "$greeter_gid"
