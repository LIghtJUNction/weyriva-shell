use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use serde_json::json;
use tempfile::tempdir;
use weyriva::Broker;
use weyriva::model::PluginProfile;
use weyriva::v4_host_session::V4HostSession;
use weyriva::v4_manifest::parse_plugin;

mod common;

fn write_v4_plugin(root: &std::path::Path, entries: &serde_json::Value) -> std::path::PathBuf {
    let plugin = root.join("kaomoji-provider");
    fs::create_dir_all(&plugin).expect("fixture directory should be created");
    fs::write(
        plugin.join("manifest.json"),
        serde_json::to_vec(&json!({
            "id": "kaomoji-provider",
            "name": "Kaomoji",
            "version": "1.0.0",
            "author": "Noctalia",
            "description": "Pinned legacy v4 launcher provider",
            "tags": ["Launcher", "Fun"],
            "official": true,
            "entryPoints": entries,
            "metadata": {"commandPrefix": "kaomoji"}
        }))
        .expect("fixture manifest should encode"),
    )
    .expect("fixture manifest should be written");
    fs::write(
        plugin.join("Main.qml"),
        "import QtQuick\nimport Quickshell.Io\nQtObject { IpcHandler { target: \"plugin:kaomoji\"; function toggle() {} } }\n",
    )
    .expect("main fixture should be written");
    fs::write(
        plugin.join("LauncherProvider.qml"),
        "import QtQuick\nimport Quickshell\nQtObject { property var pluginApi; property var launcher; function copy() { Quickshell.execDetached([\"wl-copy\", \"fixture\"]) } }\n",
    )
    .expect("launcher fixture should be written");
    plugin
}

#[test]
fn pinned_v4_manifest_exposes_exact_entries_and_real_handler_target() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = write_v4_plugin(
        temporary.path(),
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );

    let candidate = parse_plugin(&plugin).expect("pinned v4 manifest should parse");

    assert_eq!(candidate.provider.plugin_id, "kaomoji-provider");
    assert_eq!(candidate.provider.entry_id, "launcher");
    assert_eq!(candidate.runtime.main, "Main.qml");
    assert_eq!(candidate.runtime.launcher_provider, "LauncherProvider.qml");
    assert_eq!(candidate.runtime.handler_target, "plugin:kaomoji");
}

#[test]
fn v4_manifest_rejects_extra_entry_kinds() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = write_v4_plugin(
        temporary.path(),
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml",
            "panel": "Panel.qml"
        }),
    );

    let error = parse_plugin(&plugin).expect_err("unimplemented v4 entry kinds should fail");

    assert_eq!(error.code(), "unsupported_plugin");
}

#[test]
fn profiled_install_persists_v4_without_changing_default_v5_state_shape() {
    let temporary = tempdir().expect("temporary directory should be created");
    let source = temporary.path().join("source");
    write_v4_plugin(
        &source,
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );
    let paths = common::paths(&temporary);
    let mut broker = Broker::with_host(paths, temporary.path().join("unused-luau-host"));
    broker
        .source_add("v4-fixture", &source)
        .expect("v4 fixture source should be added");

    let installed = broker
        .install_profiled("kaomoji-provider", PluginProfile::V4Qml)
        .expect("profiled v4 plugin should install");

    assert_eq!(installed["plugins"][0]["profile"], "noctalia-v4-qml/1");
    assert_eq!(
        installed["plugins"][0]["v4_runtime"]["handler_target"],
        "plugin:kaomoji"
    );
}

