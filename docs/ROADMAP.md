# Status and roadmap

Weyriva is migrating to an independent Quickshell 0.3 / QtQuick shell. This
roadmap is an acceptance sequence, not a list of completed marketing claims.

## Current evidence

**Implemented in the repository**

- zero-choice `./install.sh` entry point and installation safety scaffolding;
- Niri session configuration and fixed keybinding intent;
- initial native Quickshell desktop, greeter, component, session-lock, and
  shell-IPC source;
- locally tested Rust plugin core and bounded Luau host for the API 3
  single-launcher-provider vertical slice, with fixture and pinned official
  Kaomoji evidence;
- Rust startup, shell, session, diagnosis, resident daemon, and plugin control;
- one-command and AUR-recipe build/package wiring for both Rust binaries;
- independently reviewed UI iteration 3 shell/greeter source; XRY previews
  these trees and retains the previously deployed control-plane milestone, but
  not the current all-Rust cutover;
- repository tests and static checks;
- the independent architecture and clean-room plugin compatibility contracts.

**In progress**

- removal of remaining stale dependency/config paths;
- completion and validation of the native Quickshell application and shared
  component system;
- bar, launcher, calendar, control center, notifications, wallpaper, OSD,
  settings, screenshot, and desktop-widget surfaces;
- Weyriva Greeter and integrated authenticated lock surface;
- integration of the locally tested Rust plugin core, ordered pinned sources,
  immutable state/install, lifecycle, host sessions, actions, Unix IPC, and
  provider categories in QML;
- remaining native IPC and Luau plugin compatibility.

**Planned**

- isolated v4 QML compatibility host;
- clean Arch package build; AUR publication remains pending;
- verified best-effort installers on other distributions;
- complete accessibility matrix;
- full XRY login, desktop, lock, recovery, Rust control-plane, visual, keyboard,
  and pointer acceptance.

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

- retain the passing API 3 launcher-provider slice as the first compatibility
  floor;
- implement the other five entry kinds and APIs 4–19 of
  `noctalia-v5-luau/1` incrementally;
- run self-authored fixtures for all six entry kinds, lifecycle, config, state,
  IPC, persistence, and error isolation;
- implement the v4 QML host only after the Luau profile is stable.

Exit: the compatibility matrix records execution evidence, not catalog
presence.

### M5 — Atomic Rust product cutover — implemented locally

- keep startup, shell, session, and diagnosis in `crates/weyriva/`;
- keep production consumers free of transitional control paths;
- keep installer, systemd units, and AUR metadata on both Rust binaries;
- prove shell/plugin startup and failure behavior from the installed package.

Repository implementation is complete; the exit remains open until a clean
installed package proves startup, failure behavior, and recovery.

### M6 — Packaging and cross-distribution install

- make the root script resolve only the independent Weyriva runtime;
- build the Arch package in a clean environment;
- prove rollback and preservation of unmanaged files;
- verify best-effort `dnf`, `apt`, and `zypper` paths based on real package
  availability.

Exit: clean-machine install, update, and uninstall tests pass.

### M7 — XRY acceptance

- install the exact reviewed revision;
- restart only the explicitly authorized unused desktop environment;
- record screenshots, logs, pointer and keyboard results;
- verify login, desktop, lock, suspend, recovery, and representative plugins.
- verify the installed Rust control plane rather than the UI-only preview.

Exit: all required XRY evidence is attached and the Reviewer approves.

See [Testing](TESTING.md) and
[the compatibility ledger](NOCTALIA_PARITY.md).
