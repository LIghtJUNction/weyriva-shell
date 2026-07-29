# Weyriva Shell

Weyriva Shell (pronounced **way-REE-vuh**) is an Arch-first, zero-configuration
Niri desktop whose shell, greeter, and lock surfaces are owned by Weyriva.
The target runtime is an independent
[Quickshell 0.3](https://quickshell.org/) / QtQuick implementation. It does not
delegate desktop ownership to Noctalia.

greetd remains the narrowly scoped system broker for VT ownership, PAM
authentication, and session creation. It is not the visible product UI and
Weyriva does not reimplement PAM.

> **Migration status:** the repository is being converted from an earlier
> Noctalia-delegating scaffold to the independent shell described here. The
> source installer, Niri profile, local control daemon, initial native
> Quickshell shell/greeter sources, and repository checks exist. Native
> surfaces, the integrated greeter and lock flow, compatible plugin execution,
> final packaging, and XRY acceptance are not complete merely because those
> scaffolds exist. See the
> [compatibility ledger](docs/NOCTALIA_PARITY.md).

## Product contract

Weyriva is intended to provide one coherent product across:

- login, desktop, authenticated lock, suspend, logout, and recovery;
- bar, tray, launcher, calendar, control center, notifications, clipboard,
  wallpaper, OSD, settings, screenshots, and desktop widgets;
- a native Weyriva control plane and versioned plugin compatibility layers;
- a deterministic default profile with no installer questionnaire.

The design language combines two project-owned influences:

- Apple-inspired direct manipulation: immediate feedback, spatial continuity,
  interruptible motion, and accessible reduced-motion alternatives;
- Anthropic-inspired editorial art: bold uneven near-black linework, irregular
  ivory carrier shapes, and one muted opaque accent field.

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

The product goal is one command on supported Linux systems:

```bash
./install.sh
```

There are no personalization prompts. Arch and Arch-family systems are the
primary target; Fedora, Debian/Ubuntu, and openSUSE are best-effort targets.
Users who need a different policy should fork the repository.

The script exists today, but its dependency and session path are part of the
active native-shell migration. Until the gates in [Testing](docs/TESTING.md)
pass, treat it as an integration scaffold rather than a production-ready
desktop installer. It must not restart an occupied graphical session without
an explicit operational request.

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

## Control and plugins

The current repository includes a versioned local JSON control daemon and a
legacy executable-plugin lane. They are migration infrastructure, not the
desktop renderer. The independent Quickshell runtime will own native surface
IPC.

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
```

Repository checks are necessary but do not replace real login, lock, pointer,
keyboard, plugin, packaging, or XRY evidence.

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
