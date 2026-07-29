# Architecture

Weyriva is a branded, opinionated Niri distribution of the upstream Noctalia
v5 engine. It does not reproduce Noctalia surfaces in Python, Waybar, or a
second plugin host. Noctalia Greeter owns the visible login surface; Noctalia
v5 owns the desktop and in-session lock surface.

This document defines the intended end state. Installed/runtime acceptance is
tracked separately in [Noctalia parity](NOCTALIA_PARITY.md).

## System chain

```text
display-manager.service
└─ greetd.service
   └─ noctalia-greeter-session
      ├─ noctalia-greeter-compositor
      └─ noctalia-greeter
         └─ PAM-authenticated request through GREETD_SOCK
            └─ weyriva.desktop
               └─ weyriva session start
                  └─ niri-session
                     └─ niri.service / graphical-session.target
                        ├─ weyriva-shell.service
                        └─ weyriva-ipc.service
```

greetd is not removed. It is the hidden privileged broker for VT/seat control,
PAM, credential and account transition, session accounting, and starting the
selected Wayland session. Weyriva does not rewrite PAM or provide autologin.
TTY2 remains the permanent recovery path.

Noctalia Greeter replaces tuigreet in the target architecture. Exact login,
lock, recovery, and logging behavior is in
[Session lifecycle](SESSION_LIFECYCLE.md).

## Surface ownership

One session has one owner for each visible surface:

| Surface | Owner |
| --- | --- |
| Login | Noctalia Greeter |
| Compositor/window layout | Niri |
| Bar/tray/taskbar | Noctalia v5 |
| Launcher/control center/settings | Noctalia v5 |
| Notifications/history | Noctalia v5 |
| Clipboard history | Noctalia v5 |
| Wallpaper/backdrop | Noctalia v5 |
| OSD/screenshots | Noctalia v5 |
| Idle/lock/session controls | Noctalia v5 |
| Desktop/lock widgets | Noctalia v5 |
| Current v5 plugin UI/services | Noctalia v5 |

Waybar, fuzzel, mako, swaybg, swaylock, and swayidle must not be started beside
Noctalia. Duplicate process ownership causes the exact class of “visible but
dead” or conflicting controls this architecture is designed to prevent.

## Isolated profile

Before executing Noctalia, `weyriva shell` sets:

```text
NOCTALIA_CONFIG_HOME = $XDG_CONFIG_HOME/weyriva
NOCTALIA_STATE_HOME  = $XDG_STATE_HOME/weyriva
NOCTALIA_DATA_HOME   = $XDG_DATA_HOME/weyriva
```

Noctalia appends `/noctalia`. The resulting profile contains declarative
configuration, app-managed overrides, palette and wallpaper assets, plugin
sources, materialized plugins, and catalog state without adopting a user's
standalone Noctalia profile.

The packaged defaults are deterministic:

- wallpaper-derived `soft` colors with a complete palette available as an
  explicit offline fallback selection;
- light, dark, and fixed-schedule automatic mode;
- one 400ms wallpaper fade;
- launcher, terminal, clipboard, workspaces, DND, theme, status, and recovery
  at first-class reach;
- official and community Noctalia v5 plugin sources.

## systemd user lifecycle

Niri's systemd integration owns `graphical-session.target`. Weyriva units are
enabled through `niri.service.wants`, not started twice through an additional
Niri `spawn-at-startup`.

`weyriva-shell.service` executes:

```bash
weyriva shell run
```

The CLI uses an argument array and replaces itself with the Noctalia binary.
The service is graphical-session-bound, restarts on failure with a maximum of
three attempts in 30 seconds, and invokes a failsafe session exit after that
budget. It must not declare `WatchdogSec`, because Noctalia does not send
watchdog keepalives.

After a crash in a locked session, the replacement shell must reconcile logind
state and immediately reacquire `ext-session-lock-v1`; if it cannot prove that
secure state, it exits the graphical session and returns to Greeter.

These behaviors require live failure injection before acceptance.

## Control planes

There are two intentionally separate control planes:

1. `weyriva shell msg ...` delegates native commands to the running Noctalia
   engine using the isolated profile.
2. `weyriva ipc call weyriva.*` speaks Weyriva's versioned local JSON protocol
   for diagnostics, compositor queries, and legacy executable plugins.

Neither lane joins user arguments into a shell command. The exact boundary is
documented in [IPC](IPC.md).

## Plugin architecture

There are three distinct lanes:

1. **Native Noctalia v5.** `plugin.toml` and trusted Luau run directly in the
   installed Noctalia engine and its engine-declared API range.
2. **Legacy Weyriva executables.** Version-1 JSON manifests extend only the
   reserved local IPC lane.
3. **Legacy Noctalia v4 QML.** `manifest.json` and QML require an isolated
   Quickshell companion host and remain unimplemented/unaccepted.

Noctalia v5 exposes disable, not per-plugin deletion. Details are in
[Plugins](PLUGINS.md).

## Privilege and security boundaries

- greetd owns PAM/VT/session privilege; Weyriva does not imitate it in a user
  service.
- Greeter appearance sync stages user data and invokes the packaged privileged
  apply helper through polkit/run0.
- `greeter.toml` is declarative and wins over mutable `sync.toml`.
- Native and legacy plugins are trusted user code, not sandboxed.
- The `weyriva.*` socket is protected by per-user filesystem permissions, not
  an authentication protocol.
- User files and state outside managed Weyriva paths are not adopted or
  overwritten without preservation.

## Design boundary

Apple-inspired interaction rules govern hierarchy, material, input, and motion.
Anthropic-inspired project artwork is restricted to wallpaper and illustration
layers. Neither reference implies endorsement. See
[Design system](DESIGN_SYSTEM.md), [Theming](THEMING.md), and
[Motion](MOTION.md).

## Invariants

- One login surface, one compositor, one shell engine, one owner per surface.
- greetd remains an internal privileged broker; no PAM rewrite or autologin.
- TTY2 recovery remains available.
- Niri/systemd starts each Weyriva user service once.
- Lock reconciliation fails closed.
- The Noctalia engine always receives the isolated Weyriva roots.
- Native v5 plugins run unchanged through their installed engine.
- v4 compatibility is not claimed before its companion host and real tests.
- Installation has one default and no personalization prompt.
- Completion requires repository, package, interaction, and XRY evidence.
