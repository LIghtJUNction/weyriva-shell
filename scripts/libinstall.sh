#!/usr/bin/env bash

set -euo pipefail

# shellcheck disable=SC2034 # Exported to scripts that source this library.
WEYRIVA_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
WEYRIVA_APPLY=0
WEYRIVA_FORCE=0
WEYRIVA_TIMESTAMP=$(date +%Y%m%d-%H%M%S)
WEYRIVA_STATE_HOME=${XDG_STATE_HOME:-"$HOME/.local/state"}
WEYRIVA_STATE_DIR="$WEYRIVA_STATE_HOME/weyriva"
WEYRIVA_STATE_FILE="$WEYRIVA_STATE_DIR/installed-files.tsv"
declare -A WEYRIVA_HASHES=()
declare -A WEYRIVA_NEXT_HASHES=()
declare -A WEYRIVA_DESIRED=()

log() { printf '%s\n' "$*"; }

parse_install_flags() {
    while (($#)); do
        case "$1" in
            --apply) WEYRIVA_APPLY=1 ;;
            --force) WEYRIVA_FORCE=1 ;;
            -h|--help)
                log "Usage: $0 [--apply] [--force]"
                log "Without --apply, only a dry-run plan is printed."
                exit 0
                ;;
            *) log "Unknown option: $1" >&2; exit 2 ;;
        esac
        shift
    done
}

file_hash() { sha256sum "$1" | cut -d ' ' -f 1; }

load_state() {
    local digest destination
    [[ -f $WEYRIVA_STATE_FILE ]] || return 0
    while IFS=$'\t' read -r digest destination; do
        [[ -n $digest && -n $destination ]] || continue
        WEYRIVA_HASHES["$destination"]=$digest
        WEYRIVA_NEXT_HASHES["$destination"]=$digest
    done <"$WEYRIVA_STATE_FILE"
}

write_state() {
    [[ $WEYRIVA_APPLY -eq 1 ]] || return 0
    mkdir -p "$WEYRIVA_STATE_DIR"
    chmod 0700 "$WEYRIVA_STATE_DIR"
    local temporary="$WEYRIVA_STATE_FILE.tmp.$$"
    : >"$temporary"
    local destination
    for destination in "${!WEYRIVA_NEXT_HASHES[@]}"; do
        printf '%s\t%s\n' "${WEYRIVA_NEXT_HASHES[$destination]}" "$destination"
    done | sort -k2 >"$temporary"
    chmod 0600 "$temporary"
    mv "$temporary" "$WEYRIVA_STATE_FILE"
}

install_file() {
    local source=$1
    local destination=$2
    local mode=${3:-0644}
    local source_hash current_hash previous_hash
    WEYRIVA_DESIRED["$destination"]=1
    source_hash=$(file_hash "$source")
    previous_hash=${WEYRIVA_HASHES[$destination]:-}
    current_hash=
    [[ -f $destination ]] && current_hash=$(file_hash "$destination")
    if [[ -f "$destination" ]] && [[ $source_hash == "$current_hash" ]]; then
        if [[ -z $previous_hash ]]; then
            log "preserve   $destination (identical but not managed by Weyriva)"
        else
            log "unchanged  $destination"
        fi
        if [[ $WEYRIVA_APPLY -eq 1 && -n $previous_hash ]]; then
            WEYRIVA_NEXT_HASHES["$destination"]=$source_hash
        fi
        return
    fi
    if [[ -e "$destination" && $WEYRIVA_FORCE -ne 1 ]]; then
        if [[ -z $previous_hash ]]; then
            log "preserve   $destination (not managed; use --force to back up and replace)"
            return
        fi
        if [[ $current_hash != "$previous_hash" ]]; then
            log "preserve   $destination (modified since Weyriva installed it)"
            return
        fi
    fi
    if [[ $WEYRIVA_APPLY -ne 1 ]]; then
        if [[ -e "$destination" ]]; then
            log "would back up $destination and install replacement"
        else
            log "would install $destination"
        fi
        return
    fi
    mkdir -p "$(dirname "$destination")"
    if [[ -e "$destination" ]]; then
        local backup="${destination}.weyriva-backup-${WEYRIVA_TIMESTAMP}"
        cp -a "$destination" "$backup"
        log "backup     $backup"
    fi
    install -m "$mode" "$source" "$destination"
    WEYRIVA_NEXT_HASHES["$destination"]=$source_hash
    log "installed  $destination"
}

remove_obsolete_managed() {
    local destination installed_hash current_hash
    for destination in "${!WEYRIVA_HASHES[@]}"; do
        [[ -z ${WEYRIVA_DESIRED[$destination]:-} ]] || continue
        installed_hash=${WEYRIVA_HASHES[$destination]}
        if [[ -f $destination ]]; then
            current_hash=$(file_hash "$destination")
            if [[ $current_hash == "$installed_hash" ]]; then
                if [[ $WEYRIVA_APPLY -eq 1 ]]; then
                    rm -f "$destination"
                    log "removed obsolete $destination"
                else
                    log "would remove obsolete $destination"
                fi
            else
                log "preserve obsolete $destination (modified since Weyriva installed it)"
            fi
        fi
        if [[ $WEYRIVA_APPLY -eq 1 ]]; then
            unset 'WEYRIVA_NEXT_HASHES[$destination]'
        fi
    done
}

install_tree() {
    local source_root=$1
    local destination_root=$2
    while IFS= read -r -d '' source; do
        local relative=${source#"$source_root"/}
        install_file "$source" "$destination_root/$relative"
    done < <(find "$source_root" -type f -print0 | sort -z)
}
