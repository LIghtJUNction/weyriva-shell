# Plugin conformance fixtures

All fixtures are authored by Weyriva. They exercise public observable behavior
without copying upstream shell implementation or plugin source.

Only `v5-launcher-api3` is currently passed locally. Every other fixture below
is a planned acceptance case and remains not run unless its section later
records a different status.

Each fixture records:

- profile and requested API;
- fixture revision;
- manifest and entry under test;
- expected events and visible result;
- observed result and logs;
- host version;
- local/runtime/system/XRY evidence level.

## v5 fixtures

### `v5-launcher-api3`

Status: passed locally.

The source lives under `tests/fixtures/plugins/v5-launcher-api3`. It proves one
API 3 launcher provider can load in the independent Rust host, query and
activate results, handle IPC and shutdown, use config/JSON/relative files and
process-lifetime state, and emit bounded clipboard/notification/query actions.
Negative entries exercise traversal and symlink reads, invalid/oversized
models and actions, response/result limits, memory exhaustion, infinite loops,
unsupported namespaces, malformed methods, and protocol recovery.

Control-plane fixtures under `tests/fixtures/plugin-sources` prove
pinned/ordered source resolution, immutable atomic install, explicit lifecycle,
failure state, exit evidence, safe action argv, and daemon/CLI round trips. A
separate live-source probe installed official `noctalia/kaomoji` 1.0.1 from commit
`4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6` and exercised its `kao` launcher
prefix.

This status applies only to API 3 with one launcher-provider entry.

### `v5-all-entries-api3`

One plugin declares all six entry kinds:

- widget renders text/glyph and handles click, right-click, scroll, update, and
  IPC;
- shortcut toggles active state;
- launcher echoes queries, returns scored results, and activates one;
- desktop widget renders a keyed declarative tree;
- panel opens, renders controls, closes, and handles IPC;
- service publishes a counter through shared state.

Pass requires visible input and lifecycle evidence, not only callbacks in logs.

### `v5-config-state-persistence`

Exercises:

- root, widget, panel, and desktop-widget settings;
- every documented setting type;
- defaults, min/max, select options, conditional visibility, and advanced;
- widget-over-root precedence;
- undeclared key warning and `nil`;
- `state.set/get/watch` copied values across VMs;
- process-lifetime state reset;
- persistent data surviving source update and shell restart.

### `v5-api-boundaries`

Separate packages request APIs 2, 3, 15, 16, 17, 18, and one number above the
host range.

Pass requires deterministic acceptance or rejection and a correct diagnostic.
No package may partially load.

### `v5-lifecycle-errors`

Records top-level load, update, enable, config change, reload, disable,
uninstall, shutdown, and exit reasons. Injects:

- syntax error;
- callback exception;
- slow callback;
- failed async operation;
- service crash;
- malformed declarative UI.

Pass requires core-shell responsiveness and bounded teardown.

### `v5-source-precedence`

Provides the same canonical ID through local drop-in, path, multiple user git
sources, official, and community sources. It verifies deterministic winner,
compatible historic revision selection, update failure recovery, and no mixed
files from two revisions.

### `v5-ipc-targeting`

Places one widget entry on multiple outputs plus singleton service and panel
entries. It verifies focused, connector, placement, and all targeting; payload
encoding; unknown event; callback failure; and final response behavior.

## v4 fixtures

### `v4-core-components`

Self-authored `Main.qml`, `BarWidget.qml`, `ControlCenterWidget.qml`, and
`Panel.qml` verify imports, injected properties, `mainInstance`, panel
open/close, input, and teardown.

### `v4-desktop-settings`

Verifies `DraggableDesktopWidget`, `widgetData`, scale/drag state,
`DesktopWidgetSettings.qml`, `widgetSettings.save()`, and fallback to the
plugin-wide settings component.

### `v4-launcher-provider`

Verifies command prefix, `init`, open notification, query, categories, results,
selection, and launcher open/close/toggle.

### `v4-api-ipc-i18n`

Verifies `pluginApi` properties/functions, mutable settings plus explicit save,
translation current-language/English/key fallback, plural/interpolation
behavior, and `IpcHandler` string arguments.

### `v4-failure-isolation`

Injects missing imports, invalid QML, binding loops, missing injected
properties, unavailable services, settings failure, and IPC exceptions.
The isolated host may fail the plugin, but not the core shell.

## Cross-profile visual acceptance

For UI entries:

- pointer and keyboard actions work;
- focus is visible and restored;
- disabled/loading/error states are distinct;
- text scale and contrast remain usable;
- reduced motion is honored where the ABI allows;
- closing or disabling removes the surface and input region;
- a plugin failure cannot leave an invisible input blocker.

## Pass policy

A fixture status is one of:

- not run;
- failed;
- partially passed;
- passed locally;
- passed in native runtime;
- verified on XRY.

“Partially passed” never widens the advertised compatibility range.