fn write_query_fixture(plugin: &std::path::Path) {
    fs::create_dir(plugin.join("data")).expect("fixture data directory should be created");
    fs::write(
        plugin.join("data/kaomoji.json"),
        r#"{"a'b":{"new_tags":["happy"]}}"#,
    )
    .expect("fixture data should be written");
    fs::write(
        plugin.join("Main.qml"),
        r#"
import QtQuick
import Quickshell.Io

Item {
    property var pluginApi
    IpcHandler {
        target: "plugin:kaomoji"
        function toggle(): string {
            pluginApi.withCurrentScreen(function(screen) {
                pluginApi.toggleLauncher()
            })
            return "toggled"
        }
    }
}
"#,
    )
    .expect("main fixture should be replaced");
    fs::write(
        plugin.join("LauncherProvider.qml"),
        r#"
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons

Item {
    visible: false
    property var pluginApi
    property var launcher
    FileView {
        id: dataFile
        path: pluginApi.pluginDir + "/data/kaomoji.json"
        blockLoading: true
    }
    function init() {
        Logger.i("Kaomoji", "fixture initialized")
    }
    function handleCommand(text) {
        return text.startsWith(">kaomoji")
    }
    function getResults(text) {
        if (!text.startsWith(">kaomoji"))
            return []
        const database = JSON.parse(dataFile.text())
        return Object.keys(database).map(function(value, index) {
            return {
                id: String(index),
                title: value,
                subtitle: database[value].new_tags.join(", "),
                activate: function() {
                    Quickshell.execDetached([
                        "sh",
                        "-c",
                        "printf '%s' '" + value.replace(/'/g, "'\\''") + "' | wl-copy"
                    ])
                }
            }
        })
    }
}
"#,
    )
    .expect("launcher fixture should be replaced");
}

#[test]
fn v4_host_uses_fixed_quickshell_argv_and_private_bootstrap() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = write_v4_plugin(
        temporary.path(),
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );
    let candidate = parse_plugin(&plugin).expect("v4 fixture should parse");
    let executable = temporary.path().join("fake-quickshell");
    fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
if [ "$1" = "--path" ]; then
    printf '%s\n' "$@" > "$XDG_RUNTIME_DIR/launch.args"
    while :; do sleep 1; done
fi
while [ ! -f "$XDG_RUNTIME_DIR/launch.args" ]; do sleep 0.01; done
printf '%s\n' '{"ok":true,"id":1,"result":{"ready":true},"error":null}'
"#,
    )
    .expect("fake quickshell should be written");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
        .expect("fake quickshell should be executable");
    let host_root = temporary.path().join("owned-v4-host");
    fs::create_dir(&host_root).expect("owned host root should be created");
    fs::write(host_root.join("shell.qml"), "import QtQuick\nQtObject {}\n")
        .expect("owned shell fixture should be written");

    let mut session = V4HostSession::start(
        &executable,
        &host_root,
        &temporary.path().join("runtime"),
        &candidate.provider.plugin_id,
        &candidate.root,
        &candidate.runtime,
    )
    .expect("v4 host should start");

    let runtime = session.runtime_dir().to_path_buf();
    let launch = fs::read_to_string(runtime.join("launch.args"))
        .expect("launch evidence should be readable");
    let runtime_mode = fs::metadata(&runtime)
        .expect("runtime metadata should be readable")
        .mode()
        & 0o777;
    let bootstrap_mode = fs::metadata(runtime.join("bootstrap.json"))
        .expect("bootstrap metadata should be readable")
        .mode()
        & 0o777;
    let bootstrap: serde_json::Value = serde_json::from_slice(
        &fs::read(runtime.join("bootstrap.json")).expect("bootstrap should be readable"),
    )
    .expect("bootstrap should be JSON");
    session
        .terminate()
        .expect("v4 host termination should be proven");

    assert_eq!(launch, format!("--path\n{}\n", host_root.display()));
    assert_eq!(runtime_mode, 0o700);
    assert_eq!(bootstrap_mode, 0o600);
    assert_eq!(bootstrap["handlerTarget"], "plugin:kaomoji");
}

#[test]
fn owned_v4_host_loads_fixture_and_normalizes_query_and_activation() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = write_v4_plugin(
        temporary.path(),
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );
    write_query_fixture(&plugin);
    let candidate = parse_plugin(&plugin).expect("self fixture should parse");
    let host_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../v4-host");
    let mut session = V4HostSession::start(
        std::path::Path::new("/usr/bin/quickshell"),
        &host_root,
        &temporary.path().join("runtime"),
        &candidate.provider.plugin_id,
        &candidate.root,
        &candidate.runtime,
    )
    .expect("owned v4 host should load the fixture");

    let query = session
        .request("query", json!({"query": ">kaomoji happy"}))
        .expect("v4 query should succeed");
    let activation = session
        .request("activate", json!({"id": "0"}))
        .expect("v4 activation should succeed");
    session
        .shutdown()
        .expect("owned v4 host should shut down cleanly");

    assert_eq!(query["query"], ">kaomoji happy");
    assert_eq!(query["results"][0]["title"], "a'b");
    assert_eq!(
        activation["actions"][0],
        json!({"type": "clipboard", "text": "a'b", "mime": "text/plain"})
    );
}

