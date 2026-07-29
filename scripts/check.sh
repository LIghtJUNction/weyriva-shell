#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CHECK_TMP=$(mktemp -d)
trap 'rm -rf "$CHECK_TMP"' EXIT

printf '%s\n' '[check] Python compile'
PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" python3 -m py_compile "$ROOT/bin/weyriva" "$ROOT/examples/plugins/hello/hello.py" "$ROOT/tests/test_weyriva.py"

printf '%s\n' '[check] Python unit tests'
PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" python3 -m unittest discover -s "$ROOT/tests" -v

printf '%s\n' '[check] Bash syntax'
bash -n "$ROOT/install.sh"
while IFS= read -r -d '' script; do bash -n "$script"; done < <(find "$ROOT/scripts" -name '*.sh' -type f -print0)

printf '%s\n' '[check] JSON, TOML, INI, and desktop syntax'
python3 - "$ROOT" <<'PY'
import configparser
import json
import sys
import tomllib
from pathlib import Path

root = Path(sys.argv[1])
for path in (*root.rglob("*.json"), *root.rglob("*.jsonc")):
    json.loads(path.read_text(encoding="utf-8"))
for path in root.rglob("*.toml"):
    with path.open("rb") as stream:
        tomllib.load(stream)
for relative in ("user-share/wayland-sessions/weyriva.desktop",):
    parser = configparser.ConfigParser(interpolation=None)
    with (root / relative).open(encoding="utf-8") as stream:
        parser.read_file(stream)
PY

printf '%s\n' '[check] Installer dry-run and isolated HOME behavior'
INSTALL_HOME="$CHECK_TMP/home"
mkdir -p "$INSTALL_HOME"
HOME="$INSTALL_HOME" XDG_CONFIG_HOME="$INSTALL_HOME/config" XDG_DATA_HOME="$INSTALL_HOME/data" "$ROOT/scripts/install.sh" >/dev/null
if find "$INSTALL_HOME" -mindepth 1 -print -quit | grep -q .; then
    printf '%s\n' 'dry-run wrote into temporary HOME' >&2
    exit 1
fi
HOME="$INSTALL_HOME" XDG_CONFIG_HOME="$INSTALL_HOME/config" XDG_DATA_HOME="$INSTALL_HOME/data" "$ROOT/scripts/install.sh" --apply >/dev/null
test ! -e "$INSTALL_HOME/data/wayland-sessions/weyriva.desktop"
printf '\n' >>"$INSTALL_HOME/config/weyriva/defaults.json"
LOCAL_DEFAULTS_HASH=$(sha256sum "$INSTALL_HOME/config/weyriva/defaults.json" | cut -d ' ' -f 1)
HOME="$INSTALL_HOME" XDG_CONFIG_HOME="$INSTALL_HOME/config" XDG_DATA_HOME="$INSTALL_HOME/data" "$ROOT/scripts/install.sh" --apply >/dev/null
test "$(sha256sum "$INSTALL_HOME/config/weyriva/defaults.json" | cut -d ' ' -f 1)" = "$LOCAL_DEFAULTS_HASH"

printf '%s\n' '[check] Identical pre-existing files remain unowned'
UNOWNED_HOME="$CHECK_TMP/unowned-home"
mkdir -p "$UNOWNED_HOME/config/weyriva"
cp "$ROOT/config/weyriva/defaults.json" "$UNOWNED_HOME/config/weyriva/defaults.json"
UNOWNED_ENV=(env HOME="$UNOWNED_HOME" XDG_CONFIG_HOME="$UNOWNED_HOME/config" XDG_DATA_HOME="$UNOWNED_HOME/data" XDG_STATE_HOME="$UNOWNED_HOME/state")
"${UNOWNED_ENV[@]}" "$ROOT/scripts/install.sh" --apply >/dev/null
if grep -Fq "$UNOWNED_HOME/config/weyriva/defaults.json" "$UNOWNED_HOME/state/weyriva/installed-files.tsv"; then
    printf '%s\n' 'identical pre-existing file was incorrectly adopted' >&2
    exit 1
fi
"${UNOWNED_ENV[@]}" "$ROOT/scripts/uninstall.sh" --apply >/dev/null
cmp -s "$ROOT/config/weyriva/defaults.json" "$UNOWNED_HOME/config/weyriva/defaults.json"

printf '%s\n' '[check] Managed update and uninstall behavior'
PROJECT_COPY="$CHECK_TMP/project"
cp -a "$ROOT" "$PROJECT_COPY"
MANAGED_HOME="$CHECK_TMP/managed-home"
mkdir -p "$MANAGED_HOME"
MANAGED_ENV=(env HOME="$MANAGED_HOME" XDG_CONFIG_HOME="$MANAGED_HOME/config" XDG_DATA_HOME="$MANAGED_HOME/data" XDG_STATE_HOME="$MANAGED_HOME/state")
"${MANAGED_ENV[@]}" "$PROJECT_COPY/scripts/install.sh" --apply >/dev/null
RETIRED_CONFIG="$MANAGED_HOME/config/waybar/style.css"
mkdir -p "$(dirname "$RETIRED_CONFIG")"
printf '%s\n' 'previously managed Waybar config' >"$RETIRED_CONFIG"
RETIRED_HASH=$(sha256sum "$RETIRED_CONFIG" | cut -d ' ' -f 1)
printf '%s\t%s\n' "$RETIRED_HASH" "$RETIRED_CONFIG" >>"$MANAGED_HOME/state/weyriva/installed-files.tsv"
printf '\n' >>"$PROJECT_COPY/config/weyriva/defaults.json"
printf '%s\n' '// local modification' >>"$MANAGED_HOME/config/niri/config.kdl"
printf '%s\n' '// locally preserved shell module' >>"$MANAGED_HOME/data/weyriva/shell/Weyriva/Theme.qml"
rm -f "$PROJECT_COPY/shell/Weyriva/Theme.qml"
"${MANAGED_ENV[@]}" "$PROJECT_COPY/scripts/install.sh" --apply >/dev/null
cmp -s "$PROJECT_COPY/config/weyriva/defaults.json" "$MANAGED_HOME/config/weyriva/defaults.json"
grep -qx '// local modification' "$MANAGED_HOME/config/niri/config.kdl"
grep -qx '// locally preserved shell module' "$MANAGED_HOME/data/weyriva/shell/Weyriva/Theme.qml"
test ! -e "$RETIRED_CONFIG"
if grep -Fq "$MANAGED_HOME/data/weyriva/shell/Weyriva/Theme.qml" "$MANAGED_HOME/state/weyriva/installed-files.tsv" || grep -Fq "$RETIRED_CONFIG" "$MANAGED_HOME/state/weyriva/installed-files.tsv"; then
    printf '%s\n' 'obsolete files remained in the ownership manifest' >&2
    exit 1
