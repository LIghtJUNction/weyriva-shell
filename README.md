# Weyriva Shell

Weyriva Shell (pronounced **way-REE-vuh**) is an Arch Linux-first,
zero-configuration Niri desktop built around the upstream
[Noctalia v5](https://github.com/noctalia-dev/noctalia) shell. One visual and
lifecycle contract covers the login screen, authenticated lock screen, and
desktop.

Noctalia Greeter is the visible login layer. greetd remains underneath as the
hidden, narrowly scoped broker for VT ownership, PAM authentication, account
transition, and session creation. Weyriva does not reimplement PAM, enable
autologin, or remove the TTY recovery path.

> **Status:** repository integration is active. A one-command source installer
> exists, but the all-in-one login chain, packaged dependency path, complete
> plugin catalogs, accessibility matrix, crash/lock recovery, and XRY visual and
> click acceptance must pass before this project is called install-ready or
> deployed. See the [parity ledger](docs/NOCTALIA_PARITY.md).

## What it delivers

- Noctalia-owned bar, tray, launcher, dock, control center, notifications,
  clipboard, wallpaper, OSD, settings, screenshots, lock/idle, desktop widgets,
  and current v5 plugins;
- Noctalia Greeter for a visually continuous login surface, with greetd hidden
  behind the Weyriva product boundary;
- one Niri/systemd user-session lifecycle with bounded shell recovery;
- light, dark, and deterministic automatic mode with source-faithful,
  gently saturated wallpaper-derived color;
- Apple-inspired interaction hierarchy and motion, paired with project-owned
  Anthropic-inspired editorial wallpaper and illustration;
- keyboard-first vibe-coding defaults for terminal, launcher, workspaces,
  clipboard, DND, theme, screenshots, status, and recovery;
- one fixed profile and no installer personalization questionnaire.

These are design influences, not endorsements. Weyriva is not affiliated with
or sponsored by Apple, Anthropic, or Noctalia.

## Install

Weyriva targets Linux/Niri/Wayland. Arch and Arch-family systems are primary,
with AUR/systemd as the first packaging and service path. Fedora,
Debian/Ubuntu, and openSUSE remain best effort where their repositories provide
compatible Niri, Noctalia, Noctalia Greeter, greetd, and supporting packages.

From a checkout, the supported command is:

```bash
./install.sh
```

The installer has no choices. It preserves replaced managed user files with
timestamped backups and does not silently restart the active graphical session.
Arch installation prefers configured repositories and may use an
already-installed `paru` or `yay` as the invoking ordinary user; it does not
bootstrap an AUR helper.

The installer applies the fixed privileged login template without questions,
backs up the previous template, enables the login service for the next boot,
and never restarts the active graphical session. The packaged path installs the
Noctalia Greeter session and keeps greetd as an internal broker. Until the
gates in [Testing](docs/TESTING.md) pass, do not treat a successful file copy
as a successful desktop deployment.

## Everyday controls

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

The exact workflow is in
[Developer experience](docs/DEVELOPER_EXPERIENCE.md).

## Shell and plugin control

`weyriva shell` always carries the isolated Weyriva config, state, and data
roots:

```bash
weyriva shell run
weyriva shell config validate
weyriva shell msg status
weyriva shell msg panel-toggle launcher
weyriva shell msg settings-toggle
weyriva shell msg theme-mode-get
weyriva shell msg color-scheme-get
weyriva shell msg session lock
```

Current Noctalia v5 plugins run directly through the installed engine:

```bash
weyriva plugin list
weyriva plugin install noctalia/screen_recorder
weyriva plugin disable noctalia/screen_recorder
weyriva plugin enable noctalia/screen_recorder
weyriva plugin update official
weyriva plugin source list
```

`plugin install ID` is an enable/materialize convenience. Noctalia v5 exposes
disable, not per-plugin removal. The old Weyriva JSON executable lane remains
explicitly legacy, and Noctalia v4 QML compatibility remains pending. See
[Plugins](docs/PLUGINS.md).

Weyriva's reserved local control plane is separate:

```bash
weyriva status
weyriva diagnose
weyriva diagnose --json
weyriva ipc call weyriva.info
weyriva ipc call weyriva.niri.outputs
```

## Development and acceptance

```bash
make test
make check
```

CI/local checks do not replace real login, lock, pointer, keyboard, plugin, or
XRY acceptance.

Developer documentation:

- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Session lifecycle](docs/SESSION_LIFECYCLE.md)
- [Design system](docs/DESIGN_SYSTEM.md)
- [Theming](docs/THEMING.md)
- [Motion](docs/MOTION.md)
- [Accessibility](docs/ACCESSIBILITY.md)
- [Developer experience](docs/DEVELOPER_EXPERIENCE.md)
- [IPC](docs/IPC.md)
- [Plugins](docs/PLUGINS.md)
- [Testing](docs/TESTING.md)
- [Noctalia parity](docs/NOCTALIA_PARITY.md)

A concise Chinese overview is in
[docs/README.zh-CN.md](docs/README.zh-CN.md).

## License

MIT. See [LICENSE](LICENSE).
