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
   └─ Weyriva Greeter                planned Quickshell login surface
      └─ greetd PAM/session request
         └─ niri-session
            ├─ Weyriva Shell         Quickshell desktop + lock surfaces
            └─ Weyriva control plane native or bounded sidecar
```

The exact greeter executable and service wiring remain in progress. A
Noctalia-branded greeter or shell is not an acceptable final implementation.

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
| Local diagnostic JSON IPC | Weyriva control daemon | Implemented locally; integration pending |
| Plugin compatibility hosts | Weyriva | In progress |

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

## Control planes

Weyriva reserves `weyriva.*` for its versioned JSON control protocol. The
native shell IPC surface is independent of any reference implementation.
Transitional bridges may remain during migration, but they are not documented
as stable product APIs and must be removed before release.

All control paths use structured argument arrays or typed messages. User
content must never be interpolated into shell command strings.

## Plugin architecture

Plugin compatibility is implemented by Weyriva-owned hosts:

1. a trusted Luau host compatible with the documented Noctalia v5 public ABI;
2. an isolated Quickshell/QML host for the documented Noctalia v4 public ABI;
3. the existing bounded executable-plugin lane, explicitly marked legacy.

Noctalia source internals are not copied. Compatibility is established through
public docs, manifests, and black-box conformance fixtures. Details are in
[Plugins](PLUGINS.md) and
[the compatibility contract](plugins/compatibility-contract.md).

## Design boundary

The QtQuick component library implements the Weyriva design system. Apple-style
fluidity is translated into immediate feedback, direct tracking, velocity-aware
interruptible motion, and reduced-motion alternatives. Anthropic-style art is
translated into an original flat editorial grammar. Neither reference controls
runtime architecture.

## Migration invariants

- Weyriva is the only visible shell/greeter/lock owner.
- greetd remains the internal PAM/VT broker.
- Niri remains the compositor.
- Noctalia is reference material only.
- A rendered surface is not accepted until its controls work.
- Manifest parsing and catalog listing are not plugin compatibility.
- Source, package, installed runtime, and XRY evidence are reported separately.
- Unsupported or unverified behavior is visible in the ledger, never implied.

See [Session lifecycle](SESSION_LIFECYCLE.md),
[IPC](IPC.md), and [Testing](TESTING.md).
