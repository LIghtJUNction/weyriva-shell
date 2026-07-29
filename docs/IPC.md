# IPC and control planes

Weyriva owns its IPC. The target native Quickshell runtime must not forward
shell commands to Noctalia. Transitional bridges in the current scaffold are
migration artifacts and are not stable product contracts.

## Rust control lane

`crates/weyriva/` is the source for the production CLI and resident
daemon. Its user-local Unix socket carries the versioned `weyriva.*` JSON
protocol for shell startup/session control and **Weyriva Plugins**. Startup,
shell, session, diagnosis, plugin lifecycle, the installer, systemd units, and
the local AUR recipe use the Rust command surface. `/usr/bin/weyriva` is the
installed path produced by those local packaging paths; publication and
target-machine deployment remain separate claims.

The locally implemented plugin CLI shape uses explicit canonical IDs:

```bash
weyriva plugin source list
weyriva plugin install noctalia/kaomoji
weyriva plugin enable noctalia/kaomoji
weyriva plugin status noctalia/kaomoji
weyriva plugin reload noctalia/kaomoji
weyriva plugin disable noctalia/kaomoji
weyriva plugin uninstall noctalia/kaomoji
```

The protocol is versioned. Requests and responses are bounded, malformed
frames are rejected, output is size-limited, and plugin execution has bounded
time and resources. The socket is user-local and must not become a network
listener.

Weyriva Plugins lifecycle methods are versioned under `weyriva.plugin.v1.*`: source
list/add/remove, install, status, enable, disable, reload, uninstall, query, and
activate. The daemon owns persistent host processes; the QML launcher calls the
CLI with argument arrays and renders validated result objects.

The daemon-to-host protocol is newline-delimited JSON identified by
`weyriva-luau-host/1`. Messages, result counts, strings, VM memory, callback
time, actions, and filesystem reads are bounded. This API 3 launcher slice is
implemented and tested locally, including provider categories transported to
QML, and is wired into local installation/package metadata. It has not been
verified through a clean package install or this revision's XRY deployment.
`noctalia-v5-luau/1` names the upstream compatibility profile, not a Weyriva
product version.

## Native shell lane

The native lane incrementally controls Weyriva surfaces and state:

- status and readiness;
- panel open, close, and toggle;
- notifications and Do Not Disturb;
- theme and wallpaper;
- screenshot requests;
- session lock, lock-and-suspend, logout, reboot, and shutdown;
- plugin sources, lifecycle, and entry IPC.

The final method names and payload schemas are frozen only when implemented,
tested, and published with a protocol version. Documentation must not preserve
old Noctalia command names merely for convenience.

## Namespace rules

- `weyriva.*` is reserved for project-owned methods.
- plugin methods use a distinct, validated namespace.
- method names are stable within a protocol major version.
- unknown fields are either explicitly ignored or rejected; behavior is
  documented per method.
- errors contain a stable code and a human-readable message.

## Asynchronous operations

Operations such as wallpaper application, screenshots, plugin updates,
authentication-adjacent actions, and session transitions cannot be represented
honestly by “request accepted” alone.

The native protocol must distinguish:

1. request rejected;
2. request accepted with operation ID;
3. progress where meaningful;
4. final success or failure;
5. cancellation when supported.

Visible controls observe the same operation state as IPC clients.

## Plugin IPC

Plugin IPC depends on the compatibility profile:

- `noctalia-v5-luau/1` entries receive `onIpc(event, payload)` through the
  Rust `weyriva-luau-host`;
- v4 QML plugins register `IpcHandler` targets in the isolated compatibility
  host once that planned lane exists.

Python is test tooling only. It is not a documented production protocol,
runtime, or plugin-authoring surface.

No profile is compatible until IPC arguments, targeting, lifecycle, errors, and
return behavior pass self-authored fixtures. See
[Conformance fixtures](plugins/conformance-fixtures.md).

## Security

- Never construct a shell command by concatenating untrusted text.
- Validate method names, payload shape, size, and target.
- Keep user and plugin namespaces separate.
- Do not expose passwords, tokens, clipboard data, or private notification
  content in diagnostics.
- Treat compatible Luau and QML plugins as trusted user code, not as a sandbox.
- Privileged operations use an existing scoped broker and return explicit
  authorization errors.

## Diagnosis

A healthy JSON socket proves only that the daemon answered. It does not prove a
button, calendar, lock, greeter, native panel, or plugin UI works. IPC
acceptance always pairs the response with visible or stateful evidence.
