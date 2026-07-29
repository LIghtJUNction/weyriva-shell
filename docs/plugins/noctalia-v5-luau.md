# Noctalia v5-compatible Luau profile

Profile: `noctalia-v5-luau/1`

Runtime owner: Weyriva

Status: API 3 single-launcher-provider slice passed locally; overall in progress

This profile reproduces the documented public v5 plugin ABI in a Weyriva-owned
Luau host. It does not require or launch Noctalia.
It is a compatibility profile inside **Weyriva Plugins**, not a Weyriva product
version. The passing slice is local implementation/test evidence, not an
installed-runtime or XRY claim.

## Public baseline

The normative research baseline is the Noctalia documentation commit
`a0fcbcafc709836f46e1c23b18ade6947d442e26`. See
[Upstream baselines](upstream-baselines.md).

The public ledger at that commit defines cumulative API levels `3..19`.
Noctalia beta.6 accepts `3..16`; current upstream main accepts `3..19`.
Weyriva advertises only the exact entry/API combinations its fixtures pass.
At present that is API 3 with exactly one launcher-provider entry.

## Package layout

```text
plugin/
├── plugin.toml
├── one-or-more-entry.luau
├── translations/
│   └── en.json
└── optional data files
```

Each entry executes in an isolated Luau VM. Top-level script code executes once
when that entry loads.

## Manifest identity

Public root fields:

| Field | Contract |
|---|---|
| `id` | canonical `<author>/<plugin>` identity |
| `name` | required; missing value rejects the manifest |
| `version` | plugin version |
| `plugin_api` | required positive integer and compatibility gate |
| `author` | author identity |
| `license` | optional; documented default `MIT` |
| `deprecated` | optional soft listing state, not a compatibility gate |
| `icon` | optional glyph |
| `description` | description |
| `tags` | optional catalog tags |
| `dependencies` | optional external-tool metadata |

`dependencies` is informational in the public v5 contract: missing tools do
not by themselves block enable. Weyriva may report them prominently but must
not silently reinterpret them as package-manager authority.

## Entry kinds

Every entry has an `id` unique within the plugin and an `.luau` `entry`.
The full address is `<author>/<plugin>:<entry-id>`.

| Table | Kind | Primary behavior |
|---|---|---|
| `[[widget]]` | bar widget | render, ticks, pointer, scroll, IPC |
| `[[shortcut]]` | control-center tile | label/icon/state, click, IPC |
| `[[launcher_provider]]` | launcher provider | query/results/activation |
| `[[desktop_widget]]` | desktop widget | declarative UI, ticks, IPC |
| `[[panel]]` | pop-up panel | declarative UI, open/close, keys, IPC |
| `[[service]]` | headless service | background state and IPC |

Representative entry-specific manifest fields include:

- widget `[widget.actions]` gesture defaults;
- launcher `prefix`, `glyph`, `include_in_global_search`, `debounce_ms`;
- panel `width`, `height`, `placement`, `position`, `open_near_click`,
  `dismiss_on_outside_click`, `keyboard_focus`, `persistent`, `capture_keys`.

Entry-specific fields are API-gated where the public ledger says so.

## Lifecycle

Documented callback surface:

| Callback | Entries |
|---|---|
| `update()` | widget, desktop widget, panel, service |
| `onClick()`, `onRightClick()` | widget, shortcut |
| `onMiddleClick()` | widget |
| `onScroll(axis, steps, startsGesture)` | widget |
| `onQuery(text)`, `onActivate(id)` | launcher provider |
| `onOpen(context)`, `onClose()` | panel |
| `onKey(chord, pressed)` | panel |
| `onFrameTick(deltaMs)` | desktop widget and panel when requested |
| `onIpc(event, payload)` | all six kinds |
| `onConfigChanged()` | service |
| `onEnable()` | service at API 17 |
| `onExit(signal, reason)` | all six kinds |

Publicly documented exit signals are `2` for SIGINT, `15` for SIGTERM, and `0`
for other teardown. API 17 adds reasons `disable`, `uninstall`, `reload`, and
`shutdown`.

