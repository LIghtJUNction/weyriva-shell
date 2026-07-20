# IPC protocol

Protocol version 1 uses newline-delimited UTF-8 JSON over:

```text
$XDG_RUNTIME_DIR/weyriva/weyriva.sock
```

The daemon creates the parent directory as mode `0700` and socket as `0600`. An exclusive lock prevents a second daemon from replacing a live socket. Each connection has a three-second read timeout, carries one newline-terminated request of at most 64 KiB, and receives one response. At most 16 handlers run concurrently; excess connections are closed. Clients use a three-second default timeout.

Request:

```json
{"protocol":1,"id":42,"method":"weyriva.ping","params":{}}
```

Success and error responses:

```json
{"id":42,"result":{"pong":true,"protocol":1}}
{"id":42,"error":{"code":"method_not_found","message":"unknown method: example.missing"}}
```

`id` is any scalar JSON value. `params` is forwarded as one JSON value to plugins. Built-ins are `weyriva.ping`, `weyriva.info`, `weyriva.plugin.list`, `weyriva.launcher.open`, and `weyriva.notifications.dismiss_all`. Action methods accept fixed executable arguments: callers cannot supply commands.

This is a local convenience API, not an authentication boundary. Unix permissions separate users; any process already running as the same user can call it.
