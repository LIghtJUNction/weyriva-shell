#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
APPLY=0
[[ ${1:-} == "--apply" ]] && APPLY=1

if command -v pacman >/dev/null && pacman -Q weyriva-shell-git >/dev/null 2>&1; then
    printf '%s\n' "This is an AUR-managed installation. Update it with your AUR helper:"
    printf '%s\n' "  paru -Syu weyriva-shell-git"
    exit 0
fi
if [[ ! -d "$ROOT/.git" ]]; then
    printf '%s\n' "No Git checkout detected. Download a new release or use the AUR package." >&2
    exit 1
fi
if [[ $APPLY -ne 1 ]]; then
    printf '%s\n' "Dry-run: would run 'git pull --ff-only' in $ROOT, then the preservation-first installer."
    printf '%s\n' "Run $0 --apply to continue. Existing configs are still preserved."
    exit 0
fi
git -C "$ROOT" pull --ff-only
"$SCRIPT_DIR/install.sh" --apply
