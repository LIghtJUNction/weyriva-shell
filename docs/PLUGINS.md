# Plugins

Weyriva owns its plugin hosts. Noctalia is used only as a pinned public ABI and
behavior reference; Weyriva does not delegate compatible plugin execution to a
Noctalia shell.

## Compatibility lanes

| Lane | Format | Host | Status |
|---|---|---|---|
| v5-compatible | `plugin.toml` + Luau | Weyriva trusted Luau host | In progress |
| v4-compatible | `manifest.json` + QML | isolated Weyriva Quickshell host | Planned |
| Weyriva legacy | JSON manifest + executable | bounded local JSON daemon | Implemented locally |

Compatibility means actual lifecycle, UI, settings, state, IPC, and error
behavior. Manifest parsing, catalog listing, downloading, or copying files does
not count.

## v5-compatible Luau

The public v5 profile has six entry kinds:

- bar widget;
- control-center shortcut;
- launcher provider;
- desktop widget;
- panel;
- headless service.

Each entry runs in an isolated Luau VM and communicates through documented host
namespaces and copied plain state. Plugins are trusted and not sandboxed.

Weyriva implements API levels incrementally and advertises only levels that
pass conformance. The first target is the published `3..16` range associated
with Noctalia beta.3–beta.6; levels 17 and 18 remain separate probe gates.

See [Noctalia v5 Luau](plugins/noctalia-v5-luau.md).

## v4-compatible QML

The v4 profile loads QML entry points that depend on injected `pluginApi`,
Quickshell modules, `qs.*` imports, settings, translations, services, and IPC.
It therefore requires a real isolated compatibility host.

Weyriva will not load arbitrary v4 components into the core shell process.
Parsing `manifest.json` or showing a card is not compatibility.

See [Noctalia v4 QML](plugins/noctalia-v4-qml.md).

## Legacy executable plugins

The current repository can discover validated JSON manifests and expose bounded
executable methods through the local `weyriva.*` daemon.

This lane:

- has request and output limits;
- has an execution timeout;
- cannot add native bars, panels, settings, greeter, lock, or desktop widgets;
- is maintained only for migration and diagnostics;
- is not evidence for v5 or v4 compatibility.

## Sources and installation

The product goal is a zero-choice source policy with pinned official and
community baselines plus explicitly added user sources. Source precedence,
revision selection, update safety, and failure recovery must be implemented by
Weyriva before source commands are documented as stable.

No dependency manager is inferred from a plugin's metadata. The UI reports
external requirements and failures; it does not silently grant privilege or
install arbitrary packages.

## Trust

Luau, QML, and executable plugins are trusted user code. Installation must
show source, author, version, requested API, declared external dependencies,
and compatibility result. Credentials and plugin data must not appear in logs.

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
