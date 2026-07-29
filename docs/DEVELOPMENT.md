# Development

Weyriva is migrating from a Noctalia-delegating scaffold to an independent
Quickshell 0.3 / QtQuick shell. New work must advance the independent
architecture and must not deepen or document the transitional delegation as a
stable product API.

## Repository map

Current repository-owned areas:

| Path | Role |
|---|---|
| `bin/weyriva` | CLI, diagnostics, local JSON IPC, and migration helpers |
| `shell/` | native Quickshell desktop and lock source; integration in progress |
| `greeter/` | native Quickshell greetd client source; system acceptance pending |
| `config/weyriva/` | project-owned deterministic defaults |
| `config/niri/` | Niri session policy and bindings |
| `config/greetd/` | privileged broker template; not the visible greeter |
| `systemd/` | current user-session service scaffolding |
| `scripts/`, `install.sh` | zero-choice installation and update scaffolding |
| `tests/` | repository-level regression tests |
| `docs/` | architecture, behavior, compatibility, and acceptance contracts |

The earlier `config/noctalia/` runtime profile has been removed from the live
tree and is a forbidden legacy path for new work. The native source is still a
migration-stage implementation: its presence does not establish complete
surface or system behavior.

## Architecture rules

- Weyriva owns every visible surface.
- greetd owns PAM/VT/session privilege only.
- Niri owns compositing and workspace policy.
- platform adapters do not manipulate presentation components directly.
- state and actions are typed and testable.
- one visible surface has one lifecycle owner.
- Noctalia is public reference material only.
- plugins are trusted code but still receive bounded, isolated runtimes where
  the ABI permits.

## Normal edit loop

```bash
make test
make check
```

Use the narrowest relevant test while iterating, then the full repository
checks before handoff. Quickshell syntax, QML imports, runtime rendering, and
Wayland input require their own validation once the native tree is present.

For a surface change:

1. identify state, actions, focus ownership, and failure states;
2. define the intended end state and shared component use;
3. implement the smallest coherent slice;
4. verify pointer and keyboard behavior;
5. verify loading, empty, disabled, error, and completion states;
6. capture runtime evidence on the target environment.

## Clean-room compatibility

Noctalia-compatible plugin work uses public documentation, public manifests,
installed tool output, and self-authored behavioral fixtures. Do not copy
Noctalia shell implementation source into Weyriva.

Separate:

- public ABI facts;
- Weyriva implementation choices;
- behavior that still requires a black-box probe.

See [Plugin compatibility](plugins/compatibility-contract.md).

## Security boundaries

- do not rewrite or replace the distribution PAM policy;
- do not enable autologin;
- do not log passwords, tokens, clipboard secrets, or plugin credentials;
- use argument arrays or typed protocol messages, never interpolated shell
  strings;
- acquire secure lock before reporting locked;
- fail closed if lock ownership is uncertain;
- do not broaden privileges for convenience.

## Documentation discipline

Every material claim uses one of:

- **Implemented** — source plus local check;
- **In progress** — architecture fixed, implementation incomplete;
- **Planned** — accepted but not implemented;
- **Verified** — executed in the environment named by the claim.

When code and docs disagree, report the mismatch; do not silently describe the
target as current behavior.

## Handoff evidence

Report:

1. exact changed files;
2. validation command and status;
3. runtime/tool versions;
4. whether login, lock, suspend, packaging, and plugins were exercised;
5. whether XRY pointer, keyboard, screenshot, and log evidence exists;
6. residual gaps and temporary migration artifacts.

See [Architecture](ARCHITECTURE.md), [Testing](TESTING.md), and
[Roadmap](ROADMAP.md).
