# Testing and acceptance

Weyriva is complete only when repository checks, package/session integration,
and live XRY interaction agree. Source inspection, a successful compile, or a
healthy process does not prove that buttons, calendars, login, lock, plugins,
or accessibility work.

## Evidence levels

| Level | Proves | Does not prove |
| --- | --- | --- |
| Static | Syntax, schema, policy, file relationships | Runtime rendering or interaction |
| Isolated runtime | Weyriva profile loads and native IPC answers | Login/PAM, hardware, all controls |
| Packaged session | Installed paths, dependencies, systemd chain | Full interaction/catalog coverage |
| XRY acceptance | Real visual, pointer, keyboard, lock, and recovery behavior | Other hardware/distributions |

## Repository checks

Focused unit tests:

```bash
make test
```

Full repository check:

```bash
make check
```

Before handoff also run:

```bash
git diff --check
```

Report skipped runtime checks explicitly. The absence of `noctalia`, `niri`,
packaging tools, or a Wayland session is not a pass.

## Configuration checks

```bash
weyriva shell config validate
weyriva shell config export full
```

Verify the export resolves:

- one bar/notification/wallpaper/lock owner;
- `theme.source = "wallpaper"`;
- `theme.wallpaper_scheme = "soft"`;
- deterministic auto schedule;
- one 400ms fade transition;
- both light and dark wallpaper assets;
- native Greeter auto-sync when the Greeter package is available;
- official and community v5 plugin sources.

Remember that `settings.toml` loads after the declarative profile.

## Palette semantic lint

`noctalia config validate` validates TOML but may not catch invalid custom
palette resources. Tests must parse `Weyriva.json` and require, for both dark
and light:

- all 16 UI roles;
- valid opaque hex values;
- terminal background, foreground, cursor, cursor text, selection background,
  and selection foreground;
- eight `normal` ANSI colors;
- eight `bright` ANSI colors;
- readable foreground/background, selection, and cursor pairs.

The runtime acceptance additionally checks:

```bash
weyriva shell msg color-scheme-set custom Weyriva
weyriva shell msg color-scheme-get
journalctl --user -u weyriva-shell.service -b --no-pager
```

There must be no invalid-custom-palette fallback in the journal. Restore the
intended wallpaper source afterward:

```bash
weyriva shell msg color-scheme-set wallpaper soft
```

## Native shell IPC smoke

Read-only:

```bash
weyriva shell msg status
weyriva shell msg theme-mode-get
weyriva shell msg color-scheme-get
weyriva shell msg wallpaper-get
weyriva shell msg notification-dnd-status
```

State-changing checks must record and restore prior state:

```bash
weyriva shell msg theme-mode-set light
weyriva shell msg theme-mode-set dark
weyriva shell msg theme-mode-set auto
weyriva shell msg color-scheme-set wallpaper soft
```

Test wallpaper paths with spaces, `color:#RRGGBB`, all-monitor and one-monitor
forms. Verify persistence in the isolated state layer and visible output.

## Interaction matrix

For each light, dark, high-contrast, normal-motion, and reduced-motion mode:

- click every bar widget and control-center shortcut;
- repeat activation by keyboard;
- operate launcher results, categories, settings, clipboard rows, wallpaper
  tiles, notification actions, tray menus, media, network, Bluetooth, audio,
  brightness, battery, session, and screenshot controls;
- test calendar previous/next/today/date/account/refresh/event interactions;
- verify disabled/loading/error states and missing optional dependencies;
- verify Escape dismissal and focus return;
- capture screenshots of desktop, settings, panels, notifications, OSD, lock,
  and Greeter.

A process-health check cannot replace this matrix.

## Plugin matrix

Pin official and community source commits. For every catalog ID:

1. lint manifest and dependency declarations;
2. enable and poll until the final state;
3. exercise each declared entry;
4. open settings where provided;
5. send plugin IPC where provided;
6. update the owning source and verify controlled reload;
7. disable and verify surfaces/services disappear;
8. re-enable and verify state;
9. test missing declared dependencies;
10. record installed-engine API acceptance or rejection.

The matrix must cover widget, shortcut, launcher provider, desktop widget,
panel, and service. Noctalia v5 has no per-plugin remove operation; do not test
disable as deletion. See [Plugins](PLUGINS.md).

## systemd and package checks

Arch/AUR acceptance includes:

```bash
makepkg --printsrcinfo
makepkg --verifysource
```

A real package build is required before publication, but do not run it
concurrently with another heavy build. Inspect the resulting package contents
for:

- Noctalia, Noctalia Greeter, greetd, Niri, Foot, fonts, and required policy
  dependencies;
- no tuigreet runtime dependency;
- greetd template using `noctalia-greeter-session`;
- user services wired through Niri/systemd without duplicate startup;
- wallpaper and palette assets;
- session desktop entry and helper paths;
- preservation-first install/update behavior.

Other distributions remain best effort until their native package names,
Greeter availability, PAM stack, and a real install have been exercised.

## Login, PAM, crash, and lock matrix

- cold boot to visible Noctalia Greeter;
- valid password, invalid password, empty password rejection, and cancellation;
- session and user selection;
- no autologin;
- TTY2 remains reachable;
- clean login/logout/relogin;
- lock by shortcut, IPC, idle, and pre-suspend path;
- resume remains locked;
- kill the shell while unlocked and verify bounded restart;
- kill it while locked and verify immediate secure lock reconciliation;
- trigger more than three failures in 30 seconds and verify failsafe session
  exit to Greeter;
- test missing/corrupt profile and unavailable Greeter sync helper;
- inspect greetd, Niri, shell, and IPC journals.

Do not modify PAM merely to make a failing test pass. Diagnose the distribution
stack and document the compatibility boundary.

## XRY evidence

Each result uses:

```text
id:
commit:
installed package/version:
surface:
mode and scale:
input path:
precondition:
action:
expected:
observed:
evidence:
logs:
status: pass | fail | blocked | not-run
residual risk:
```

Required visual evidence includes login, desktop, all primary panels, calendar,
settings, notifications, OSD, plugin surfaces, lock screen, light/dark,
high-contrast, reduced-motion, and at least one failure state. Required click
evidence includes the exact control and observed result, not only a screenshot
of the open surface.

## Completion rule

The parity ledger changes to complete only when:

- the installed commit matches the reviewed source;
- repository and package checks pass;
- no duplicate surface owner exists;
- all required pointer and keyboard interactions pass;
- login/PAM/lock/crash recovery passes;
- catalog coverage is recorded;
- XRY screenshots and journals are attached;
- residual risks are explicit;
- legacy v4 remains labelled pending until its companion host is real.
