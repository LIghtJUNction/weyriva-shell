# Development

This is the entry point for contributors working on Weyriva itself. Weyriva is
an opinionated distribution around the upstream Noctalia v5 shell, not a second
desktop-shell implementation. Changes must preserve one owner for every visible
surface and one deterministic installation profile.

The repository is still under acceptance. Passing local checks does not prove
that login, lock recovery, plugin rendering, or the complete desktop has passed
on XRY hardware. Those gates are tracked in
[Noctalia parity](NOCTALIA_PARITY.md).

## Repository map

| Path | Owner and purpose |
| --- | --- |
| `bin/weyriva` | Weyriva CLI, isolated Noctalia delegation, diagnostics, and the legacy local IPC lane |
| `config/noctalia/` | Fixed declarative shell profile and offline fallback palette |
| `config/niri/` | Niri compositor defaults and user-facing key bindings |
| `config/greetd/` | System template for the hidden login broker |
| `assets/` | Project-owned wallpapers and desktop/session assets |
| `systemd/` | Graphical-session-bound user services and recovery units |
| `scripts/` | Installation, update, validation, and maintainer helpers |
| `packaging/aur/` | Arch/AUR packaging source |
| `tests/` | Standard-library unit and contract tests |
| `docs/` | Architecture, design, protocol, lifecycle, and acceptance contracts |

Noctalia owns the bar, tray, launcher, dock, control center, notifications,
wallpaper, OSD, settings, lock screen, idle policy, screenshots, desktop
widgets, and current v5 plugin rendering. Weyriva owns distribution defaults,
Niri integration, login/session wiring, install/update behavior, diagnostics,
and the reserved `weyriva.*` namespace. Do not add a parallel Waybar, mako,
wallpaper host, lock host, or plugin UI runtime.

## Isolated profile

`weyriva shell` starts Noctalia with three Weyriva-specific roots:

```text
NOCTALIA_CONFIG_HOME = $XDG_CONFIG_HOME/weyriva
NOCTALIA_STATE_HOME  = $XDG_STATE_HOME/weyriva
NOCTALIA_DATA_HOME   = $XDG_DATA_HOME/weyriva
```

Noctalia appends `/noctalia`, so the normal configuration is
`$XDG_CONFIG_HOME/weyriva/noctalia/config.toml` and GUI/IPC overrides are in
`$XDG_STATE_HOME/weyriva/noctalia/settings.toml`. The latter loads last and can
override the packaged profile. The upstream merge and export rules are pinned
in the [Noctalia configuration reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/configuration/index.mdx).

Never diagnose Weyriva by inspecting only a standalone
`~/.config/noctalia/` profile.

## Normal edit loop

Run focused tests while editing:

```bash
make test
```

Run the repository check before handoff:

```bash
make check
```

The check includes Python tests, shell syntax and static policy checks, Niri
validation when Niri is installed, and an isolated `noctalia config validate`
when Noctalia is installed. A skipped optional runtime is not a pass for its
acceptance gate.

Inspect the effective Weyriva profile without editing app-managed state:

```bash
weyriva shell config validate
weyriva shell config export full
```

For a running development session, Noctalia watches both configuration layers.
An explicit reload is also available:

```bash
weyriva shell msg config-reload
weyriva shell msg status
```

Do not delete `settings.toml` as a routine debugging step. First inspect it and
the exported effective configuration; it is user-owned runtime state.

## Safe boundaries

- Keep package, installer, systemd, session, and documentation claims aligned.
- Do not rewrite PAM. Weyriva relies on the distribution's greetd PAM stack.
- Do not add autologin or remove the TTY recovery path.
- Do not invoke delegated plugin or shell commands through a shell string.
- Do not claim an API range broader than the installed Noctalia engine reports.
- Do not call a source-inspection result a live desktop pass.
- Preserve unrelated worktree changes and user configuration.
- Treat all native and legacy plugins as trusted user code, not sandboxed code.

## Design and implementation references

- [Design system](DESIGN_SYSTEM.md)
- [Theming](THEMING.md)
- [Motion](MOTION.md)
- [Accessibility](ACCESSIBILITY.md)
- [Session lifecycle](SESSION_LIFECYCLE.md)
- [Developer experience](DEVELOPER_EXPERIENCE.md)
- [IPC](IPC.md)
- [Plugins](PLUGINS.md)
- [Testing](TESTING.md)

## Handoff evidence

A development handoff reports:

1. files intentionally changed;
2. checks that passed, failed, were skipped, or were not run;
3. effective Noctalia version and profile roots when runtime behavior matters;
4. whether login, lock, suspend, or systemd behavior was exercised;
5. whether XRY pointer, keyboard, visual, and screenshot evidence exists;
6. residual risks and the exact next required action.

Do not stage or commit `SWAP.md`. It is temporary Planner continuity state.
