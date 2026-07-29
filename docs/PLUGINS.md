# Plugins

Weyriva owns its plugin hosts. Noctalia is used only as a pinned public ABI and
behavior reference; Weyriva does not delegate compatible plugin execution to a
Noctalia shell.

The product name is **Weyriva Plugins**. It has no `v5` product version;
`noctalia-v5-luau/1` is only an exact upstream compatibility profile.

## Compatibility profiles

| Lane | Format | Host | Status |
|---|---|---|---|
| `noctalia-v5-luau/1` | `plugin.toml` + Luau | Rust `weyriva-luau-host` | API 3 single-launcher-provider slice locally tested and package-wired; target verification pending |
| `noctalia-v4-qml/1` | `manifest.json` + QML | isolated Weyriva Quickshell host | Planned |

Compatibility means actual lifecycle, UI, settings, state, IPC, and error
behavior. Manifest parsing, catalog listing, downloading, or copying files does
not count.

Python is repository test tooling only, not a production dependency or plugin
language.

## Current Luau slice

The only accepted vertical slice is:

- `plugin_api = 3`;
- exactly one `[[launcher_provider]]` entry;
- pinned official and community sources plus ordered local path sources;
- immutable digest-addressed install, explicit enable/disable/reload/uninstall;
- one persistent, bounded `weyriva-luau-host` process per entry;
- query, activation, `onIpc`, `onExit`, config defaults, JSON, relative reads,
  process-lifetime state, clipboard, notifications, and launcher query actions;
- dynamic QML prefix and category discovery, category filtering, debounce,
  loading, empty, error, keyboard, and activation states.

This slice passed the self-authored `v5-launcher-api3` fixture and a local
installation/query/activation probe of pinned `noctalia/kaomoji`. It does not
widen the advertised range beyond that exact surface. The Rust binaries and
local package consumers are wired; clean-package, installed-service, and
target-machine interaction evidence remains incomplete.

The locally implemented daemon-backed command interface is:

```bash
weyriva plugin source list
weyriva plugin install noctalia/kaomoji
weyriva plugin enable noctalia/kaomoji
weyriva plugin status noctalia/kaomoji
weyriva plugin reload noctalia/kaomoji
weyriva plugin disable noctalia/kaomoji
weyriva plugin uninstall noctalia/kaomoji
```

The Rust plugin core in `crates/weyriva/` owns ordered sources, immutable
install and state, lifecycle, host sessions, bounded actions, and Unix IPC.
QML is UI only and receives validated provider and result data. The local
installer and AUR recipe place this binary at `/usr/bin/weyriva`; neither a
published package nor a target deployment is implied.

## Noctalia Luau compatibility

The public v5 profile has six entry kinds:

- bar widget;
- control-center shortcut;
- launcher provider;
- desktop widget;
- panel;
- headless service.

The target profile runs each entry in an isolated Luau VM and communicates
through documented host namespaces and copied plain state. Plugins are trusted
and not sandboxed. Current implementation evidence covers only the single API
3 launcher-provider entry described above.

Weyriva implements API levels incrementally and advertises only levels that
pass conformance. API 3 launcher-provider support is the first passing slice.
The remaining API 3 entry kinds and APIs 4–19 remain separate gates.

See [Noctalia v5 Luau](plugins/noctalia-v5-luau.md).

## v4-compatible QML

The v4 profile loads QML entry points that depend on injected `pluginApi`,
Quickshell modules, `qs.*` imports, settings, translations, services, and IPC.
It therefore requires a real isolated compatibility host.

Weyriva will not load arbitrary v4 components into the core shell process.
Parsing `manifest.json` or showing a card is not compatibility.

See [Noctalia v4 QML](plugins/noctalia-v4-qml.md).

## Sources and installation

The Rust control plane uses pinned official and community baselines. Later
local path sources override earlier and built-in sources. Git user sources,
implicit local drop-ins, historical `[[plugin.release]]` selection, catalog UX,
and interrupted-update recovery remain incomplete.

No dependency manager is inferred from a plugin's metadata. The UI reports
external requirements and failures; it does not silently grant privilege or
install arbitrary packages.

## Trust

Luau and QML plugins are trusted user code. Installation must show source,
author, version, requested API, declared external dependencies, and
compatibility result. Credentials and plugin data must not appear in logs.

## Required evidence

Acceptance requires self-authored fixtures for:

- every entry kind;
- enable, disable, update, reload, uninstall, and shutdown;
- settings scopes and changes;
- cross-entry and persistent state;
- IPC targeting and errors;
- source precedence and compatible revision selection;
- crash, timeout, malformed UI, and missing dependency isolation.

See:

- [Compatibility contract](plugins/compatibility-contract.md)
- [Conformance fixtures](plugins/conformance-fixtures.md)
- [Known gaps](plugins/known-gaps.md)
- [Upstream baselines](plugins/upstream-baselines.md)
