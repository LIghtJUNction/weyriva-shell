# Compatibility and parity ledger

This historical filename is retained for stable links. The ledger no longer
describes Noctalia as Weyriva's engine. Noctalia is a pinned public behavior and
plugin-ABI reference used to define observable compatibility targets.

Status meanings are defined in the [README](../README.md).

## Reference baselines

| Reference | Pin | Purpose | Status |
|---|---|---|---|
| Noctalia docs | `a0fcbcafc709836f46e1c23b18ade6947d442e26` | Public v5/v4 behavior and ABI | Reviewed |
| Official v5 plugins | `4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6` | Public manifest/runtime corpus | Reviewed; Kaomoji probed |
| Community v5 plugins | `35afaa444de6389164360b1ecadb87c972b32912` | Compatibility corpus | Reviewed |
| Legacy v4 plugins | `ea21cb63d063075bc0acd72d8b946ce2c5eef00d` | QML compatibility corpus | Reviewed |

Reviewing a reference is not runtime acceptance.

## Shell surfaces

| Surface | Target owner | Repository status | Real interaction verified |
|---|---|---|---|
| Bar/tray/taskbar | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Launcher | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Calendar | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Control center | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Notifications/history | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Clipboard history | Weyriva | In progress | No |
| Wallpaper | Weyriva | UI iteration 3 source reviewed; preview only | No |
| OSD | Weyriva | In progress | No |
| Settings | Weyriva | UI iteration 3 source reviewed; preview only | No |
| Screenshots | Weyriva | In progress | No |
| Desktop widgets | Weyriva | In progress | No |
| Login | Weyriva Greeter over greetd | UI iteration 3 source reviewed; lifecycle unaccepted | No |
| In-session lock | Weyriva over session-lock protocol | UI iteration 3 source reviewed; security unaccepted | No |

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
| One-command zero-choice install | Implemented with both Rust binaries | clean-machine install |
| Arch package | Local recipe cut over to both Rust binaries | clean `makepkg`, install, and publication |
| Other distributions | Planned | per-distribution installation evidence |
| XRY | UI iteration 3 preview plus prior control-plane milestone | current all-Rust deployment plus complete login/desktop/lock/plugin interaction |

## Plugin compatibility

| Lane | Status | What is still required |
|---|---|---|
| Noctalia v5-compatible Luau | Rust API 3 single-launcher-provider slice passed locally and package-wired | clean installed-runtime evidence, remaining five entry kinds, APIs 4–19, full corpus and XRY |
| Noctalia v4-compatible QML | Planned | isolated host, imports/context, render/input/settings/IPC |

Catalog listing, manifest parsing, or copied files do not satisfy this table.
See [plugin conformance fixtures](plugins/conformance-fixtures.md).

## Design and accessibility

| Requirement | Status |
|---|---|
| Shared semantic tokens and component states | In progress |
| Anthropic-inspired environment/brand/empty-state grammar | Implemented in UI iteration 3 source |
| Apple-inspired source-owned functional chrome and interruptible motion | Implemented in UI iteration 3 source; runtime acceptance pending |
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
