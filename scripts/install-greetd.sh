#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} != "--apply" ]]; then
    printf '%s\n' "Dry-run: would validate and repair the Weyriva login chain."
    printf '%s\n' "This requires a system/AUR installation and never restarts greetd."
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
target_user=${SUDO_USER:-}
[[ -n $target_user && $target_user != root ]] || {
    printf '%s\n' "Run with sudo from the intended desktop user." >&2
    exit 1
}
exec /usr/bin/weyriva startup ensure --user "$target_user"
