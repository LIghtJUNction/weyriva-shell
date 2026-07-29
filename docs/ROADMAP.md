# Status and roadmap

Weyriva is migrating to an independent Quickshell 0.3 / QtQuick shell. This
roadmap is an acceptance sequence, not a list of completed marketing claims.

## Current evidence

**Implemented in the repository**

- zero-choice `./install.sh` entry point and installation safety scaffolding;
- Niri session configuration and fixed keybinding intent;
- initial native Quickshell desktop, greeter, component, session-lock, and
  shell-IPC source;
- versioned local JSON IPC daemon and diagnostics;
- bounded legacy executable-plugin handling;
- repository tests and static checks;
- the independent architecture and clean-room plugin compatibility contracts.

**In progress**

- removal of Noctalia runtime delegation and stale dependency/config paths;
- completion and validation of the native Quickshell application and shared
  component system;
- bar, launcher, calendar, control center, notifications, wallpaper, OSD,
  settings, screenshot, and desktop-widget surfaces;
- Weyriva Greeter and integrated authenticated lock surface;
- native IPC and Luau plugin compatibility.

**Planned**

- isolated v4 QML compatibility host;
- clean Arch package build and publication;
- verified best-effort installers on other distributions;
- complete accessibility matrix;
- XRY login, desktop, lock, recovery, visual, keyboard, and pointer acceptance.

Nothing in the second or third list is complete until the corresponding tests
and environment evidence exist.

## Milestones

### M1 — Independent runtime

- start one Weyriva Quickshell 0.3 process inside Niri;
- remove Noctalia runtime/package/profile delegation;
- establish typed state, surface routing, logs, and native IPC;
- prove bounded startup and clean shutdown.

Exit: shell starts without Noctalia installed and a native status surface
responds to pointer, keyboard, and IPC.

### M2 — Usable desktop surfaces

- implement bar/tray, launcher, calendar, control center, notifications,
  clipboard, wallpaper, OSD, settings, screenshots, and desktop widgets;
- use one component library and shared theme tokens;
- test empty, loading, error, disabled, pressed, focus, and success states.

Exit: every visible control in the interaction matrix performs its action.

### M3 — Login, lock, and recovery

- implement Weyriva Greeter over greetd;
- implement secure in-session lock with `ext-session-lock-v1`;
- qualify idle, suspend, resume, logout, crash-loop, locked restart, and TTY
  recovery;
- fail closed when lock ownership cannot be established.

Exit: cold-boot and recovery matrix passes on target hardware.

### M4 — Plugin compatibility

- implement documented v5 Luau API levels incrementally;
- run self-authored fixtures for all six entry kinds, lifecycle, config, state,
  IPC, persistence, and error isolation;
- implement the v4 host only after the v5 path is stable.

Exit: the compatibility matrix records execution evidence, not catalog
presence.

### M5 — Packaging and cross-distribution install

- make the root script resolve only the independent Weyriva runtime;
- build the Arch package in a clean environment;
- prove rollback and preservation of unmanaged files;
- document best-effort support based on real package availability.

Exit: clean-machine install, update, and uninstall tests pass.

### M6 — XRY acceptance

- install the exact reviewed revision;
- restart only the explicitly authorized unused desktop environment;
- record screenshots, logs, pointer and keyboard results;
- verify login, desktop, lock, suspend, recovery, and representative plugins.

Exit: all required XRY evidence is attached and the Reviewer approves.

See [Testing](TESTING.md) and
[the compatibility ledger](NOCTALIA_PARITY.md).
