# Weyriva Shell

Weyriva Shell (pronounced **way-REE-vuh**) is an Arch Linux-first, composed Wayland desktop built around niri. It joins a warm editorial visual language with Waybar, fuzzel, mako, greetd/tuigreet, user services, a small IPC control plane, and trusted local plugins.

> **Status:** version 0.1.0 is a working repository foundation, not a finished desktop distribution. Its runtime, tests, preservation-first installer, and static configuration ship now. Full hardware coverage, a graphical settings app, broad plugin ecosystem, stable AUR release, and real-session qualification remain roadmap work. This repository does not claim that Weyriva is installed or validated in your current desktop session.

The original coral/cream/ink artwork is a project-owned editorial design. Weyriva is not affiliated with, endorsed by, or presented as artwork from Anthropic.

## Components

- niri scrolling compositor and session
- Waybar panel, fuzzel launcher, mako notification daemon (the requested “moko” is treated as the real Arch package `mako`)
- greetd with tuigreet as an explicit, separate system template
- `weyriva` Python standard-library CLI and protocol-v1 Unix-socket daemon
- explicit-manifest executable plugins under XDG config/data paths
- original cactus editorial PNG wallpaper and graphical-session-bound systemd user services

## Visual system

The default desktop uses a compact ivory Waybar rail, calm ivory launcher and
notifications, and cactus focus and selection states over the bundled cactus
editorial wallpaper. The shared palette is ink `#141413`, ivory `#FAF9F5`, and
cactus `#BCD1CA`; the original project artwork is inspired by an editorial
hand-drawn language and is not affiliated with Anthropic.

## Install

Weyriva is a Linux Niri/Wayland shell, not a Windows or macOS desktop. Arch and
CachyOS are the fully supported path. Fedora, Debian/Ubuntu, and openSUSE use
their native package managers on a best-effort basis; if their repositories do
not provide the required desktop packages, installation stops before any
Weyriva configuration is copied.

Run the one supported setup command from a checkout:

```bash
./install.sh
```

It installs Niri, Waybar, fuzzel, mako, swaybg, swaylock, swayidle, Foot, Noto
Sans, and pavucontrol; Arch-family systems also receive gsimplecal. It then
automatically makes timestamped backups and replaces the Weyriva-managed user
files. There are no installer choices or configuration prompts. The installer
does not enable or restart greetd or a graphical session. Weyriva has one
intentional default; fork the project if you want to maintain a personalized
variant.

### Checkout maintenance

The `scripts/update.sh` and `scripts/uninstall.sh` helpers are for maintainers
working from a Git checkout. They retain their dry-run and preservation-first
behavior.

Applied user installs record installed paths and SHA-256 digests under `${XDG_STATE_HOME:-$HOME/.local/state}/weyriva`. Pre-existing files remain unowned even when their content is identical. Updates replace only files that still match their recorded digest; locally modified files are preserved. Obsolete owned files are removed only when unchanged, while modified obsolete files are preserved and released from management. Uninstall uses the same ownership rule and never guesses from the current checkout.

The current `packaging/aur/PKGBUILD` is a `weyriva-shell-git` scaffold for
maintainers; it has not been published to AUR. Generate `.SRCINFO` from that
directory with `makepkg --printsrcinfo > .SRCINFO` before an AUR submission.

### Advanced greetd maintenance

The installer intentionally does not manage a login manager. The separate
`scripts/install-greetd.sh` helper is for maintainers repairing an existing
system installation at `/usr/bin/weyriva`; review its template before using it.
It backs up the prior configuration and never enables or restarts greetd.

## Control plane

```bash
weyriva daemon
weyriva status
weyriva diagnose
weyriva diagnose --json
sudo weyriva startup ensure
weyriva ipc call weyriva.info
weyriva ipc call weyriva.methods
weyriva ipc call weyriva.launcher.open
weyriva ipc call weyriva.notifications.dnd
weyriva ipc call weyriva.panel.toggle
weyriva plugin list
weyriva plugin validate examples/plugins/hello.json
weyriva plugin reload
weyriva ipc call weyriva.niri.outputs
weyriva session lock
weyriva wallpaper set ~/Pictures/wallpaper.png
weyriva wallpaper status
weyriva wallpaper reset
```

`weyriva ipc call weyriva.notifications.dnd` toggles mako do-not-disturb (bound to
Mod+N in the packaged niri config); pass `--params '{"enabled": true}'` to set it
explicitly. `weyriva.panel.toggle` hides or shows Waybar (Mod+B) and
`weyriva.panel.reload` reloads its configuration. `weyriva wallpaper set` records a
per-user wallpaper override under XDG config, restarts the wallpaper service when a
user service manager is available, and `reset` returns to the bundled cactus
artwork. The primary session shortcuts are Mod+Space (launcher), Mod+Return
(terminal), Mod+B (panel), Mod+N (do-not-disturb), Mod+Shift+X (lock), and Print
(screenshot). The fixed idle lifecycle locks after five minutes, before sleep, and
when the session lock event is emitted.

The Waybar clock, network, audio, and battery controls are clickable: they open
the calendar, NetworkManager controls, audio controls, and power details. When a
graphical helper is unavailable, Weyriva opens a readable Foot fallback instead.

`weyriva diagnose` is the Niri-only health check for the compositor, session entry,
greetd login path, required desktop commands, user services, and the current Niri
socket. It exits non-zero when a required login component is missing, so it can be
used directly from shell scripts.

`sudo weyriva startup ensure` validates the selected Niri config, installs the
packaged greetd template with a timestamped backup, backs up recognized legacy
Weyriva user units while preserving custom overrides, reloads the user service
manager, and enables greetd. It never restarts greetd or the current graphical
session.

Read [IPC](docs/IPC.md), [plugins](docs/PLUGINS.md), [architecture](docs/ARCHITECTURE.md), and the [roadmap](docs/ROADMAP.md). A concise Chinese introduction is in [docs/README.zh-CN.md](docs/README.zh-CN.md).

## Development

```bash
make check
```

CI runs the same check suite. Optional `shellcheck` and niri configuration validation run when their tools are installed and otherwise report explicit skips.

## License

MIT. See [LICENSE](LICENSE).
