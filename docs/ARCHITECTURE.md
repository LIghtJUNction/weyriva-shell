# Architecture

Weyriva is deliberately composed from established Wayland components. In a system/AUR install, greetd/tuigreet selects the system `weyriva.desktop` entry, whose absolute `/usr/bin/weyriva session start` replaces itself with `niri-session`. niri's session wrapper publishes the Wayland environment and activates `graphical-session.target`. The Weyriva niri config then starts four user services; each has `PartOf`, `After`, and `Requisite` relationships with that standard target, so it stops with the graphical session. A source user install is tested directly from a TTY and is not advertised as display-manager discoverable.

The control plane is one Python 3 standard-library executable. It does not manage windows itself and has no elevated component. The daemon binds a per-user socket beneath `XDG_RUNTIME_DIR`, dispatches a small reserved `weyriva.*` method set, and then dispatches validated plugin methods. Desktop actions use fixed argument arrays and never a shell.

Configuration is split by upstream component under `config/`. User installation mirrors those trees into XDG homes. AUR packaging puts niri's fallback at `/etc/niri/config.kdl`, other defaults under `/etc/xdg`, shared assets under `/usr/share`, and units under `/usr/lib/systemd/user`. greetd remains a separately reviewed system action and its config is packaged only as a template.

## Invariants

- The daemon and plugins run with the logged-in user's authority only.
- IPC is local, bounded, newline framed, and versioned.
- `weyriva.*` is reserved; plugins cannot replace built-ins.
- Plugin manifests are explicit and duplicate methods are rejected.
- Installation is a dry run by default and never silently overwrites a conflict.
- Identical pre-existing files are not adopted; managed updates and removals require the recorded installed digest to match.
- Shell components remain replaceable instead of being embedded in a monolith.
