# Testing and acceptance

## Current status

Repository tests cover substantial source structure, CLI/IPC behavior,
installer preservation, Niri bindings, and static QML contracts. The current
visual iteration has focused Python coverage; the complete repository check and
higher evidence levels are still required.

No pointer, keyboard, animation, greeter, lock, deployment, or lifecycle
acceptance on XRY is established by these tests. Record those results only
after the exact reviewed revision is installed and exercised.

## Evidence levels

| Level | Proves |
|---|---|
| Static | syntax, imports, policy, bindings, and action structure |
| Unit | isolated Python state, protocol, and deterministic mappings |
| Integration | repository components cooperate in an isolated environment |
| Runtime | Quickshell renders and the tested input reaches real controls |
| System | Niri, greetd, PAM, systemd, portals, suspend, and packaging work |
| XRY | the exact deployed revision works on the named machine |

A lower level never substitutes for a higher one.

## Repository checks

```bash
make test
make check
```

Expected checks include Python tests, shell syntax/static analysis, Niri
validation, systemd unit verification, Qt 6 QML lint when available, installer
preservation behavior, and forbidden runtime dependency scans.

QML lint proves syntax and available type/import analysis. It does not prove a
surface rendered, accepted input, animated correctly, or completed an action.
Warnings and disabled lint categories must be reported honestly. An unavailable
optional tool is “not run,” not “passed.”

## Source-level visual contract

Tests assert behavior-bearing structure rather than exact pixels, arbitrary
component filenames, or whitespace. They should prove:

- the brand palette contains cactus `#BCD1CA`, ivory `#FAF9F5`, and ink
  `#141413`;
- QML contains no `Gradient` and no Noctalia runtime/config delegation;
- launcher, wallpaper, and settings use the centered surface family;
- control center, calendar, and notifications use the top-right family;
- every enabled control maps to a state change, executable action, dismissal,
  navigation action, or lock request;
- launcher filtering and desktop-entry execution remain wired;
- control center has two columns of real controls;
- calendar has working month navigation and a real date grid;
- notifications are dismissible and expose an empty state;
- wallpaper choices update the path, selection, and related appearance state;
- settings exposes explicit state and disables unavailable options;
- background input mask remains empty;
- greeter and lock retain real authentication semantics;
- reduced-motion cross-fade/static branches exist;
- the native QML IPC handler declares typed `status(): string`.

The status handler must never be used as evidence of secure `WlSessionLock`
recovery.

## Interaction matrix

| Surface | Runtime minimum |
|---|---|
| Bar | every item activates; focus is visible; panels dismiss |
| Launcher | type, filter, navigate, execute, empty result |
| Control center | every enabled control changes state or reports failure |
| Calendar | previous/next month, grid navigation, today/selected state |
| Notifications | dismiss one/all, empty state, DND |
| Wallpaper | visual selection, apply, appearance update, missing asset error |
| Settings | read state, change state, disabled future option |
| Background | pointer passes through to desktop/client windows |
| Greeter | password, failure, success, keyboard-only |
| Lock | all outputs covered, failure, success, resume, crash behavior |

For route transitions, also test source origin, symmetric exit, mid-animation
interruption, final state, and reduced-motion equivalence.

## Accessibility matrix

Test keyboard-only operation, focus order and return, semantic roles/names,
state announcements, light/dark contrast, 200% text scale, reduced motion,
portrait/HiDPI layouts, and multiple outputs. Dynamic wallpaper-derived palette
testing is not applicable until extraction exists.

## Session and security matrix

- cold boot to Weyriva Greeter;
- failed and successful PAM authentication;
- lock from shortcut and native IPC request;
- suspend/resume while locked;
- output hotplug while locked;
- shell crash while unlocked;
- shell crash while locked follows the documented fail-closed path;
- repeated crash loop and TTY recovery;
- no secret material in logs.

Any uncertain lock ownership is a failure. A successful typed QML status reply
proves only that the queried handler responded.

## Packaging and installation

On a clean Arch environment:

- resolve the complete independent runtime package plan first;
- preserve a generic `quickshell` consumer;
- remove only exact conflicting shell/meta packages with non-cascading
  `pacman -R --noconfirm`, never `-Rns`;
- install the generic Quickshell runtime;
- run system preflight before destination writes;
- install the complete shell, greeter, and config trees;
- preserve unmanaged and locally modified files;
- update without stale delegation artifacts;
- uninstall without deleting user-owned data;
- boot and select/start Weyriva.

Repeat the supported subset on other distributions before claiming support.

## XRY record

XRY acceptance records:

- deployed commit and package version;
- install output and resulting file hashes;
- service and journal state;
- login, desktop, lock, suspend, crash, and recovery results;
- screenshots in light and dark modes;
- pointer and keyboard results for every enabled control;
- route motion and reduced-motion results;
- all blocked, failed, or not-run cases.

The machine may be restarted only under explicit operational authorization.

## Completion

A visual iteration is complete only when implementation exists, repository
checks pass, the installed revision matches reviewed source, required runtime
and system matrices pass, XRY evidence is recorded where claimed, and review
accepts residual risk.
