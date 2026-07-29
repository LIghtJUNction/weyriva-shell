# Session lifecycle

Weyriva owns one continuous visual system across login, desktop, and lock.
greetd remains the internal PAM/VT/session broker; Niri remains the compositor.
Noctalia is not part of the target process chain.

The repository wiring is under migration. This document defines required
behavior and does not claim that the greeter, lock, or recovery flow has passed
real-machine acceptance.

## Target boot chain

```text
systemd
└─ greetd.service
   └─ Weyriva Greeter
      └─ PAM-authenticated session request over GREETD_SOCK
         └─ niri-session
            └─ Weyriva Shell
               ├─ desktop surfaces
               ├─ native IPC/plugin hosts
               └─ authenticated lock surface
```

greetd must not become a second branded UI. The exact Weyriva Greeter command
and service template remain in progress and must be validated against the live
package before installation.

## greetd boundary

greetd is authoritative for:

- VT and seat ownership;
- PAM conversation and account/session modules;
- switching from the greeter account to the authenticated user;
- creating the configured Wayland session.

Weyriva is authoritative for what the user sees and how progress, errors, and
recovery are presented. It must not:

- implement password verification itself;
- replace the distribution PAM stack;
- enable autologin;
- expose secret text in logs;
- remove the TTY recovery path.

## Greeter

The planned Weyriva Greeter is a minimal Quickshell surface. It must provide:

- account selection without leaking sensitive account metadata;
- password input with correct masking, focus, and keyboard submission;
- explicit authentication progress and recoverable failure;
- session start only after greetd confirms authentication;
- HiDPI, portrait, multi-monitor, keyboard-only, and screen-reader behavior;
- the same tokens and original editorial-art grammar as the desktop.

It must remain responsive while PAM work is pending and must not pretend an
authentication request succeeded before greetd responds.

## Desktop startup

After authentication, Niri starts the Weyriva user session. The shell must:

1. load deterministic defaults and user state;
2. establish platform adapters;
3. create exactly one owner for each visible surface;
4. publish readiness only when the primary interaction surface is usable;
5. start plugin hosts after the shell's compatibility version is known.

A successful process spawn is not shell readiness.

## Lock

The in-session lock is rendered by Weyriva and secured through
`ext-session-lock-v1`. The visual surface and secure protocol acquisition are
separate states; visual polish must never delay security.

Required sequence:

1. request lock;
2. acquire the Wayland session-lock protocol;
3. cover every output;
4. only then report secure lock;
5. authenticate through the distribution-supported path;
6. release the lock only after confirmed success.

The shell must handle output add/remove while locked and must not reveal
desktop content during transitions.

## Suspend and logout

- lock-and-suspend acquires a secure lock before suspend.
- resume returns to the authenticated lock surface.
- logout stops user surfaces and returns control to greetd.
- shutdown/reboot show progress and failure; they are never dead decorative
  buttons.

## Crash and recovery

Recovery is bounded. An unlocked shell may be restarted with backoff. A
replacement process cannot safely inherit or reacquire an abandoned
`ext-session-lock-v1` lock, so startup reconciliation succeeds only when
logind reports `LockedHint=no`.

`LockedHint=yes`, an unknown value, or any query error makes
`ExecStartPost=weyriva shell reconcile-lock` fail. The bounded systemd restart
policy is then exhausted and `weyriva-session-failsafe.service` ends the Niri
session so greetd can present a fresh authenticated login. Weyriva never keeps
a half-rendered desktop alive behind uncertain lock ownership.

Crash-loop limits, backoff, and the final action must be observable in logs.

## Recovery console

TTY2 remains the documented escape hatch:

```text
Ctrl+Alt+F2
```

From there, the user can inspect:

```bash
systemctl status greetd.service --no-pager
journalctl -u greetd.service -b --no-pager
systemctl --user status weyriva-shell.service --no-pager
journalctl --user -u weyriva-shell.service -b --no-pager
```

Unit names are verified only when the independent runtime packaging lands.

## Status

| Area | Status |
|---|---|
| greetd/PAM boundary design | Implemented in repository policy; live validation pending |
| Independent Weyriva Greeter | Source implemented; system acceptance pending |
| Native desktop startup | Source implemented; runtime acceptance pending |
| Secure integrated lock | Source implemented; security/system acceptance pending |
| Suspend/resume/logout | In progress |
| Crash-loop and locked recovery | Fail-closed repository wiring implemented; system acceptance pending |
| Cold-boot and XRY acceptance | Planned |

The lifecycle is verified only after the matrix in [Testing](TESTING.md) passes.
