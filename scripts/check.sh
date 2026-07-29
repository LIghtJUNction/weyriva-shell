#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CHECK_TMP=$(mktemp -d)
trap 'rm -rf "$CHECK_TMP"' EXIT

printf '%s\n' '[check] Python compile'
mapfile -d '' PYTHON_TEST_FILES < <(
    find "$ROOT/tests" "$ROOT/crates" \
        -path '*/tests/*' -type f -name '*.py' -print0 |
        sort -z
)
PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" \
    python3 -m py_compile "${PYTHON_TEST_FILES[@]}"

printf '%s\n' '[check] Python unit tests'
PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" \
    python3 -m unittest discover -s "$ROOT/tests" -v

printf '%s\n' '[check] Bash syntax'
bash -n "$ROOT/install.sh"
while IFS= read -r -d '' script; do bash -n "$script"; done < <(find "$ROOT/scripts" -name '*.sh' -type f -print0)

printf '%s\n' '[check] JSON, TOML, INI, and desktop syntax'
PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" \
    python3 - "$ROOT" <<'PY'
import configparser
import json
import sys
import tomllib
from pathlib import Path

root = Path(sys.argv[1])


def source_files(suffix: str):
    return (
        path
        for path in root.rglob(f"*{suffix}")
        if not {
            ".git",
            "target",
            "__pycache__",
            ".pytest_cache",
            ".ruff_cache",
        }.intersection(path.parts)
    )


for path in (*source_files(".json"), *source_files(".jsonc")):
    json.loads(path.read_text(encoding="utf-8"))
for path in source_files(".toml"):
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
PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPYCACHEPREFIX="$CHECK_TMP/pycache" \
    python3 - "$ROOT" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
excluded_names = {
    ".git",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".ruff_cache",
}
for path in root.rglob("*"):
    if not path.is_file() or excluded_names.intersection(path.parts):
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

printf '%s\n' '[check] Rust formatting'
cargo fmt --manifest-path "$ROOT/Cargo.toml" --all -- --check

printf '%s\n' '[check] Rust compile and lint'
cargo clippy \
    --manifest-path "$ROOT/Cargo.toml" \
    --locked \
    --workspace \
    --all-targets \
    --all-features \
    -- \
    -D warnings

printf '%s\n' '[check] Rust tests'
cargo test \
    --manifest-path "$ROOT/Cargo.toml" \
    --locked \
    --workspace \
    --all-targets \
    --all-features

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

printf '%s\n' '[check] all-Rust product policy'
PYTHON_SURFACE_MATCHES="$CHECK_TMP/python-surface.txt"
while IFS= read -r -d '' path; do
    relative=${path#"$ROOT"/}
    case $relative in
        tests/* | crates/*/tests/* | scripts/*.py)
            ;;
        *)
            printf '%s\n' "$relative" >>"$PYTHON_SURFACE_MATCHES"
            ;;
    esac
done < <(
    find "$ROOT" \
        \( -path "$ROOT/.git" -o -path "$ROOT/target" \) -prune -o \
        -type f -name '*.py' -print0
)
if [[ -s $PYTHON_SURFACE_MATCHES ]]; then
    printf '%s\n' 'Python file found outside test/static tooling paths:' >&2
    cat "$PYTHON_SURFACE_MATCHES" >&2
    exit 1
fi

PYTHON_RUNTIME_MATCHES="$CHECK_TMP/python-runtime.txt"
if rg -n -i \
    '(^#!.*python|^[[:space:]]*(from|import)[[:space:]]+[A-Za-z_])' \
    "$ROOT/bin" \
    "$ROOT/examples" \
    "$ROOT/lib" \
    "$ROOT/packaging" \
    "$ROOT/install.sh" \
    "$ROOT/scripts/install-system.sh" \
    "$ROOT/scripts/install-greetd.sh" \
    "$ROOT/scripts/install.sh" \
    >"$PYTHON_RUNTIME_MATCHES"; then
    printf '%s\n' 'Python runtime/import surface found in product paths:' >&2
    cat "$PYTHON_RUNTIME_MATCHES" >&2
    exit 1
fi

PYTHON_PACKAGE_MATCHES="$CHECK_TMP/python-package.txt"
if rg -n '\bpython(3)?\b' \
    "$ROOT/install.sh" \
    "$ROOT/packaging" \
    "$ROOT/scripts/install-system.sh" \
    "$ROOT/scripts/install-greetd.sh" \
    "$ROOT/scripts/install.sh" \
    >"$PYTHON_PACKAGE_MATCHES"; then
    printf '%s\n' 'installed/package Python dependency found:' >&2
    cat "$PYTHON_PACKAGE_MATCHES" >&2
    exit 1
fi

LEGACY_PLUGIN_MATCHES="$CHECK_TMP/legacy-plugin.txt"
if rg -n -i '(plugins-v5|weyriva_plugins_v5)' \
    "$ROOT/bin" \
    "$ROOT/config" \
    "$ROOT/examples" \
    "$ROOT/greeter" \
    "$ROOT/lib" \
    "$ROOT/packaging" \
    "$ROOT/shell" \
    "$ROOT/systemd" \
    "$ROOT/user-share" \
    "$ROOT/install.sh" \
    "$ROOT/scripts/install-greetd.sh" \
    "$ROOT/scripts/install.sh" \
    >"$LEGACY_PLUGIN_MATCHES"; then
    printf '%s\n' 'legacy Python plugin product name found:' >&2
    cat "$LEGACY_PLUGIN_MATCHES" >&2
    exit 1
fi
legacy_cleanup_count=$(awk '{
    count += gsub(/weyriva_plugins_v5[.]py/, "")
} END {
    print count + 0
}' "$ROOT/scripts/install-system.sh")
if [[ $legacy_cleanup_count -ne 1 ]] ||
    ! grep -Fqx \
        'legacy_runtime=/usr/lib/weyriva/weyriva_plugins_v5.py' \
        "$ROOT/scripts/install-system.sh" ||
    ! grep -Fq \
        "cp -a --no-dereference -- \"\$legacy_runtime\" \"\$legacy_backup\"" \
        "$ROOT/scripts/install-system.sh" ||
    ! grep -Fq "rm -f -- \"\$legacy_runtime\"" \
        "$ROOT/scripts/install-system.sh"; then
    printf '%s\n' \
        'legacy Python cleanup must be the single no-follow backup/removal exception' >&2
    exit 1
fi

RESIDUE_MATCHES="$CHECK_TMP/python-residue.txt"
find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" \) -prune -o \
    \( -type d -name __pycache__ -o -type f \( -name '*.pyc' -o -name '*.pyo' \) \) \
    -print >"$RESIDUE_MATCHES"
if [[ -s $RESIDUE_MATCHES ]]; then
    printf '%s\n' 'Python cache residue found in repository:' >&2
    cat "$RESIDUE_MATCHES" >&2
    exit 1
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
