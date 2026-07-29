#[cfg(unix)]
use std::fs;

use serde_json::json;

use crate::support::HostProcess;
#[cfg(unix)]
use crate::support::{TempDir, canonical, fixture_dir};

#[test]
fn unsupported_json_method_uses_the_json_namespace_diagnostic() {
    let host = HostProcess::start("unsupported-json-api.luau", json!({}));

    let fatal = host.read();

    assert_eq!(
        (
            fatal["error"]["code"].as_str(),
            fatal["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("noctalia.json.parse"))
        ),
        (Some("unsupported_api"), true)
    );
}

#[test]
fn traversal_read_is_rejected_during_plugin_load() {
    let host = HostProcess::start("read-path.luau", json!({"path": "../outside.txt"}));

    let fatal = host.read();

    assert_eq!(fatal["error"]["code"], "file_access");
}

#[cfg(unix)]
#[test]
fn symlink_read_that_escapes_plugin_root_is_rejected() {
    use std::os::unix::fs::symlink;

    let plugin = TempDir::new("symlink-plugin");
    let outside = TempDir::new("symlink-outside");
    fs::copy(
        fixture_dir().join("read-path.luau"),
        plugin.path.join("read-path.luau"),
    )
    .expect("read fixture should copy");
    fs::write(outside.path.join("secret.txt"), "outside").expect("outside fixture should write");
    symlink(
        outside.path.join("secret.txt"),
        plugin.path.join("escaped.txt"),
    )
    .expect("escape symlink should be created");
    let host = HostProcess::start_in(
        canonical(&plugin.path),
        "read-path.luau",
        json!({"path": "escaped.txt"}),
    );

    let fatal = host.read();

    assert_eq!(fatal["error"]["code"], "file_access");
}

#[test]
fn unsupported_namespace_fails_with_a_compatibility_error() {
    let host = HostProcess::start("unsupported-api.luau", json!({}));

    let fatal = host.read();

    assert_eq!(fatal["error"]["code"], "unsupported_api");
}

#[test]
fn unsupported_global_still_fails_with_a_compatibility_error() {
    let host = HostProcess::start("unsupported-global.luau", json!({}));

    let fatal = host.read();

    assert_eq!(
        (
            fatal["error"]["code"].as_str(),
            fatal["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("global.unsupportedLauncherGlobal"))
        ),
        (Some("unsupported_api"), true)
    );
}
