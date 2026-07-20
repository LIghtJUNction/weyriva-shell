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
- original SVG wallpaper and graphical-session-bound systemd user services

## Try it safely

The installer is a dry run unless `--apply` is given. It preserves conflicts; `--force` makes timestamped backups before replacement.

```bash
./scripts/check.sh
./scripts/install.sh
./scripts/install.sh --apply
```

The default user install deliberately does not copy a Wayland session desktop entry and is not registered with a system display manager. Test it from a TTY with `~/.local/bin/weyriva session start`; Weyriva adds its own executable directory to the niri session's `PATH` so its startup commands remain reachable. The session expects `niri`, `waybar`, `fuzzel`, `mako`, `swaybg`, `foot`, Noto Sans, and Nerd Symbols fonts. Selecting **Weyriva Shell** in greetd requires the future system/AUR installation, which owns `/usr/bin/weyriva` and `/usr/share/wayland-sessions/weyriva.desktop`.

### Updates and removal

```bash
./scripts/update.sh          # dry-run for a Git checkout
./scripts/update.sh --apply
./scripts/uninstall.sh       # dry-run; modified files are preserved
./scripts/uninstall.sh --apply
```

Applied user installs record installed paths and SHA-256 digests under `${XDG_STATE_HOME:-$HOME/.local/state}/weyriva`. Pre-existing files remain unowned even when their content is identical. Updates replace only files that still match their recorded digest; locally modified files are preserved. Obsolete owned files are removed only when unchanged, while modified obsolete files are preserved and released from management. Uninstall uses the same ownership rule and never guesses from the current checkout.

For the future AUR package, update through your AUR helper. The current `packaging/aur/PKGBUILD` is a `weyriva-shell-git` scaffold; it has not been published to AUR. Generate `.SRCINFO` from that directory with `makepkg --printsrcinfo > .SRCINFO` before an AUR submission.

### greetd

Installing or changing a login manager is system-wide and can lock you out. Review `config/greetd/config.toml` first. The separate helper requires an existing system installation at `/usr/bin/weyriva`, root, and explicit application. It backs up an existing config and never enables or restarts greetd. Its session lookup remains `/usr/share/wayland-sessions`; per-user desktop entries are not claimed to work with greetd.

```bash
./scripts/install-greetd.sh
sudo ./scripts/install-greetd.sh --apply
```

## Control plane

```bash
weyriva daemon
weyriva status
weyriva diagnose
weyriva diagnose --json
sudo weyriva startup ensure
weyriva ipc call weyriva.info
weyriva ipc call weyriva.launcher.open
weyriva plugin list
```

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
