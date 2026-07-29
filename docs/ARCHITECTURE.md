# Architecture

Weyriva is an independent desktop shell built with Quickshell 0.3 and QtQuick
for a Niri Wayland session. Weyriva owns every visible surface. greetd is the
internal PAM/VT/session broker. Noctalia is neither a runtime dependency nor a
surface owner in the target architecture; it is a pinned public behavior and
plugin-ABI reference.

The repository is in migration. This document defines the end state and labels
current evidence without treating transitional delegation code as product
architecture.

## Process model

Target boot and session chain:

```text
systemd
└─ greetd.service                    privileged, non-visual broker
   └─ Weyriva Greeter                Quickshell login surface
      └─ greetd PAM/session request
         └─ niri-session
            ├─ Weyriva Shell         Quickshell desktop + lock UI
            ├─ /usr/bin/weyriva      Rust startup/session/control daemon
            └─ /usr/bin/weyriva-luau-host
                                     one bounded Rust process per entry
```

This chain is implemented in repository source and local packaging: startup,
shell, session, diagnosis, the resident daemon, and the bounded Luau host are
Rust. The root installer, user units, and AUR recipe consume both binaries.
That does not prove a clean package install or deployment of this revision.

## Ownership

| Boundary | Owner | Current status |
|---|---|---|
| PAM, VT, seat, account transition | greetd and distribution PAM stack | Implemented scaffold; live acceptance pending |
| Login visuals and input | Weyriva Greeter | In progress |
| Compositing and workspace policy | Niri | Implemented configuration; live acceptance pending |
| Bar, tray, launcher, calendar, control center | Weyriva Shell | In progress |
| Notifications, clipboard, wallpaper, OSD, settings | Weyriva Shell | In progress |
| Screenshot UX and desktop widgets | Weyriva Shell | In progress |
| Authenticated lock and idle UI | Weyriva Shell | In progress |
| Native shell IPC | Weyriva Shell | In progress |
| Rust CLI, daemon, startup/session control, diagnose, Unix IPC | `crates/weyriva/` → `/usr/bin/weyriva` | Implemented and package-wired locally; installed-system verification pending |
| API 3 launcher-provider host | `crates/weyriva-luau-host/` → `/usr/bin/weyriva-luau-host` | Implemented, locally tested, and package-wired; target verification pending |
| Other plugin compatibility hosts | Weyriva | Planned |

No two processes may own the same visible surface. Parallel bars, launchers,
notification daemons, wallpaper hosts, or lockers are rejected because they
produce ambiguous input and lifecycle ownership.

## Runtime layers

The native runtime has four logical layers:

1. **Platform adapters** — Niri IPC, Wayland protocols, logind, PipeWire,
   portals, network, power, and clipboard.
2. **State and policy** — session state, settings, theme tokens, notification
   model, plugin registry, and recovery decisions.
3. **Surface controllers** — lifecycle and focus ownership for each shell,
   greeter, panel, overlay, and lock surface.
4. **QtQuick presentation** — components, layout, input, animation, semantics,
   and the project-owned visual system.

Adapters must not reach directly into visual components. Surface controllers
translate state into explicit view models and actions. This keeps tests
deterministic and prevents panels from becoming collections of shell commands.

## Session and privilege boundary

greetd owns only privileged login mechanics. Weyriva Greeter communicates over
`GREETD_SOCK`, displays authentication progress, and requests a session. It
does not read password databases, invent an authentication protocol, modify
the distribution PAM stack, or enable autologin.

The desktop process runs as the authenticated user. Privileged actions must use
an existing narrowly scoped system mechanism and expose progress and failure;
the shell must never gain broad root privileges.

The lock surface belongs to the authenticated session and must acquire
`ext-session-lock-v1`. If recovery cannot prove secure ownership after a shell
failure, the safe outcome is to terminate the graphical session and return to
the greeter.

## Module and control-plane map

- `crates/weyriva/` is the source for the CLI, resident daemon,
  startup, shell, session, diagnose, plugin lifecycle, bounded actions, and
  user-local Unix socket.
- `crates/weyriva-luau-host/` is the bounded Luau entry runtime.
- `shell/` and `greeter/` own QtQuick presentation and must not execute plugin
  source.

The complete documented `weyriva` command surface and the API 3
launcher-provider slice of `weyriva-luau-host` are implemented, and all local
startup/install/service/package consumers use the Rust binaries. Broader
plugin profiles and installed-environment acceptance remain incomplete.
Weyriva reserves `weyriva.*` for its versioned JSON protocol.

All control paths use structured argument arrays or typed messages. User
content must never be interpolated into shell command strings.

## Plugin architecture

The plugin product is **Weyriva Plugins**. Compatibility is implemented by
Weyriva-owned hosts:

1. `crates/weyriva-luau-host/`, targeting `/usr/bin/weyriva-luau-host` and
   currently conformant only for the API 3 single-launcher-provider slice of
   `noctalia-v5-luau/1`;
2. a planned isolated Quickshell/QML host for the documented Noctalia v4 public
   ABI.

Python is repository test tooling only. It is not a production host, runtime
dependency, compatibility profile, or plugin-authoring language.

Noctalia source internals are not copied. Compatibility is established through
public docs, manifests, and black-box conformance fixtures. Details are in
[Plugins](PLUGINS.md) and
[the compatibility contract](plugins/compatibility-contract.md).

The locally tested Rust plugin core owns ordered pinned and local sources,
digest-addressed immutable installs, lifecycle state, persistent host sessions,
safe OS action adapters, and Unix IPC for the accepted slice. QML is
presentation only: it never loads plugin code and receives a validated launcher
model, including declared provider categories, over the versioned daemon/host
boundary.

The locally tested plugin slice is package-wired, but it is not a
target-machine acceptance claim. Clean package installation, service behavior,
shell interaction, and XRY remain separate gates. The profile identifier
contains `v5` only to name an upstream compatibility target; it is not a
Weyriva product version.

## Design boundary

The QtQuick component library implements the Weyriva design system. Apple-style
behavior owns functional chrome: exact source-screen ownership, immediate
feedback, trigger-specific symmetric paths, current-value interruptible motion,
and reduced-motion alternatives. Anthropic-style art is restricted to the
environment, brand moments, greeter/lock composition, and genuine empty states.
Functional panels use a command palette, compact utility popovers, semantic
light/dark surfaces, and structured workspaces—not a universal carrier, card
grid, or dead controls. Neither reference controls runtime architecture.

## Migration invariants

- Weyriva is the only visible shell/greeter/lock owner.
- greetd remains the internal PAM/VT broker.
- Niri remains the compositor.
- Noctalia is reference material only.
- Rust owns production control and Luau execution; Python is test tooling only.
- A rendered surface is not accepted until its controls work.
- Manifest parsing and catalog listing are not plugin compatibility.
- Source, package, installed runtime, and XRY evidence are reported separately.
- Unsupported or unverified behavior is visible in the ledger, never implied.

See [Session lifecycle](SESSION_LIFECYCLE.md),
[IPC](IPC.md), and [Testing](TESTING.md).
