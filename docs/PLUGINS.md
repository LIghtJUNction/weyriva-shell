# Plugins

Weyriva has three plugin lanes with different manifests, runtimes, and
acceptance requirements. They must never be presented as one compatibility
layer.

## Native Noctalia v5

Current plugins use `plugin.toml`, trusted Luau entries, and the same Noctalia
engine that renders the Weyriva shell. Weyriva does not translate manifests,
generate QML, or widen the engine's API range.

The installed engine is authoritative:

- Noctalia v5.0.0-beta.6 accepts plugin API 3–16;
- the reviewed upstream main baseline accepts 3–18;
- a future engine may differ, so runtime/package acceptance must query that
  engine rather than trust this document.

The six current entry kinds are:

| Manifest entry | Surface |
| --- | --- |
| `[[widget]]` | Bar widget |
| `[[shortcut]]` | Control-center shortcut |
| `[[launcher_provider]]` | Launcher provider |
| `[[desktop_widget]]` | Desktop widget |
| `[[panel]]` | Pop-up panel |
| `[[service]]` | Headless background service |

Authoritative references:

- [plugin usage](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/plugins/index.mdx);
- [manifest](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/plugins/development/manifest.mdx);
- [entries](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/plugins/development/entries.mdx);
- [plugin API ledger](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/plugins/development/plugin-api.mdx).

Native plugins are trusted, unsandboxed user code. They can access files and
the network and spawn processes with the user's authority. Review source and
dependencies before enabling one.

### Sources

The fixed profile seeds:

- [official plugins](https://github.com/noctalia-dev/official-plugins);
- [community plugins](https://github.com/noctalia-dev/community-plugins).

Private Git sources must already be accessible through a credential helper, SSH
agent, or SSH configuration. Native source management is non-interactive.

### Commands

```bash
weyriva plugin list
weyriva plugin install noctalia/screen_recorder
weyriva plugin enable noctalia/screen_recorder
weyriva plugin disable noctalia/screen_recorder
weyriva plugin update official
weyriva plugin source list
weyriva plugin source add my-repo git https://github.com/me/my-plugins
weyriva plugin source add my-dev path /absolute/path/to/my-plugins
weyriva plugin source remove my-repo
weyriva plugin lint
weyriva plugin lint /absolute/path/to/plugin
```

`plugin install ID` is Weyriva's zero-configuration alias for native enable:
the owning source is fetched/materialized when required and the plugin becomes
active.

Noctalia v5 exposes **disable, not per-plugin remove**. Disabling a plugin does
not promise deletion of materialized files. `plugin update` names a configured
source, not an individual plugin. Weyriva must not invent a remove command that
the installed engine cannot honor.

Native caches, exported plugins, settings, and state remain inside the isolated
Weyriva profile.

## Legacy Weyriva executables

The original Weyriva version-1 JSON manifest lane remains only for existing
local automation. A manifest maps reserved method names to executable argument
arrays. The local daemon writes `params` as JSON to stdin and expects one JSON
value on stdout.

```bash
weyriva plugin legacy-list
weyriva plugin legacy-reload
weyriva plugin legacy-validate examples/plugins/hello.json
```

This lane:

- extends only the local `weyriva.*` JSON protocol;
- cannot add native bars, panels, desktop widgets, settings, or Greeter UI;
- applies execution time and output bounds and never evaluates a shell;
- is still trusted user code and is not sandboxed;
- is not evidence of Noctalia plugin compatibility.

See [IPC](IPC.md) for the protocol boundary.

## Legacy Noctalia v4 QML

Noctalia v4 plugins use `manifest.json` and QML files such as `Main.qml`,
`BarWidget.qml`, `DesktopWidget.qml`, `Panel.qml`, and `Settings.qml`. The
upstream archive is
[legacy-v4-plugins](https://github.com/noctalia-dev/legacy-v4-plugins).

Weyriva does **not** currently support this ABI. Compatibility requires an
isolated Quickshell companion host with:

- manifest and dependency validation;
- QML engine/runtime isolation from native v5;
- real bar/panel/desktop/settings rendering;
- plugin IPC and lifecycle integration;
- controlled reload and shutdown;
- pointer, keyboard, visual, and XRY acceptance.

Parsing `manifest.json`, listing a plugin, or copying its files is not
compatibility. The companion host must not steal surfaces from the native
Noctalia engine.

## Acceptance

Catalog presence is not a pass. The native matrix must pin source commits and,
for every ID, lint, enable, await a final state, exercise each entry, test
settings/IPC/dependencies, update the source, disable, and re-enable. It must
cover all six entry kinds plus lower/upper/out-of-range API cases.

Detailed procedure and evidence format are in [Testing](TESTING.md).
