# Noctalia v4-compatible QML profile

Profile: `noctalia-v4-qml/1`

Runtime owner: isolated Weyriva Quickshell host

Status: planned

The v4 format is a QML component ABI, not merely a JSON manifest. Compatible
plugins depend on injected context, Quickshell modules, `qs.*` imports,
services, settings, translations, and IPC. Weyriva must execute them outside
the core shell process.

## Package layout

Public packages use `manifest.json` plus any subset of:

```text
Main.qml
BarWidget.qml
DesktopWidget.qml
DesktopWidgetSettings.qml
ControlCenterWidget.qml
LauncherProvider.qml
Panel.qml
Settings.qml
i18n/<language>.json
```

At least one entry point is required.

## Manifest

Required public fields:

- `id`;
- `name`;
- `version`;
- `author`;
- `description`;
- `entryPoints`.

The documented version format is `x.y.z`. `id` is expected to match the plugin
directory and use lower-case kebab case.

Optional fields include:

- `minNoctaliaVersion`;
- `license`;
- `repository`;
- `dependencies.plugins`;
- `metadata.commandPrefix`;
- `metadata.defaultSettings`.

The public v4 docs explicitly say plugin dependency resolution was not
implemented. Weyriva preserves that observable distinction instead of claiming
automatic dependency installation.

## Entry points

| Manifest key | Component role |
|---|---|
| `main` | background logic and IPC handlers |
| `barWidget` | bar component |
| `desktopWidget` | draggable/scalable desktop component |
| `desktopWidgetSettings` | per-instance desktop-widget settings |
| `controlCenterWidget` | control-center action |
| `launcherProvider` | launcher query/browse provider |
| `panel` | overlay panel |
| `settings` | plugin settings UI |

If `desktopWidgetSettings` is absent, public docs describe a fallback to
`settings` when available.

## QML imports

Representative public imports form part of the compatibility surface:

```qml
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Widgets
import qs.Services.UI
import qs.Services.System
import qs.Modules.DesktopWidgets
```

The exact minimum module graph and import versions require behavior probes.
Weyriva must not expose its private core components accidentally; the
compatibility host supplies an explicit module façade.

## Injected Plugin API

Every entry component declares:

```qml
property var pluginApi: null
```

Documented properties include:

- `pluginId`;
- `pluginDir`;
- mutable `pluginSettings`;
- `manifest`;
- `currentLanguage`;
- `pluginTranslations`;
- `mainInstance`;
- component references for available entries;
- panel-only `panelOpenScreen`.

Documented functions include:

- `saveSettings()`;
- `openPanel`, `closePanel`, `togglePanel`;
- `openLauncher`, `closeLauncher`, `toggleLauncher`;
- `withCurrentScreen`;
- `tr`, `trp`, `hasTranslation`.

Settings persistence requires an explicit `saveSettings()` call.

## Entry context

### Bar widget

Documented injected properties:

```qml
property ShellScreen screen
property string widgetId
property string section
property int sectionWidgetIndex
property int sectionWidgetsCount
```

### Control-center widget

Receives `screen` and `pluginApi`.

### Desktop widget

Uses `DraggableDesktopWidget` from `qs.Modules.DesktopWidgets` and relies on
`screen`, `widgetData`, `widgetIndex`, `isDragging`, `isScaling`,
`showBackground`, and `widgetScale`.

### Desktop-widget settings

Receives `pluginApi` and `widgetSettings`. It implements `saveSettings()` and
persists per-instance changes through `widgetSettings.save()`.

### Panel

Documented panel properties include:

```qml
readonly property var geometryPlaceholder
readonly property bool allowAttach
property real contentPreferredWidth
property real contentPreferredHeight
```

### Launcher provider

Receives `pluginApi` and `launcher`; public behavior includes `init`,
`onOpened`, `handleCommand`, `commands`, `getResults`, category selection, and
result activation.

### Settings

Receives `pluginApi` and must implement `saveSettings()` for the host dialog.

## IPC

v4 plugins use `Quickshell.Io.IpcHandler` in `Main.qml`:

```qml
IpcHandler {
  target: "plugin:plugin-id"
  function command(argument: string) { }
}
```

The target prefix is `plugin:` followed by the manifest ID. CLI arguments enter
as strings and the plugin parses richer types. The public command shape is:

```text
qs -c noctalia-shell ipc call plugin:<id> <method> [arguments]
```

Weyriva provides compatible routing inside its isolated host; it does not
launch `noctalia-shell`.

## Isolation

- v4 plugins never load into the core Weyriva shell process;
- one broken import or binding must not terminate the desktop;
- the host limits available private modules to the compatibility façade;
- settings and plugin data are scoped by canonical ID;
- logs identify plugin and entry without exposing secrets.

## Acceptance

Compatibility requires real Quickshell loading, rendering, pointer and keyboard
input, settings persistence, translation fallback, IPC, lifecycle, and failure
isolation. Manifest discovery alone is not a pass.
