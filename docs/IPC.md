# IPC and control planes

Weyriva exposes two different protocols. They share a CLI brand but not a
socket, namespace, runtime, or plugin model.

## Native Noctalia lane

`weyriva shell msg` delegates a fixed argument array to Noctalia with the
isolated Weyriva profile. It controls the actual visible shell.

General and surfaces:

```bash
weyriva shell msg status
weyriva shell msg config-reload
weyriva shell msg settings-open
weyriva shell msg settings-toggle
weyriva shell msg bar-toggle
weyriva shell msg panel-toggle launcher
weyriva shell msg panel-toggle control-center
weyriva shell msg panel-toggle clipboard
weyriva shell msg panel-toggle wallpaper
weyriva shell msg panel-toggle session
weyriva shell msg session lock
weyriva shell msg session lock-and-suspend
```

Theme:

```bash
weyriva shell msg theme-mode-get
weyriva shell msg theme-mode-toggle
weyriva shell msg theme-mode-set dark
weyriva shell msg theme-mode-set light
weyriva shell msg theme-mode-set auto
weyriva shell msg color-scheme-get
weyriva shell msg color-scheme-set wallpaper soft
weyriva shell msg color-scheme-set custom Weyriva
weyriva shell msg color-scheme-set builtin Noctalia
weyriva shell msg templates-apply
```

Wallpaper:

```bash
weyriva shell msg wallpaper-get
weyriva shell msg wallpaper-get DP-1
weyriva shell msg wallpaper-set /absolute/path/image.png
weyriva shell msg wallpaper-set DP-1 /absolute/path/image.png
weyriva shell msg wallpaper-random
weyriva shell msg wallpaper-random DP-1
weyriva shell msg wallpaper-next
weyriva shell msg wallpaper-previous
```

Wallpaper paths accept an existing image path or `color:#RRGGBB` /
`color:#RRGGBBAA`. Set/mode/color commands persist through Noctalia's isolated
`settings.toml`; record and restore state during tests.

Notifications, clipboard, and screenshots:

```bash
weyriva shell msg notification-dnd-status
weyriva shell msg notification-dnd-toggle
weyriva shell msg notification-dnd-set on
weyriva shell msg notification-dnd-set off
weyriva shell msg clipboard-text
weyriva shell msg screenshot-region
weyriva shell msg screenshot-fullscreen
weyriva shell msg screenshot-fullscreen pick
weyriva shell msg screenshot-fullscreen DP-1
weyriva shell msg screenshot-fullscreen all
```

Greeter:

```bash
weyriva shell msg greeter-sync
```

Noctalia registers `greeter-sync` only when Noctalia Greeter, the appearance
apply helper, and a usable privilege escalation path are available. It stages
palette, wallpaper, font, session actions, and output metadata; it does not
reimplement login or PAM.

The authoritative native commands are pinned in:

- [shell IPC](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/ipc/shell.mdx);
- [surfaces IPC](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/ipc/surfaces.mdx);
- [media and UI IPC](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/ipc/media-and-ui.mdx).

Noctalia does not expose animation speed/enabled, high contrast, or UI scale as
native IPC commands. Those settings use configuration/Settings and hot reload.

## Weyriva local JSON lane

The legacy/local protocol uses newline-delimited UTF-8 JSON over:

```text
$XDG_RUNTIME_DIR/weyriva/weyriva.sock
```

The daemon creates the directory as mode `0700` and socket as `0600`. One
newline-terminated request is limited to 64 KiB and receives one response.
Connections and plugin execution are bounded by the implementation's time and
output limits.

Request:

```json
{"protocol":1,"id":42,"method":"weyriva.ping","params":{}}
```

Responses:

```json
{"id":42,"result":{"pong":true,"protocol":1}}
{"id":42,"error":{"code":"method_not_found","message":"unknown method: example.missing"}}
```

`id` is a scalar JSON value. Built-in methods include:

```text
weyriva.ping
weyriva.info
weyriva.methods
weyriva.plugin.list
weyriva.plugin.reload
weyriva.niri.outputs
weyriva.niri.windows
```

Additional compatibility methods may remain while legacy clients migrate, but
they do not own Noctalia's bar, notifications, wallpaper, or lock surfaces.
Use the native lane for those actions.

Example:

```bash
weyriva ipc call weyriva.info
weyriva ipc call weyriva.methods
weyriva ipc call weyriva.niri.outputs
```

`weyriva.plugin.reload` rescans only legacy executable manifests. It does not
reload native Noctalia v5 plugins.

## Security boundary

Neither protocol is a sandbox. Native plugins and legacy executables run with
the logged-in user's authority. The local socket relies on Unix permissions;
any process already running as that user can call it.

Delegation uses argument arrays. User-provided IPC payloads must never become a
shell command, privilege prefix, or arbitrary executable path in built-in
actions.

## Diagnosis

If a visible control fails, determine the lane first:

```bash
weyriva shell msg status
weyriva status
weyriva ipc call weyriva.info
```

A healthy `weyriva.*` socket does not prove that Noctalia IPC or a visible
button works. Conversely, a healthy Noctalia shell does not prove that a legacy
executable plugin is available.