#[test]
fn broker_routes_v4_query_and_real_handler_alias_without_core_shell_coupling() {
    let temporary = tempdir().expect("temporary directory should be created");
    let source = temporary.path().join("source");
    let plugin = write_v4_plugin(
        &source,
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );
    fs::write(
        plugin.join("Main.qml"),
        r#"
import QtQuick
import Quickshell.Io
Item {
    property var pluginApi
    IpcHandler {
        target: "plugin:kaomoji"
        function toggle(): string {
            pluginApi.withCurrentScreen(function(screen) {
                pluginApi.toggleLauncher()
            })
            return "alias-toggled"
        }
    }
}
"#,
    )
    .expect("broker main fixture should be written");
    fs::write(
        plugin.join("LauncherProvider.qml"),
        r#"
import QtQuick
import Quickshell
Item {
    visible: false
    property var pluginApi
    property var launcher
    function handleCommand(text) {
        launcher.updateResults([{
            id: "face",
            title: "(•‿•)",
            activate: function() {
                Quickshell.execDetached(["wl-copy", "(•‿•)"])
            }
        }])
    }
}
"#,
    )
    .expect("broker launcher fixture should be written");
    let paths = common::paths(&temporary);
    let host_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../v4-host");
    let mut broker = Broker::with_hosts(
        paths,
        temporary.path().join("unused-luau-host"),
        std::path::PathBuf::from("/usr/bin/quickshell"),
        host_root,
    );
    broker
        .source_add("v4-fixture", &source)
        .expect("v4 source should be added");
    broker
        .install_profiled("kaomoji-provider", PluginProfile::V4Qml)
        .expect("v4 fixture should install");
    broker
        .enable("kaomoji-provider")
        .expect("v4 fixture should enable");

    let query = broker
        .query("kaomoji-provider:launcher", "")
        .expect("v4 broker query should succeed");
    let ipc = broker
        .ipc(
            "kaomoji-provider:launcher",
            "toggle",
            &serde_json::json!({}),
        )
        .expect("real handler alias should be routed");
    let disabled = broker
        .disable("kaomoji-provider")
        .expect("isolated v4 host should terminate on disable");

    assert_eq!(query["results"][0]["title"], "(•‿•)");
    assert_eq!(ipc["value"], "alias-toggled");
    assert_eq!(
        ipc["actions"][0],
        json!({"type": "launcher_set_query", "query": ""})
    );
    assert_eq!(disabled["shutdown"]["onExit"], false);
}

#[test]
fn v4_load_failure_is_isolated_and_broker_remains_available() {
    let temporary = tempdir().expect("temporary directory should be created");
    let source = temporary.path().join("source");
    let plugin = write_v4_plugin(
        &source,
        &json!({
            "main": "Main.qml",
            "launcherProvider": "LauncherProvider.qml"
        }),
    );
    fs::write(
        plugin.join("LauncherProvider.qml"),
        "import QtQuick\nimport Quickshell\nItem { property var pluginApi; property var launcher; function copy() { Quickshell.execDetached([]) }\n",
    )
    .expect("malformed launcher fixture should be written");
    let paths = common::paths(&temporary);
    let host_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../v4-host");
    let mut broker = Broker::with_hosts(
        paths,
        temporary.path().join("unused-luau-host"),
        std::path::PathBuf::from("/usr/bin/quickshell"),
        host_root,
    );
    broker
        .source_add("broken-v4-fixture", &source)
        .expect("broken v4 source should be added");
    broker
        .install_profiled("kaomoji-provider", PluginProfile::V4Qml)
        .expect("broken QML should remain an isolated runtime concern");

    let error = broker
        .enable("kaomoji-provider")
        .expect_err("malformed QML must not be enabled");
    let status = broker
        .status(Some("kaomoji-provider"))
        .expect("broker status should survive the host failure");
    let sources = broker
        .source_list()
        .expect("broker source API should survive the host failure");

    assert!(
        matches!(
            error.code(),
            "plugin_error" | "host_protocol" | "host_timeout" | "host_exited"
        ),
        "unexpected isolated host error: {error}"
    );
    assert_eq!(status.plugins[0].lifecycle, "failed");
    assert!(status.plugins[0].failure.is_some());
    assert!(
        sources["sources"]
            .as_array()
            .expect("source list should be an array")
            .iter()
            .any(|source| source["name"] == "broken-v4-fixture")
    );
}

#[test]
fn owned_v4_host_is_headless_and_core_shell_never_references_plugin_qml() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let forbidden_surface_tokens = [
        "Window {",
        "PanelWindow",
        "LayerShell",
        "MouseArea",
        "TapHandler",
        "HoverHandler",
        "Keys.on",
    ];
    for entry in walkdir::WalkDir::new(repository.join("v4-host")) {
        let entry = entry.expect("owned v4 host should be readable");
        if entry.path().extension().and_then(std::ffi::OsStr::to_str) != Some("qml") {
            continue;
        }
        let source = fs::read_to_string(entry.path()).expect("owned QML should be readable");
        for token in forbidden_surface_tokens {
            assert!(
                !source.contains(token),
                "{} must remain headless and input-free: {token}",
                entry.path().display()
            );
        }
    }
    for entry in walkdir::WalkDir::new(repository.join("shell")) {
        let entry = entry.expect("core shell should be readable");
        if entry.path().extension().and_then(std::ffi::OsStr::to_str) != Some("qml") {
            continue;
        }
        let source = fs::read_to_string(entry.path()).expect("core shell QML should be readable");
        for token in [
            "LauncherProvider.qml",
            "kaomoji-provider",
            "pluginDir +",
            "manifest.json",
        ] {
            assert!(
                !source.contains(token),
                "{} must not import external plugin QML: {token}",
                entry.path().display()
            );
        }
    }
}
