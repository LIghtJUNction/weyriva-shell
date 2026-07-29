# Noctalia parity

Weyriva targets the complete current Noctalia v5 shell through the upstream
engine, plus a coherent Noctalia Greeter login layer and direct compatibility
with official/community v5 plugins. It does not reimplement the surface list
piecemeal.

This is an acceptance ledger, not a marketing checklist. Source implementation
does not become complete without package, runtime, interaction, and XRY
evidence.

## Reviewed baselines

| Project | Pinned review |
| --- | --- |
| Noctalia source | [`cebcc62284a42620ebb3518b3243665b43c11a96`](https://github.com/noctalia-dev/noctalia/tree/cebcc62284a42620ebb3518b3243665b43c11a96) |
| Noctalia docs | [`f88820cc90170ceb212efdea87711802ebaca1c9`](https://github.com/noctalia-dev/noctalia-docs/tree/f88820cc90170ceb212efdea87711802ebaca1c9) |
| Noctalia Greeter | [`d6275cbcb5b9acae2348bed16e358aa6c2cf8188`](https://github.com/noctalia-dev/noctalia-greeter/tree/d6275cbcb5b9acae2348bed16e358aa6c2cf8188) |

The installed package may use another commit or tagged release. Acceptance must
record the effective engine and Greeter versions and refresh drift-sensitive
expectations.

Status vocabulary:

- **Engine-backed; acceptance pending** — upstream owns the feature, but
  repository/package/XRY evidence is incomplete.
- **Weyriva integration; acceptance pending** — Weyriva adds required wiring,
  policy, or recovery around upstream.
- **Pending implementation** — the required runtime path is not complete.
- **Accepted** — all named evidence exists for the installed commit.

No row is accepted merely because its source is present.

## Shell feature matrix

| Requirement | Intended evidence | Status |
| --- | --- | --- |
| Multi-monitor bars and widgets | Render, click, keyboard, placement, scale on multiple outputs | Engine-backed; acceptance pending |
| Tray and menus | Real StatusNotifier items, menus, actions, error states | Engine-backed; acceptance pending |
| Media | MPRIS metadata, controls, multiple players | Engine-backed; acceptance pending |
| Network/Bluetooth | Real backend state, toggles, secrets/errors | Engine-backed; acceptance pending |
| Battery/power/brightness | Supported hardware and failure behavior | Engine-backed; acceptance pending |
| Weather/calendar | Empty/loading/offline/auth/populated flows and all calendar controls | Engine-backed; acceptance pending |
| Workspaces/taskbar | Niri focus, move, title/state updates | Engine-backed; acceptance pending |
| Dock | Pin, launch, focus, close, multi-monitor | Engine-backed; acceptance pending |
| Launcher | Apps, commands, categories, calculator, session and plugin providers | Engine-backed; acceptance pending |
| Control center | Every tab, shortcut, pointer and keyboard action | Engine-backed; acceptance pending |
| Notifications/history | Daemon ownership, DND, actions, dismiss/history | Engine-backed; acceptance pending |
| Clipboard | Live copy/paste, encrypted history, pins, clear/error states | Engine-backed; acceptance pending |
| Wallpaper/backdrop | Picker, per-monitor, automation, colors, transitions | Engine-backed; acceptance pending |
| OSD | Audio, brightness, radio, power, DND and privacy events | Engine-backed; acceptance pending |
| Screenshots | Region, selected output, all outputs, configured destinations | Engine-backed; acceptance pending |
| Settings/hot reload | All controls, validation, persistence, effective config | Engine-backed; acceptance pending |
| Desktop and lock widgets | Layout/editor, native/plugin entries, input behavior | Engine-backed; acceptance pending |
| Session lock/idle/actions | Shortcut, IPC, idle, suspend, auth, logout/power | Engine-backed; acceptance pending |

## Theme, motion, and accessibility

| Requirement | Intended evidence | Status |
| --- | --- | --- |
| Dynamic color | `source=wallpaper`, `soft`, wallpaper changes, dark/light tokens | Engine-backed; acceptance pending |
| Offline fallback palette | 16 roles plus complete dark/light terminal maps; no fallback log | Weyriva integration; acceptance pending |
| Light/dark/auto | Fixed schedule, manual toggle/widget/IPC, persistence | Weyriva integration; acceptance pending |
| Greeter theme continuity | Palette, wallpaper, font, mode and output sync | Weyriva integration; acceptance pending |
| Deterministic motion | One 400ms fade, no random default effects | Weyriva integration; acceptance pending |
| Reduced motion | Shell animation off plus wallpaper fade/empty mapping | Weyriva integration; acceptance pending |
| High contrast/UI scale | Rendered modes, focus/target/contrast matrix | Engine-backed; acceptance pending |
| Screen readers | Names, roles, state, focus and live events through real tooling | Unverified; no support claim |

The exact contracts are in [Theming](THEMING.md), [Motion](MOTION.md), and
[Accessibility](ACCESSIBILITY.md).

## Login, systemd, and packaging

| Requirement | Intended evidence | Status |
| --- | --- | --- |
| Visible login layer | greetd starts `noctalia-greeter-session`; no tuigreet | Weyriva integration; acceptance pending |
| Privileged boundary | Distro greetd PAM/logind path, no PAM rewrite/autologin | Weyriva integration; acceptance pending |
| Login/desktop chain | display-manager → greetd → Greeter → Weyriva entry → Niri | Weyriva integration; acceptance pending |
| User services | `niri.service.wants`, graphical-session binding, no duplicate spawn | Weyriva integration; acceptance pending |
| Shell crash recovery | Maximum 3 failures/30s then session-exit failsafe | Weyriva integration; acceptance pending |
| Locked crash recovery | Reacquire lock immediately or fail closed | Pending live implementation/evidence |
| TTY2 recovery | VT remains reachable; documented journal/diagnose flow | Weyriva integration; acceptance pending |
| Arch/AUR package | Real clean build, dependency/content inspection, `.SRCINFO` | Pending package build |
| One-command install | Fresh Arch install, backup/ownership/update behavior | Pending live install |
| Other distributions | Native dependency names, install, PAM and session evidence | Best effort; pending |

greetd remains the hidden PAM/VT/session broker; the target is not to delete its
binary. See [Session lifecycle](SESSION_LIFECYCLE.md).

## IPC and plugins

| Requirement | Intended evidence | Status |
| --- | --- | --- |
| Native shell IPC | `weyriva shell msg` against isolated running Noctalia | Engine-backed plus Weyriva integration; acceptance pending |
| Weyriva local IPC | Versioned socket bounds, methods and compositor queries | Weyriva-specific; regression pending |
| Native v5 plugins | Same engine, canonical IDs, all six entry kinds | Catalog matrix pending |
| Official/community catalogs | Every pinned ID lifecycle-tested | Pending |
| Legacy Weyriva executables | Explicit legacy CLI/socket lane only | Regression pending |
| Legacy Noctalia v4 QML | Isolated Quickshell companion host and real rendering | Pending implementation |

Noctalia v5 has no per-plugin remove operation. Disable must not be described or
tested as deletion. See [Plugins](PLUGINS.md).

## Completion gates

| Gate | Evidence | State |
| --- | --- | --- |
| Repository | Unit/static/config/palette checks | Pending final reviewed run |
| Surface ownership | Process/service evidence shows one owner per surface | Pending |
| Arch package | Clean build and content/dependency inspection | Pending |
| Login/PAM | Cold boot, invalid/valid auth, logout/relogin, TTY2 | Pending |
| Lock/recovery | Idle/suspend/crash/locked-crash/crash-loop | Pending |
| Interaction | Every primary button/calendar/settings control by pointer and keyboard | Pending |
| Theme/accessibility | Light/dark/auto/high-contrast/reduced-motion/scale | Pending |
| Plugin catalogs | Complete official/community lifecycle matrix | Pending |
| XRY | Screenshots, click results, journals, installed commit | Pending |
| Legacy v4 | Companion host, rendering, settings, IPC, lifecycle | Pending |

Evidence procedure is defined in [Testing](TESTING.md). Until all applicable
gates pass, README and release notes must say the integration is in progress,
not deployed or fully accepted.