fi
"${MANAGED_ENV[@]}" "$PROJECT_COPY/scripts/uninstall.sh" --apply >/dev/null
test ! -e "$MANAGED_HOME/config/weyriva/defaults.json"
grep -qx '// local modification' "$MANAGED_HOME/config/niri/config.kdl"
test -e "$MANAGED_HOME/data/weyriva/shell/Weyriva/Theme.qml"

printf '%s\n' '[check] Repository text whitespace and final newlines'
python3 - "$ROOT" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
excluded = {root / ".git", root / "__pycache__"}
for path in root.rglob("*"):
    if not path.is_file() or any(parent in excluded for parent in path.parents):
        continue
    data = path.read_bytes()
    if b"\0" in data:
        continue
    if data and not data.endswith(b"\n"):
        raise SystemExit(f"missing final newline: {path.relative_to(root)}")
    for number, line in enumerate(data.splitlines(), 1):
        if line.rstrip(b" \t") != line:
            raise SystemExit(f"trailing whitespace: {path.relative_to(root)}:{number}")
PY

if command -v shellcheck >/dev/null; then
    printf '%s\n' '[check] shellcheck'
    shellcheck -x "$ROOT"/scripts/*.sh
else
    printf '%s\n' '[skip] shellcheck is not installed'
fi

if command -v niri >/dev/null; then
    printf '%s\n' '[check] niri configuration'
    niri validate -c "$ROOT/config/niri/config.kdl"
else
    printf '%s\n' '[skip] niri is not installed'
fi

if command -v systemd-analyze >/dev/null; then
    printf '%s\n' '[check] systemd user units'
    systemd-analyze --user verify "$ROOT"/systemd/*.service
else
    printf '%s\n' '[skip] systemd-analyze is not installed'
fi

QT6_QMLLINT=
if [[ -x /usr/lib/qt6/bin/qmllint ]]; then
    QT6_QMLLINT=/usr/lib/qt6/bin/qmllint
elif command -v qmllint6 >/dev/null; then
    QT6_QMLLINT=$(command -v qmllint6)
fi
if [[ -n "$QT6_QMLLINT" ]]; then
    printf '%s\n' '[check] Qt 6 QML lint'
    QML_IMPORT_ROOT="$CHECK_TMP/qml-imports"
    mkdir -p "$QML_IMPORT_ROOT/qs"
    ln -s "$ROOT/shell/Weyriva" "$QML_IMPORT_ROOT/qs/Weyriva"
    mapfile -d '' QML_FILES < <(
        find "$ROOT/shell" "$ROOT/greeter" -name '*.qml' -type f -print0 |
            sort -z
    )
    "$QT6_QMLLINT" \
        --ignore-settings \
        --max-warnings 0 \
        --uncreatable-type disable \
        --unqualified disable \
        -I /usr/lib/qt6/qml \
        -I "$QML_IMPORT_ROOT" \
        "${QML_FILES[@]}"
else
    printf '%s\n' '[skip] Qt 6 qmllint is not installed'
fi

printf '%s\n' '[check] forbidden runtime dependency scan'
FORBIDDEN_RUNTIME_MATCHES="$CHECK_TMP/forbidden-runtime.txt"
if rg -n -i 'noctalia' \
    "$ROOT/bin" \
    "$ROOT/config" \
    "$ROOT/greeter" \
    "$ROOT/packaging" \
    "$ROOT/shell" \
    "$ROOT/systemd" \
    "$ROOT/user-share" \
    "$ROOT/scripts/install-system.sh" \
    "$ROOT/scripts/install.sh" \
    >"$FORBIDDEN_RUNTIME_MATCHES"; then
    printf '%s\n' 'forbidden Noctalia runtime/config/delegation reference found:' >&2
    cat "$FORBIDDEN_RUNTIME_MATCHES" >&2
    exit 1
fi
if [[ $(rg -o -i '(cachyos-niri-noctalia|noctalia-shell)' "$ROOT/install.sh" |
    sort -u | paste -sd ' ' -) != 'cachyos-niri-noctalia noctalia-shell' ]]; then
    printf '%s\n' 'Arch legacy conflict removal must name exactly cachyos-niri-noctalia and noctalia-shell' >&2
    exit 1
fi
if rg -n -i 'noctalia' "$ROOT/install.sh" |
    grep -Ev 'for package_name in cachyos-niri-noctalia noctalia-shell'; then
    printf '%s\n' 'unexpected Noctalia reference outside the exact Arch migration loop' >&2
    exit 1
fi

printf '%s\n' '[check] all required checks passed'
