# Session lifecycle

Weyriva presents one product surface across login, lock, and desktop while
retaining the narrow privileged components required for secure Linux session
creation. Noctalia Greeter is the visible login UI. Noctalia v5 is the
in-session shell and lock screen. Niri is the compositor.

This is the required architecture and acceptance contract. Repository files,
installed state, and XRY behavior must all agree before it is called delivered.

## Boot and login chain

```text
display-manager.service
└─ greetd.service                    internal privileged broker
   └─ noctalia-greeter-session       visible Weyriva login layer
      ├─ noctalia-greeter-compositor
      └─ noctalia-greeter
         └─ greetd PAM authentication and session request
            └─ weyriva.desktop
               └─ weyriva session start
                  └─ niri-session
                     └─ niri.service / graphical-session.target
                        ├─ weyriva-shell.service
                        └─ weyriva-ipc.service
```

The precise executable path for `noctalia-greeter-session` comes from the
installed package. Do not hard-code `/usr/local/bin` when the package installs
to `/usr/bin`.

## greetd boundary

greetd remains installed and running. It is not a separately branded or
user-maintained desktop layer; it is the hidden broker for:

- virtual-terminal and seat handoff;
- PAM conversation and credential verification;
- login/session accounting through logind or elogind;
- environment and session creation;
- UID/GID transition to the authenticated user;
- starting the selected Wayland session.

Weyriva does not rewrite this boundary. It does not implement PAM, remove
greetd, or add autologin. Noctalia Greeter intentionally talks to
`GREETD_SOCK`. The official Greeter setup and security model are pinned in the
[installation reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/greeter/index.mdx).

The distribution-provided greetd PAM stack is authoritative. Installation may
validate that the stack exists and integrates with logind/elogind; it must not
overwrite an entire distro PAM file to force compatibility.

## Surface ownership

| Phase | Visible owner |
| --- | --- |
| Login | Noctalia Greeter |
| Desktop | Noctalia v5 inside the Weyriva isolated profile |
| In-session lock | Noctalia v5 lock screen |
| Compositor | Niri |
| PAM/VT/session broker | greetd, hidden from the visual product |

tuigreet is not part of the target chain. Waybar, fuzzel, mako, swaybg,
swaylock, and swayidle must not run beside Noctalia as competing surface
owners.

## systemd user lifecycle

Niri's systemd user integration owns the graphical session. Weyriva user units
belong beneath `niri.service.wants` and are bound to
`graphical-session.target`. Manual `spawn-at-startup` of the same units would
create duplicate starts and is forbidden once the systemd path is active.

`weyriva-shell.service` runs:

```bash
weyriva shell run
```

The CLI replaces itself with Noctalia and supplies the isolated profile roots.
The service must stop with the graphical session and restart only on failure.

## Bounded recovery

A shell crash must not leave a permanently unlocked, half-owned, or
button-less desktop.

Required recovery policy:

- restart the shell on failure;
- allow no more than three attempts within 30 seconds;
- do not use `WatchdogSec` because Noctalia does not emit systemd watchdog
  keepalives;
- after the retry budget is exhausted, invoke a failsafe that exits the Niri
  graphical session;
- return control to the login surface rather than spin indefinitely.

The exact unit directives and failsafe executable are implementation details,
but tests must prove the bound. A process that restarts forever is not a
recovery design.

## Locked-session reconciliation

Lock state belongs to the authenticated session, not only to one Noctalia
process. After a shell restart:

1. query the current logind/elogind session state;
2. if the session is still locked, the replacement shell must immediately
   reacquire `ext-session-lock-v1`;
3. do not show usable desktop content between restart and reacquisition;
4. if secure reconciliation cannot be proved, fail closed by ending the
   graphical session so the system returns to Greeter.

This behavior remains a pending acceptance gate until a real shell crash while
locked has been observed on XRY.

## Suspend and logout

- `session lock-and-suspend` acquires the lock before suspend.
- Resume must return to the locked in-session surface.
- Logout stops graphical-session-bound user services and Niri, then returns to
  Greeter.
- Reboot and shutdown remain privileged system actions exposed through
  authenticated session controls; button feedback and failure reporting are
  required.

## Greeter appearance

Noctalia can sync the resolved desktop palette, mode, wallpaper, font, corner
radius, session actions, output layout, and transforms:

```bash
weyriva shell msg greeter-sync
```

Precedence is:

```text
/var/lib/noctalia-greeter/greeter.toml
> /var/lib/noctalia-greeter/sync.toml
> Greeter built-in defaults
```

Declarative `greeter.toml` is never overwritten by Sync. A complete
`[appearance.palette]` there overrides the synced palette. The source behavior
is pinned in
[`appearance_config.cpp`](https://github.com/noctalia-dev/noctalia-greeter/blob/d6275cbcb5b9acae2348bed16e358aa6c2cf8188/src/greeter/appearance_config.cpp#L101-L128)
and the full key set in the pinned
[Greeter configuration](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/greeter/configuration.mdx).

## Recovery console

TTY2 is the permanent recovery path. Installation and session configuration
must not consume or disable it.

Typical recovery checks from TTY2:

```bash
systemctl status greetd.service --no-pager
journalctl -u greetd.service -b --no-pager
systemctl --user status niri.service weyriva-shell.service weyriva-ipc.service --no-pager
journalctl --user -u weyriva-shell.service -b --no-pager
weyriva diagnose
```

User-service commands require the affected user's environment/session bus; if
that is unavailable from the TTY, inspect the system journal and user journal
files rather than guessing.

## Required logs

| Layer | Primary evidence |
| --- | --- |
| greetd and Greeter | `journalctl -u greetd.service -b` |
| Niri user session | `systemctl --user status niri.service` and user journal |
| Noctalia shell | `journalctl --user -u weyriva-shell.service -b` |
| Weyriva IPC | `journalctl --user -u weyriva-ipc.service -b` |
| Effective configuration | `weyriva shell config export full` |
| Runtime shell status | `weyriva shell msg status` |

Greeter normally logs through syslog/journald under the greetd unit. Do not
invent a dedicated Greeter log file unless `NOCTALIA_GREETER_LOG` explicitly
configures one.

## Acceptance

The lifecycle is complete only after testing:

- cold boot to Greeter;
- valid and invalid PAM authentication;
- user/session selection without autologin;
- desktop start and clean logout;
- lock by shortcut, IPC, idle, and suspend;
- shell crash unlocked and locked;
- bounded crash-loop failsafe;
- TTY2 recovery;
- multi-monitor, portrait, and HiDPI Greeter;
- appearance sync and declarative override precedence;
- journal evidence showing no duplicate surface owners.