`onEnable()` describes explicit successful enable, not ordinary startup,
source update, script reload, or settings-driven service restart.

If a plugin is already disabled, no entry VM exists and uninstall cannot invoke
its `onExit`.

## Settings

Settings are declared and typed:

- root `[[setting]]`: shared plugin-level value;
- `[[widget.setting]]`: per widget entry/placement;
- `[[panel.setting]]`: panel setting shown with plugin settings;
- public official manifests also use `[[desktop_widget.setting]]`.

Documented types:

```text
string string_list string_map bool int double select file folder glyph color
```

Fields include `key`, `type`, `label_key`, `description_key`, `default`,
`min`, `max`, `options`, `extensions`, `visible_when`, and `advanced`.
`label_key` is required; literal `label` and `description` are rejected.

`noctalia.getConfig(key)` in the public ABI becomes the same named function in
the compatibility environment. Undeclared keys warn and return `nil`. A widget
entry value overrides a root value with the same key.

For setting changes:

- widget, desktop widget, and panel entries are rebuilt;
- a service with `onConfigChanged()` keeps its VM and receives updated values;
- a service without it restarts its VM.

## Runtime namespaces

The profile exposes documented namespaces only:

- `noctalia.*` for runtime, system, subprocess, filesystem, HTTP, translation,
  JSON, string, state, wallpaper, panel, clipboard, and notification helpers;
- `barWidget.*`;
- `shortcut.*`;
- `launcher.*`;
- `desktopWidget.*`;
- `panel.*`;
- `ui.*` declarative controls.

The namespace name is compatibility syntax, not evidence that Noctalia owns
the runtime. Weyriva supplies the implementation.

Every method is implemented and fixture-gated individually. An absent method
must produce a deterministic compatibility error, not disappear silently.

## State and persistence

Entry VMs do not share Luau memory. Public shared state is:

```text
noctalia.state.set(key, value)
noctalia.state.get(key)
noctalia.state.watch(key, callback)
```

Values are copied plain data: strings, numbers, booleans, and tables of those.
State is per-plugin and process-lifetime only. It survives a documented
settings-driven service restart but not a plugin stop.

Persistent data belongs in the compatibility implementation of
`noctalia.pluginDataDir()`. A plugin must not persist into its runtime source
directory because source updates may replace it.

## IPC

`onIpc(event, payload)` is addressable for all entry kinds. Bar entries may
have multiple output/placement instances; service and other non-bar entries
have no output and match the documented all-target behavior.

Panel open/close/toggle is a separate surface action. Targeting, payload
encoding, error semantics, and broadcast ordering remain fixture-gated.

## Sources

The public source precedence is:

1. local data-directory drop-in;
2. later-added source over earlier source;
3. user source over built-in official/community source.

Only one copy of a canonical plugin ID loads.

A git catalog may provide older `[[plugin.release]]` rows with a full
40-character revision. The host chooses the newest revision allowed by its API
range. Path sources have no revision export.

## API capability ledger

| API | Publicly introduced capability |
|---:|---|
| 3 | mandatory `plugin_api` declaration |
| 4 | HTTP streaming |
| 5 | declarative panel drag and drop |
| 6 | `string_map` settings |
| 7 | insecure-TLS request option |
| 8 | panel outside-click policy |
| 9 | closure callbacks in UI trees |
| 10 | panel keyboard-focus policy |
| 11 | persistent panels |
| 12 | system statistics and millisecond clock |
| 13 | captured panel keys and `onKey` |
| 14 | widget gesture-action defaults |
| 15 | open the plugin's settings |
| 16 | extended system/disk/network statistics |
| 17 | service lifecycle and exit reasons; unreleased at baseline |
| 18 | panel frame ticks; unreleased at baseline |
| 19 | time-zone list and lookup |

## Acceptance

Weyriva must pass [Conformance fixtures](conformance-fixtures.md) without an
installed Noctalia runtime before advertising this profile.
