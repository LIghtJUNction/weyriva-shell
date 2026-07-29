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

log "Weyriva user installation ($([[ $WEYRIVA_APPLY -eq 1 ]] && echo apply || echo dry-run))"
install_file "$WEYRIVA_ROOT/bin/weyriva" "$BIN_HOME/weyriva" 0755
install_tree "$WEYRIVA_ROOT/config/niri" "$CONFIG_HOME/niri"
install_tree "$WEYRIVA_ROOT/config/weyriva" "$CONFIG_HOME/weyriva"
install_tree "$WEYRIVA_ROOT/shell" "$DATA_HOME/weyriva/shell"
install_file \
    "$WEYRIVA_ROOT/assets/wallpapers/weyriva-cactus.png" \
    "$DATA_HOME/weyriva/wallpapers/light/weyriva-cactus.png"
install_file \
    "$WEYRIVA_ROOT/assets/wallpapers/weyriva-cactus-dark.png" \
    "$DATA_HOME/weyriva/wallpapers/dark/weyriva-cactus-dark.png"
remove_obsolete_managed
write_state

if [[ $WEYRIVA_APPLY -eq 1 ]]; then
    log "Files installed. Test from a TTY with: $BIN_HOME/weyriva session start"
else
    log "Dry-run only. Re-run with --apply; add --force only to back up and replace conflicts."
fi
log "The Weyriva login surface requires the root one-command or AUR system installation."
