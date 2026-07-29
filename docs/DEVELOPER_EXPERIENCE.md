# Developer experience

Weyriva ships one zero-configuration “vibe-coding” workflow. It favors fast
keyboard access, visible system state, predictable workspaces, and a reliable
recovery path. The installer does not ask which terminal, launcher, theme, or
layout to use. A substantially different workflow belongs in a maintained fork.

## Default flow

| Action | Shortcut | Owner |
| --- | --- | --- |
| Launcher | `Mod+Space` | Noctalia launcher |
| Terminal | `Mod+Return` | Foot |
| Clipboard history | `Mod+V` | Noctalia clipboard |
| Control center/status | `Mod+C` | Noctalia control center |
| Do Not Disturb | `Mod+N` | Noctalia notifications |
| Toggle bar | `Mod+B` | Noctalia bar |
| Wallpaper picker | `Mod+W` | Noctalia wallpaper panel |
| Theme mode | `Mod+Shift+T` | Noctalia theme mode |
| Session/recovery actions | `Mod+Shift+E` | Noctalia session panel |
| Lock | `Mod+Shift+X` | Noctalia lock |
| Region screenshot | `Print` | Noctalia screenshot |
| Close window | `Mod+Q` | Niri |
| Focus left/down/up/right | `Mod+H/J/K/L` | Niri |
| Move column left/right | `Mod+Shift+H/L` | Niri |
| Workspace 1/2/3 | `Mod+1/2/3` | Niri |
| Move to workspace 1/2/3 | `Mod+Shift+1/2/3` | Niri |

The live Niri configuration is authoritative if a shortcut changes. README and
this table must be updated in the same change.

## Launcher

The launcher is the primary entry point for applications, commands,
calculator/emoji results, session actions, windows, wallpapers, and native
plugin providers. Prefer canonical Noctalia providers over parallel scripts or
a second launcher.

If an action fails, the launcher or resulting notification must expose the
failure. A row that highlights but never activates does not pass.

## Terminal and workspaces

Foot is the fixed terminal because it is small, Wayland-native, and available
on Arch-family systems. Applications marked `Terminal=true` are launched
through Noctalia's terminal discovery path. The default three workspaces keep
the workflow obvious:

1. editor and primary task;
2. terminal/build/logs;
3. browser/reference/communication.

This is a convention, not a hard workspace restriction. Forks may change it;
the base installer does not prompt.

## Clipboard and focus

`Mod+V` opens Noctalia's encrypted clipboard-history surface. Native live
clipboard behavior and stored history are distinct; disabling history must not
break ordinary text-field copy/paste.

`Mod+N` toggles Do Not Disturb. It suppresses notification toasts while history
continues to collect. Use it for focus sessions instead of disabling the
notification daemon or starting a second daemon.

Useful status commands:

```bash
weyriva shell msg notification-dnd-status
weyriva shell msg clipboard-text
weyriva shell msg status
weyriva status
```

## Theme and screenshots

The default mode follows a fixed zero-configuration day/night schedule.
`Mod+Shift+T` is the manual light/dark override. Exact scriptable controls are:

```bash
weyriva shell msg theme-mode-get
weyriva shell msg theme-mode-toggle
weyriva shell msg theme-mode-set auto
weyriva shell msg color-scheme-get
```

`Print` starts region capture. Full-screen and output-specific paths are also
available:

```bash
weyriva shell msg screenshot-fullscreen
weyriva shell msg screenshot-fullscreen pick
weyriva shell msg screenshot-fullscreen DP-1
weyriva shell msg screenshot-fullscreen all
```

## Developer status

The control center and bar expose network, Bluetooth, volume, brightness,
battery, media, notifications, tray, and session state. Detailed shell and
compositor diagnostics remain available without depending on those buttons:

```bash
weyriva diagnose
weyriva diagnose --json
weyriva shell msg status
weyriva ipc call weyriva.info
weyriva ipc call weyriva.niri.outputs
weyriva ipc call weyriva.niri.windows
```

## Recovery

If a panel is closed or stale:

```bash
weyriva shell msg config-reload
systemctl --user restart weyriva-shell.service
```

Restarting the shell is a bounded recovery action, not a substitute for fixing
dead controls. When the session is locked, restart must reconcile lock state or
fail closed as documented in [Session lifecycle](SESSION_LIFECYCLE.md).

If the graphical session cannot recover, switch to TTY2 and inspect:

```bash
journalctl --user -u weyriva-shell.service -b --no-pager
weyriva diagnose
```

Do not repeatedly restart greetd from an active graphical session; that ends the
login session.

## Fork policy

The base distribution intentionally has no installer questionnaire. Fork when
changing:

- terminal, compositor, login surface, or shell engine;
- workspace count or navigation model;
- package policy or supported distributions;
- global visual language or palette transform;
- plugin trust policy;
- default privacy/network behavior;
- authentication/session architecture.

Small runtime choices already exposed by Noctalia Settings remain user state,
but Weyriva does not promise to preserve a customized distribution profile
across arbitrary fork-level changes.
