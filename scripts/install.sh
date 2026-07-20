#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source-path=SCRIPTDIR
# shellcheck source=libinstall.sh
source "$SCRIPT_DIR/libinstall.sh"
parse_install_flags "$@"
load_state

CONFIG_HOME=${XDG_CONFIG_HOME:-"$HOME/.config"}
DATA_HOME=${XDG_DATA_HOME:-"$HOME/.local/share"}
BIN_HOME="$HOME/.local/bin"
SYSTEMD_HOME="$CONFIG_HOME/systemd/user"

log "Weyriva user installation ($([[ $WEYRIVA_APPLY -eq 1 ]] && echo apply || echo dry-run))"
install_file "$WEYRIVA_ROOT/bin/weyriva" "$BIN_HOME/weyriva" 0755
install_tree "$WEYRIVA_ROOT/config/niri" "$CONFIG_HOME/niri"
install_tree "$WEYRIVA_ROOT/config/waybar" "$CONFIG_HOME/waybar"
install_tree "$WEYRIVA_ROOT/config/fuzzel" "$CONFIG_HOME/fuzzel"
install_tree "$WEYRIVA_ROOT/config/mako" "$CONFIG_HOME/mako"
install_tree "$WEYRIVA_ROOT/systemd" "$SYSTEMD_HOME"
install_tree "$WEYRIVA_ROOT/assets/wallpapers" "$DATA_HOME/weyriva/wallpapers"
remove_obsolete_managed
write_state

if [[ $WEYRIVA_APPLY -eq 1 ]]; then
    log "Files installed. Test from a TTY with: $BIN_HOME/weyriva session start"
    log "If already in a user session, run: systemctl --user daemon-reload"
else
    log "Dry-run only. Re-run with --apply; add --force only to back up and replace conflicts."
fi
log "greetd selection requires a system/AUR install; its config step remains separate."
