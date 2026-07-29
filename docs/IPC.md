# IPC and control planes

Weyriva owns its IPC. The target native Quickshell runtime must not forward
shell commands to Noctalia. Transitional bridges in the current scaffold are
migration artifacts and are not stable product contracts.

## Versioned local JSON lane

The repository currently implements a local `weyriva.*` JSON control lane for
diagnostics, Niri queries, and bounded legacy executable plugins.

Representative calls:

```bash
weyriva ipc call weyriva.ping
weyriva ipc call weyriva.info
weyriva ipc call weyriva.methods
weyriva ipc call weyriva.niri.outputs
weyriva ipc call weyriva.niri.windows
weyriva ipc call weyriva.plugin.list
```

The protocol is versioned. Requests and responses are bounded, malformed
frames are rejected, plugin output is size-limited, and plugin execution has a
timeout. The socket is user-local and must not become a network listener.

This lane is **implemented locally**. Its installed service and full
integration with the independent shell are not yet verified.

## Native shell lane

The planned native lane controls Weyriva surfaces and state:

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

- v5 Luau entries receive `onIpc(event, payload)` through the Weyriva host;
- v4 QML plugins register `IpcHandler` targets in the isolated compatibility
  host;
- legacy executable plugins remain on the bounded JSON lane.

No profile is compatible until IPC arguments, targeting, lifecycle, errors, and
return behavior pass self-authored fixtures. See
[Conformance fixtures](plugins/conformance-fixtures.md).

## Security

- Never construct a shell command by concatenating untrusted text.
- Validate method names, payload shape, size, and target.
- Keep user and plugin namespaces separate.
- Do not expose passwords, tokens, clipboard data, or private notification
  content in diagnostics.
- Treat compatible Luau, QML, and executable plugins as trusted user code, not
  as a sandbox.
- Privileged operations use an existing scoped broker and return explicit
  authorization errors.

## Diagnosis

A healthy JSON socket proves only that the daemon answered. It does not prove a
button, calendar, lock, greeter, native panel, or plugin UI works. IPC
acceptance always pairs the response with visible or stateful evidence.
