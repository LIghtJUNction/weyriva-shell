#!/usr/bin/env bash
set -euo pipefail

APPLY=0
[[ ${1:-} == "--apply" ]] && APPLY=1
STATE_HOME=${XDG_STATE_HOME:-"$HOME/.local/state"}
STATE_DIR="$STATE_HOME/weyriva"
STATE_FILE="$STATE_DIR/installed-files.tsv"
NEXT_STATE="$STATE_DIR/installed-files.tsv.tmp.$$"

if [[ ! -f $STATE_FILE ]]; then
    printf '%s\n' "No Weyriva state manifest found; nothing can be removed safely."
    exit 0
fi
if [[ $APPLY -eq 1 ]]; then mkdir -p "$STATE_DIR"; : >"$NEXT_STATE"; fi
while IFS=$'\t' read -r installed_hash destination; do
    [[ -n $installed_hash && -n $destination ]] || continue
    if [[ ! -f $destination ]]; then continue; fi
    current_hash=$(sha256sum "$destination" | cut -d ' ' -f 1)
    if [[ $current_hash != "$installed_hash" ]]; then
        printf '%s\n' "preserve modified $destination"
        [[ $APPLY -eq 1 ]] && printf '%s\t%s\n' "$installed_hash" "$destination" >>"$NEXT_STATE"
    elif [[ $APPLY -eq 1 ]]; then
        rm -f "$destination"
        printf '%s\n' "removed $destination"
    else
        printf '%s\n' "would remove $destination"
    fi
done <"$STATE_FILE"

if [[ $APPLY -eq 1 ]]; then
    chmod 0600 "$NEXT_STATE"
    mv "$NEXT_STATE" "$STATE_FILE"
fi

if [[ $APPLY -ne 1 ]]; then printf '%s\n' "Dry-run only. Re-run with --apply to remove only unchanged Weyriva files."; fi
printf '%s\n' "System greetd configuration is never removed automatically."
