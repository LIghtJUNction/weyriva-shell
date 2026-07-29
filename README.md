# Weyriva Shell

Weyriva Shell (pronounced **way-REE-vuh**) is an Arch-first, zero-configuration
Niri desktop whose shell, greeter, and lock surfaces are owned by Weyriva.
The production architecture uses Rust for the `weyriva` CLI, resident
daemon, startup/session control, diagnosis, and plugin control plane; Rust also
provides the isolated `weyriva-luau-host`.
[Quickshell 0.3](https://quickshell.org/) / QtQuick is the presentation layer
only. It does not delegate desktop ownership to Noctalia.

greetd remains the narrowly scoped system broker for VT ownership, PAM
authentication, and session creation. It is not the visible product UI and
Weyriva does not reimplement PAM.

> **Current status:** the production control plane, startup/session commands,
> diagnosis, plugin lifecycle, and Luau host are implemented in the Rust
> workspace. The one-command installer and local AUR recipe build and package
> both `weyriva` and `weyriva-luau-host`; this is a repository implementation,
> not evidence of an AUR publication or an XRY deployment. The exact
> `noctalia-v5-luau/1` API 3 single-launcher-provider slice is locally tested,
> including pinned official Kaomoji evidence and provider categories carried
> to QML. XRY has the approved UI iteration 3 shell/greeter preview and retains
> the previously deployed control-plane milestone, but not this all-Rust
> cutover. The other five entry kinds, APIs 4–19, v4 QML host, complete native
> surfaces, clean-package evidence, and full XRY acceptance remain incomplete.
> See the
> [compatibility ledger](docs/NOCTALIA_PARITY.md).

The plugin product is named **Weyriva Plugins**. The `v5` token appears only in
the upstream compatibility profile identifier; it is not a Weyriva product
version. Python may be used by repository tests, but it is neither a production
runtime dependency nor a plugin language.

## Product contract

Weyriva is intended to provide one coherent product across:

- login, desktop, authenticated lock, suspend, logout, and recovery;
- bar, tray, launcher, calendar, control center, notifications, clipboard,
  wallpaper, OSD, settings, screenshots, and desktop widgets;
- a native Weyriva control plane and versioned plugin compatibility layers;
- a deterministic default profile with no installer questionnaire.

The design language combines two project-owned influences:

- Apple-inspired product chrome: immediate feedback, exact source-screen
  ownership, source-specific interruptible motion, and reduced-motion
  alternatives;
- Anthropic-inspired editorial art only in backgrounds, brand moments,
  greeter/lock composition, and genuine empty states.

Daily UI uses a focused command palette, compact trigger-aligned utility
popovers, semantic light/dark roles, and larger structured workspaces. It does
not wrap every panel in a universal carrier, use a card grid as default
hierarchy, or expose dead controls.

These are design references, not copied products or endorsements. Weyriva is
not affiliated with Apple, Anthropic, or Noctalia.

## Status vocabulary

Documentation uses four explicit states:

| State | Meaning |
|---|---|
| **Implemented** | Present in the repository and covered by a relevant local check |
| **In progress** | The intended architecture is fixed, but implementation or integration is incomplete |
| **Planned** | Accepted scope with no sufficient implementation evidence yet |
| **Verified** | Exercised in the real environment named by the claim, with recorded evidence |

“Implemented” is not interchangeable with “verified.” In particular, local
tests cannot prove login, PAM, Wayland input, secure lock ownership, plugin UI,
or XRY behavior.

## Installation

The installation entry point is one command on supported Linux systems:

```bash
./install.sh
```

There are no personalization prompts. Arch and Arch-family systems are the
primary target; Fedora, Debian/Ubuntu, and openSUSE are best-effort targets.
Users who need a different policy should fork the repository.

The script resolves packages before mutation, builds both locked Rust release
binaries, validates the command surface, and installs the shell, greeter, user
units, and deterministic defaults without a questionnaire. Arch package
resolution is primary; `dnf`, `apt`, and `zypper` paths are best effort.
Clean-machine, published-AUR, and non-Arch evidence is still required before
calling it production-ready. It does not restart greetd or an inactive user
service.

## Intended everyday controls

The fixed interaction contract is:

```text
Mod+Space       launcher
Mod+Return      terminal
Mod+V           clipboard history
Mod+C           control center
Mod+N           Do Not Disturb
Mod+Shift+T     light/dark override
Mod+W           wallpaper
Mod+Shift+E     session and recovery actions
Mod+Shift+X     lock
Print           region screenshot
Mod+H/J/K/L     focus navigation
Mod+1/2/3       workspaces
```

These bindings describe the target product. Each surface remains unverified
until pointer, keyboard, focus, and visible-state acceptance passes.

## Weyriva Plugins

The locally tested Rust `weyriva` plugin core is the single owner of
ordered pinned sources, safe immutable installation and state, lifecycle, host
sessions, bounded actions, and Unix IPC. Each supported entry gets its own
bounded Rust Luau-host process. QML never loads plugin code; the launcher
bridge renders only validated result data. Local installers and package
metadata consume this Rust control plane; installed-machine and XRY behavior
remain separate evidence gates.

The current compatibility claim is deliberately narrow: API 3, exactly one
launcher-provider entry, verified locally with self-authored fixtures and the
pinned official Kaomoji plugin. Provider categories reach the QML launcher.
It is not a claim that the other five Noctalia entry kinds, APIs 4–19, or the
v4 QML profile work.

Noctalia is used only as a pinned public behavior and plugin-ABI reference.
Weyriva must implement compatible behavior itself; catalog discovery or
manifest parsing is not plugin compatibility. See:

- [Plugins](docs/PLUGINS.md)
- [Compatibility contract](docs/plugins/compatibility-contract.md)
- [Noctalia v5 Luau profile](docs/plugins/noctalia-v5-luau.md)
- [Noctalia v4 QML profile](docs/plugins/noctalia-v4-qml.md)

## Development and acceptance

```bash
make test
make check
./scripts/check.sh
```

The Make targets run locked Rust checks. `scripts/check.sh` adds repository
policy, installer, shell, config, QML, and optional system-tool checks; Python
is used there only as test tooling. No local check replaces real login, lock,
pointer, keyboard, packaging, or XRY evidence.

Documentation:

- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Session lifecycle](docs/SESSION_LIFECYCLE.md)
- [Design system](docs/DESIGN_SYSTEM.md)
- [Theming](docs/THEMING.md)
- [Motion](docs/MOTION.md)
- [Accessibility](docs/ACCESSIBILITY.md)
- [Developer experience](docs/DEVELOPER_EXPERIENCE.md)
- [IPC](docs/IPC.md)
- [Testing](docs/TESTING.md)
- [Status and roadmap](docs/ROADMAP.md)
- [Compatibility ledger](docs/NOCTALIA_PARITY.md)

A concise Chinese overview is available in
[docs/README.zh-CN.md](docs/README.zh-CN.md).

## License

MIT. See [LICENSE](LICENSE).
