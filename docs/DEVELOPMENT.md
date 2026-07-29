# Development

Weyriva is migrating from a Noctalia-delegating scaffold to an independent
Quickshell 0.3 / QtQuick shell. New work must advance the independent
architecture and must not deepen or document the transitional delegation as a
stable product API.

## Repository map

Current repository-owned areas:

| Path | Role |
|---|---|
| `crates/weyriva/` | Rust CLI/daemon, startup/session/diagnose, shell control, IPC, and plugin lifecycle |
| `crates/weyriva-luau-host/` | locally tested bounded Rust Luau host for the API 3 launcher-provider slice |
| `shell/` | native Quickshell desktop and lock source; integration in progress |
| `greeter/` | native Quickshell greetd client source; system acceptance pending |
| `config/weyriva/` | project-owned deterministic defaults |
| `config/niri/` | Niri session policy and bindings |
| `config/greetd/` | privileged broker template; not the visible greeter |
| `systemd/` | bounded user-session units invoking the Rust control plane |
| `scripts/`, `install.sh` | zero-choice installation/update paths building and installing both Rust binaries |
| `tests/` | repository-level regression tests; Python is test tooling only |
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
- Rust owns production CLI/daemon/control and Luau execution.
- QML/Quickshell owns presentation only and never loads plugin source.
- Python is neither a production dependency nor a plugin language.

## Normal edit loop

```bash
make test
make check
./scripts/check.sh
```

The Make targets run locked workspace Rust tests, formatting, and Clippy.
`scripts/check.sh` is the broader repository gate for Python-based policy and
installer tests, shell/config checks, Rust checks, and optional Niri, systemd,
and QML tools. Python remains test tooling only. Runtime rendering and Wayland
input require their own validation.

For either Rust crate, the focused contributor gate is:

```bash
cargo fmt --all -- --check
cargo check --locked -p weyriva -p weyriva-luau-host
cargo clippy --locked -p weyriva -p weyriva-luau-host --all-targets -- -D warnings
cargo test --locked -p weyriva -p weyriva-luau-host
```

These commands establish source-level Rust evidence only. They do not establish
packaging, installed service, UI interaction, or XRY acceptance.

The release build consumed by every local install/package path is:

```bash
cargo build --release --locked -p weyriva -p weyriva-luau-host
```

For a complete system install, use the zero-choice `./install.sh`; it resolves
distribution packages, builds both binaries, performs system preflight, and
then installs. `scripts/install.sh --apply` is the preservation-first
user-local path for an already built checkout. `packaging/aur/PKGBUILD` builds
and packages the same two binaries, but the recipe being present locally is
not an AUR publication claim.

The Rust control plane and API 3 Luau host have local test and package-wiring
evidence. Do not turn that into claims that the AUR package is published, the
package has passed a clean install, or the all-Rust revision is deployed on
XRY.

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

Use **Weyriva Plugins** for the product. Use
`noctalia-v5-luau/1` only for the exact upstream compatibility profile; never
present `v5` as a Weyriva product version.

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
