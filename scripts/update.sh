#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

system_path() {
    local root_prefix=$1
    local absolute_path=$2
    if [[ $root_prefix == / ]]; then
        printf '%s\n' "$absolute_path"
    else
        printf '%s%s\n' "${root_prefix%/}" "$absolute_path"
    fi
}

has_safe_parent_chain() {
    local boundary=${1%/}
    local candidate=$2
    local current next
    [[ -n $boundary ]] || boundary=/
    current=$(dirname -- "$candidate")

    while true; do
        [[ -d $current && ! -L $current ]] || return 1
        [[ $current == "$boundary" ]] && return 0
        if [[ $boundary != / ]]; then
            [[ $current == "$boundary/"* ]] || return 1
        fi
        next=$(dirname -- "$current")
        [[ $next != "$current" ]] || return 1
        current=$next
    done
}

classify_system_install() {
    local root_prefix=$1
    local source_root=$2
    if [[ $root_prefix != /* || $source_root != /* ||
        ! -d $source_root || -L $source_root ]]; then
        printf '%s\n' partial
        return
    fi

    local -a required_executables=(
        /usr/bin/weyriva
        /usr/bin/weyriva-luau-host
    )
    local -a required_files=(
        /usr/share/weyriva/config/niri/config.kdl
        /usr/share/weyriva/greetd/config.toml
        /usr/share/weyriva/wallpapers/light/weyriva-cactus.png
        /usr/share/weyriva/wallpapers/dark/weyriva-cactus-dark.png
        /usr/share/wayland-sessions/weyriva.desktop
    )
    local -a source_trees=(shell greeter config/weyriva)
    local -a explicit_sources=(
        config/niri/config.kdl
        config/greetd/config.toml
        assets/wallpapers/weyriva-cactus.png
        assets/wallpapers/weyriva-cactus-dark.png
        user-share/wayland-sessions/weyriva.desktop
    )
    local -a required_links=(
        /usr/lib/systemd/user/niri.service.wants/weyriva-ipc.service
        /usr/lib/systemd/user/niri.service.wants/weyriva-shell.service
    )
    local -a required_link_targets=(
        ../weyriva-ipc.service
        ../weyriva-shell.service
    )
    local relative source tree unsafe_entry
    for relative in "${explicit_sources[@]}"; do
        source="$source_root/$relative"
        if [[ ! -f $source || -L $source ]] ||
            ! has_safe_parent_chain "$source_root" "$source"; then
            printf '%s\n' partial
            return
        fi
    done
    for tree in "${source_trees[@]}"; do
        source="$source_root/$tree"
        if [[ ! -d $source || -L $source ]] ||
            ! has_safe_parent_chain "$source_root" "$source"; then
            printf '%s\n' partial
            return
        fi
        unsafe_entry=$(
            find "$source" -mindepth 1 ! -type d ! -type f -print -quit
        )
        if [[ -n $unsafe_entry ]]; then
            printf '%s\n' partial
            return
        fi
        local tree_files=0
        while IFS= read -r -d '' source; do
            relative=${source#"$source_root"/}
            required_files+=("/usr/share/weyriva/$relative")
            ((tree_files += 1))
        done < <(find "$source" -type f -print0 | sort -z)
        if ((tree_files == 0)); then
            printf '%s\n' partial
            return
        fi
    done

    local systemd_root="$source_root/systemd"
    if [[ ! -d $systemd_root || -L $systemd_root ]] ||
        ! has_safe_parent_chain "$source_root" "$systemd_root"; then
        printf '%s\n' partial
        return
    fi
    unsafe_entry=$(
        find "$systemd_root" -mindepth 1 ! -type d ! -type f -print -quit
    )
    if [[ -n $unsafe_entry ]]; then
        printf '%s\n' partial
        return
    fi
    local unit_count=0
    while IFS= read -r -d '' source; do
        required_files+=("/usr/lib/systemd/user/${source##*/}")
        ((unit_count += 1))
    done < <(find "$systemd_root" -maxdepth 1 -type f -name '*.service' -print0 |
        sort -z)
    if ((unit_count == 0)); then
        printf '%s\n' partial
        return
    fi

    local share_root=/usr/share/weyriva
    local wants_root=/usr/lib/systemd/user/niri.service.wants
    local total=$((
        ${#required_executables[@]} +
            ${#required_files[@]} +
            ${#required_links[@]} +
            1
    ))
    local present=0
    local safe=0
    local path candidate index parent

    for path in "${required_executables[@]}"; do
        candidate=$(system_path "$root_prefix" "$path")
        if [[ -e $candidate || -L $candidate ]]; then
            ((present += 1))
            if [[ -f $candidate && ! -L $candidate && -x $candidate ]] &&
                has_safe_parent_chain "$root_prefix" "$candidate"; then
                ((safe += 1))
            fi
        fi
    done

    for path in "${required_files[@]}"; do
        candidate=$(system_path "$root_prefix" "$path")
        if [[ -e $candidate || -L $candidate ]]; then
            ((present += 1))
            if [[ -f $candidate && ! -L $candidate ]] &&
                has_safe_parent_chain "$root_prefix" "$candidate"; then
                ((safe += 1))
            fi
        fi
    done

    candidate=$(system_path "$root_prefix" "$share_root")
    if [[ -e $candidate || -L $candidate ]]; then
        ((present += 1))
        if [[ -d $candidate && ! -L $candidate ]] &&
            has_safe_parent_chain "$root_prefix" "$candidate"; then
            ((safe += 1))
        fi
    fi

    for index in "${!required_links[@]}"; do
        candidate=$(system_path "$root_prefix" "${required_links[$index]}")
        if [[ -e $candidate || -L $candidate ]]; then
            ((present += 1))
            if [[ -L $candidate &&
                $(readlink "$candidate") == "${required_link_targets[$index]}" ]] &&
                has_safe_parent_chain "$root_prefix" "$candidate"; then
                ((safe += 1))
            fi
        fi
    done

    parent=$(system_path "$root_prefix" "$wants_root")
    if ((present > 0)) &&
        [[ ! -d $parent || -L $parent ]]; then
        printf '%s\n' partial
        return
    fi

    if ((present == 0)); then
        printf '%s\n' user-only
    elif ((present == total && safe == total)); then
        printf '%s\n' system
    else
        printf '%s\n' partial
    fi
}

run_update() {
    local system_root=$1
    local source_root=$2
    shift 2
    local apply=0
    if [[ $# -eq 1 && $1 == --apply ]]; then
        apply=1
    elif [[ $# -ne 0 ]]; then
        printf '%s\n' "Usage: $0 [--apply]" >&2
        return 2
    fi

    if command -v pacman >/dev/null &&
        pacman -Q weyriva-shell-git >/dev/null 2>&1; then
        printf '%s\n' "This is an AUR-managed installation. Update it with your AUR helper:"
        printf '%s\n' "  paru -Syu weyriva-shell-git"
        return 0
    fi
    if [[ ! -d $ROOT/.git ]]; then
        printf '%s\n' "No Git checkout detected. Download a new release or use the AUR package." >&2
        return 1
    fi

    local installation
    installation=$(classify_system_install "$system_root" "$source_root")
    if [[ $installation == partial ]]; then
        printf '%s\n' \
            "Partial or unsafe Weyriva system footprint detected; run $ROOT/install.sh to repair it before updating." >&2
        return 1
    fi

    local state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
    local user_state="$state_home/weyriva/installed-files.tsv"
    local synchronize_user=false
    if [[ -e $user_state || -L $user_state ]]; then
        if [[ ! -f $user_state || -L $user_state ]]; then
            printf 'Unsafe Weyriva user-install state manifest: %s\n' "$user_state" >&2
            return 1
        fi
        synchronize_user=true
    fi

    if ((apply == 0)); then
        if [[ $installation == system ]]; then
            printf 'Dry-run: complete system installation detected; would fast-forward %s, then run %s/install.sh.\n' \
                "$ROOT" "$ROOT"
            if [[ $synchronize_user == true ]]; then
                printf 'Dry-run: existing user-managed overrides detected; would then run %s/install.sh --apply from the same revision.\n' \
                    "$SCRIPT_DIR"
            fi
        else
            printf 'Dry-run: no system footprint detected; would fast-forward %s, build both locked Rust release binaries, then run %s/install.sh --apply.\n' \
                "$ROOT" "$SCRIPT_DIR"
        fi
        return 0
    fi

    git -C "$ROOT" pull --ff-only
    if [[ $installation == system ]]; then
        "$ROOT/install.sh"
        if [[ $synchronize_user == true ]]; then
            "$SCRIPT_DIR/install.sh" --apply
        fi
        return 0
    fi

    # shellcheck disable=SC1091 # Resolved from this checkout after the pull.
    source "$SCRIPT_DIR/libinstall.sh"
    build_release_binaries
    "$SCRIPT_DIR/install.sh" --apply
}

main() {
    run_update / "$ROOT" "$@"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    main "$@"
fi
