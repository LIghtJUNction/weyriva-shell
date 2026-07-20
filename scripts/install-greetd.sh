#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SOURCE="$SCRIPT_DIR/../config/greetd/config.toml"
DESTINATION=/etc/greetd/config.toml

if [[ ${1:-} != "--apply" ]]; then
    printf '%s\n' "Dry-run: would install $SOURCE to $DESTINATION."
    printf '%s\n' "Requires a system/AUR installation providing /usr/bin/weyriva."
    printf '%s\n' "This system-wide step requires root and never enables or restarts greetd."
    printf '%s\n' "Run: sudo $0 --apply"
    exit 0
fi
if [[ ! -x /usr/bin/weyriva ]]; then
    printf '%s\n' "A system/AUR installation providing /usr/bin/weyriva is required." >&2
    exit 1
fi
if [[ $EUID -ne 0 ]]; then
    printf '%s\n' "Run this explicit system step as root." >&2
    exit 1
fi
if [[ -e $DESTINATION ]] && cmp -s "$SOURCE" "$DESTINATION"; then
    printf '%s\n' "$DESTINATION is already current."
    exit 0
fi
if [[ -e $DESTINATION ]]; then
    timestamp=$(date +%Y%m%d-%H%M%S)
    backup="${DESTINATION}.weyriva-backup-${timestamp}"
    cp -a "$DESTINATION" "$backup"
    printf '%s\n' "Backed up existing configuration to $backup."
fi
install -d -m 0755 /etc/greetd
install -m 0644 "$SOURCE" "$DESTINATION"
printf '%s\n' "Installed $DESTINATION. Review it before enabling or restarting greetd."
