# Developer experience

## Current status

Weyriva provides a deterministic Niri-oriented desktop layout, a local control
CLI, and repository-owned Quickshell surfaces. The current source exposes the
core launcher, control center, calendar, notifications, wallpaper, settings,
terminal, lock, and compositor actions listed below.

Installed input behavior, login, lock, and lifecycle acceptance on XRY remain
pending. A command or QML handler existing in source does not prove the
installed shell is interactive.

## Fixed default flow

| Intent | Binding | Result |
|---|---|---|
| Launch/search | `Mod+Space` | centered Launcher |
| Notifications | `Mod+N` | top-right Notifications |
| Control center | `Mod+C` | top-right Control Center |
| Wallpaper | `Mod+W` | centered Wallpaper |
| Settings | `Mod+Shift+T` | centered Settings |
| Lock | `Mod+Shift+X` | request native secure lock |
| Terminal | `Mod+Return` | launch Foot |
| Close window | `Mod+Q` | Niri close action |
| Focus | `Mod+H/J/K/L` | Niri directional focus |
| Move column | `Mod+Shift+H/L` | Niri column movement |
| Workspace | `Mod+1/2/3` | Niri workspace focus |
| Move to workspace | `Mod+Shift+1/2/3` | Niri workspace movement |
| Screenshot | `Print` | Niri screenshot |

These bindings are the shipped zero-configuration default. Structural
personalization belongs in a maintained fork.

## Surface workflows

The launcher filters actual desktop entries and executes a selected entry. It
does not interpolate search text into a shell command.

The control center presents two columns of real controls. The calendar exposes
month navigation and a date grid. Notifications are dismissible and include an
empty state. Wallpaper selection is visual and updates wallpaper and related
appearance state. Settings shows explicit values and visibly disables future
capabilities.

Centered launcher/wallpaper/settings surfaces support focused work. Top-right
control-center/calendar/notification surfaces remain spatially tied to their
bar sources.

## Theme and personalization

Light and dark presentation are supported in source. Dynamic wallpaper color
extraction is not implemented. Wallpaper choices use the fixed Weyriva palette
and must not be described as generated themes.

The product avoids installation questions and per-user structural options.
Small runtime choices such as the current appearance or wallpaper may be state;
changes to layout, policy, package set, or workflow belong in a fork.

## Status and diagnostics

Useful commands include:

```bash
weyriva status
weyriva diagnose
weyriva diagnose --json
weyriva ipc call weyriva.info
weyriva ipc call weyriva.niri.outputs
```

The native QML IPC surface includes a typed `status(): string` method. This can
show that the live QML IPC target answered. It does not prove pointer input,
all route actions, PAM authentication, output coverage, or secure
`WlSessionLock` recovery.

## Recovery

The normal recovery sequence is:

1. dismiss the active surface with Escape when possible;
2. use the bounded user-shell recovery path;
3. switch to a TTY if the graphical session is unusable;
4. inspect user and greetd journals;
5. repair the installed path only with evidence of the failure.

Restart while locked is security-sensitive. Never convert a successful status
reply into evidence that the previous secure lock survived or was reacquired.
See [Session lifecycle](SESSION_LIFECYCLE.md).

## Acceptance boundary

Source tests can prove bindings, action wiring, IPC types, and the absence of
dead enabled controls. Runtime testing must prove rendering and input. System
testing must prove greetd, PAM, Niri, systemd, and lock behavior. XRY claims
require the exact installed revision and recorded interaction evidence.
