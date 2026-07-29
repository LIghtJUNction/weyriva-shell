# Compatibility and parity ledger

This historical filename is retained for stable links. The ledger no longer
describes Noctalia as Weyriva's engine. Noctalia is a pinned public behavior and
plugin-ABI reference used to define observable compatibility targets.

Status meanings are defined in the [README](../README.md).

## Reference baselines

| Reference | Pin | Purpose | Status |
|---|---|---|---|
| Noctalia docs | `b1e6e9b5235995ba6716d1814b4b127714d8f172` | Public v5/v4 behavior and ABI | Reviewed |
| Official v5 plugins | `d8616f06f707ca6ba99526fb45e0b8fae672259a` | Public manifest corpus | Reviewed |
| Community v5 plugins | `6cee9bbcc726c29e3c1190ae52c6e6135f6819ce` | Compatibility corpus | Reviewed |
| Legacy v4 plugins | `ea21cb63d063075bc0acd72d8b946ce2c5eef00d` | QML compatibility corpus | Reviewed |

Reviewing a reference is not runtime acceptance.

## Shell surfaces

| Surface | Target owner | Repository status | Real interaction verified |
|---|---|---|---|
| Bar/tray/taskbar | Weyriva | Initial source; in progress | No |
| Launcher | Weyriva | Initial source; in progress | No |
| Calendar | Weyriva | Initial source; in progress | No |
| Control center | Weyriva | Initial source; in progress | No |
| Notifications/history | Weyriva | Initial source; in progress | No |
| Clipboard history | Weyriva | In progress | No |
| Wallpaper | Weyriva | Initial source; in progress | No |
| OSD | Weyriva | In progress | No |
| Settings | Weyriva | Initial source; in progress | No |
| Screenshots | Weyriva | In progress | No |
| Desktop widgets | Weyriva | In progress | No |
| Login | Weyriva Greeter over greetd | Initial source; in progress | No |
| In-session lock | Weyriva over session-lock protocol | Initial source; in progress | No |

“Visible” will not count as passing. Buttons, dates, fields, scrolling, focus,
keyboard navigation, disabled states, errors, and completion feedback must all
be exercised.

## Architecture and lifecycle

| Contract | Status | Required evidence |
|---|---|---|
| Runs without Noctalia installed | In progress | clean runtime launch |
| One surface owner per function | In progress | process and surface audit |
| greetd remains internal broker | In progress | installed config + PAM + boot |
| Lock fails closed | In progress | locked crash/restart test |
| Bounded recovery | In progress | injected crash and logs |
| One-command zero-choice install | Implemented scaffold | clean-machine install |
| Arch package | In progress | clean `makepkg` and install |
| Other distributions | Planned | per-distribution installation evidence |
| XRY | Planned | exact-revision deployment and acceptance |

## Plugin compatibility

| Lane | Status | What is still required |
|---|---|---|
| Weyriva legacy executable JSON | Implemented locally | security and installed-runtime regression |
| Noctalia v5-compatible Luau | In progress | six entry kinds, API levels, lifecycle, state, IPC, UI |
| Noctalia v4-compatible QML | Planned | isolated host, imports/context, render/input/settings/IPC |

Catalog listing, manifest parsing, or copied files do not satisfy this table.
See [plugin conformance fixtures](plugins/conformance-fixtures.md).

## Design and accessibility

| Requirement | Status |
|---|---|
| Shared semantic tokens and component states | In progress |
| Original Anthropic-inspired artwork grammar | Implemented as documentation/assets; runtime integration pending |
| Immediate and interruptible interaction feedback | In progress |
| Reduced motion and transparency | In progress |
| Keyboard-only operation | In progress |
| Screen-reader semantics | Planned |
| Contrast and text-scale matrix | Planned |

## Completion rule

Weyriva is ready for the requested demonstration only when:

1. the independent Quickshell runtime owns the surfaces;
2. repository checks pass;
3. the installed package matches the reviewed revision;
4. every required control is interacted with;
5. login, lock, suspend, crash, and recovery pass;
6. representative plugin fixtures execute;
7. XRY evidence is recorded and independently reviewed.
