# Plugins

Weyriva plugins are trusted local executables, not sandboxed extensions. Installing one grants it the same file, network, and process access as any other program run by your user. Review its manifest and executable before installation.

The daemon loads `*.json` manifests, in deterministic order, from:

1. `$XDG_CONFIG_HOME/weyriva/plugins`
2. `$XDG_DATA_HOME/weyriva/plugins`
3. each `$XDG_DATA_DIRS/weyriva/plugins`

Example manifest:

```json
{
  "id": "my-plugin",
  "version": 1,
  "methods": {
    "my-plugin.greet": {
      "argv": ["./greet.py"],
      "timeout": 2
    }
  }
}
```

Manifest `version` must be integer `1`. IDs must match `[a-z][a-z0-9-]{1,63}`, and every method must begin with that exact ID plus a dot. The `weyriva.*` namespace is reserved. Duplicate names, invalid manifests, empty commands, and timeouts outside 0.1–30 seconds are rejected. A command containing a relative path is resolved relative to its manifest; a bare command is resolved through `PATH`.

For each call, the daemon writes `params` as JSON to stdin. The plugin must exit zero and write exactly one JSON value to stdout. Stderr is reported on failure. Stdout and stderr are read incrementally with 1 MiB and 64 KiB limits; timeout or overflow terminates the plugin process group. No shell evaluates the command.

To try the harmless example, copy both `examples/plugins/hello.json` and its `hello/` directory into your user plugin directory, restart the daemon, then call:

```bash
weyriva ipc call example.hello --params '{"name":"Weyriva"}'
```
