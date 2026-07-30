mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value as JsonValue, json};
use tempfile::tempdir;
use weyriva::sources;
use weyriva::storage::atomic_json;
use weyriva::{Broker, Paths, Result, ipc};

#[test]
fn later_local_source_wins_and_install_is_immutable() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);
    let first = temporary.path().join("first");
    let second = temporary.path().join("second");
    common::write_plugin(&first, "1.0.0", "");
    common::write_plugin(&second, "2.0.0", "");
    let mut broker = Broker::with_host(paths, temporary.path().join("unused-host"));
    broker
        .source_add("first", &first)
        .expect("first source should be added");
    broker
        .source_add("second", &second)
        .expect("second source should be added");

    let installed = broker
        .install("test/demo")
        .expect("plugin should install from a local source");
    let plugin = &installed["plugins"][0];
    let root = plugin["path"]
        .as_str()
        .map(std::path::Path::new)
        .expect("installed path should be exposed");
    let root_mode = fs::metadata(root)
        .expect("installed root should exist")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(plugin["version"], "2.0.0");
    assert_eq!(plugin["provenance"]["source"], "second");
    assert_eq!(plugin["provider"]["categories"][0]["label"], "General");
    assert_eq!(root_mode, 0o555);
}

#[test]
fn uninstall_rejects_tampered_installed_path() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);
    let source = temporary.path().join("source");
    common::write_plugin(&source, "1.0.0", "");
    let mut broker = Broker::with_host(paths.clone(), temporary.path().join("unused-host"));
    broker
        .source_add("fixture", &source)
        .expect("fixture source should be added");
    broker
        .install("test/demo")
        .expect("fixture plugin should install");
    let mut state = sources::load_state(&paths).expect("state should load");
    let record = state
        .plugins
        .get_mut("test/demo")
        .expect("installed plugin should be recorded");
    let installed = record.path.clone();
    record.path = temporary.path().join("outside");
    atomic_json(&paths.state_file(), &state).expect("tampered state should be persisted");

    let error = broker
        .uninstall("test/demo")
        .expect_err("unsafe uninstall path should be rejected");

    assert_eq!(error.code(), "unsafe_state");
    assert!(installed.exists(), "real immutable install should remain");
}

#[test]
fn unix_daemon_is_private_exclusive_and_cleans_up() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);
    let host = common::install_fake_host(temporary.path());
    let (shutdown, handle) = start_daemon(paths.clone(), host);
    wait_for_socket(&paths.socket_file());

    let response = ipc::call(
        &paths.socket_file(),
        "weyriva.plugin.v1.source.list",
        &json!({}),
    )
    .expect("daemon should answer over its Unix socket");
    let socket_mode = fs::metadata(paths.socket_file())
        .expect("socket metadata should be readable")
        .permissions()
        .mode()
        & 0o777;
    let lock_mode = fs::metadata(paths.daemon_lock_file())
        .expect("lock metadata should be readable")
        .permissions()
        .mode()
        & 0o777;
    let competing = Broker::with_host(paths.clone(), temporary.path().join("unused-host"));
    let competing_error = ipc::serve_until(&paths, competing, &AtomicBool::new(true))
        .expect_err("a second daemon should not acquire the lock");

    assert_eq!(response["result"]["schema"], 1);
    assert_eq!(socket_mode, 0o600);
    assert_eq!(lock_mode, 0o600);
    assert_eq!(competing_error.code(), "daemon_running");

    stop_daemon(&shutdown, handle);
    assert!(
        !paths.socket_file().exists(),
        "normal shutdown should remove the socket"
    );
}

#[test]
fn cli_drives_complete_launcher_provider_lifecycle() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);
    let source = temporary.path().join("source");
    let plugin = common::write_plugin(
        &source,
        "1.0.0",
        "\n[[service]]\nid = \"sync\"\nentry = \"service.luau\"\n",
    );
    fs::write(plugin.join("service.luau"), "function update() end\n")
        .expect("service fixture should be written");
    let host = common::install_fake_host(temporary.path());
    let (shutdown, handle) = start_daemon(paths.clone(), host);
    wait_for_socket(&paths.socket_file());

    let add = run_cli(
        temporary.path(),
        &["plugin", "source", "add", "fixture", path_text(&source)],
    );
    let install = run_cli(temporary.path(), &["plugin", "install", "test/demo"]);
    let enable = run_cli(temporary.path(), &["plugin", "enable", "test/demo"]);
    let query = run_cli(
        temporary.path(),
        &["plugin", "query", "test/demo:main", "hello"],
    );
    let activate = run_cli(
        temporary.path(),
        &["plugin", "activate", "test/demo:main", "row-1"],
    );
    let ipc = run_cli(
        temporary.path(),
        &[
            "plugin",
            "ipc",
            "test/demo:sync",
            "refresh",
            "--payload",
            r#"{"generation":2}"#,
        ],
    );
    let disable = run_cli(temporary.path(), &["plugin", "disable", "test/demo"]);
    let uninstall = run_cli(temporary.path(), &["plugin", "uninstall", "test/demo"]);
    let remove = run_cli(temporary.path(), &["plugin", "source", "remove", "fixture"]);

    assert_eq!(add["result"]["source"]["name"], "fixture");
    assert_eq!(install["result"]["plugins"][0]["installed"], true);
    assert_eq!(
        install["result"]["plugins"][0]["provider"]["service"]["id"],
        "sync"
    );
    assert_eq!(enable["result"]["plugins"][0]["lifecycle"], "running");
    assert_eq!(query["result"]["results"][0]["title"], "Result hello");
    assert_eq!(activate["result"]["action_results"][0]["type"], "set_query");
    assert_eq!(ipc["result"]["value"]["entry"], "sync");
    assert_eq!(ipc["result"]["value"]["event"], "refresh");
    assert_eq!(ipc["result"]["value"]["payload"]["generation"], 2);
    assert_eq!(disable["result"]["status"]["plugins"][0]["enabled"], false);
    assert_eq!(disable["result"]["shutdown"]["onExit"], true);
    assert_eq!(disable["result"]["shutdown"]["actions"], json!([]));
    assert_eq!(disable["result"]["shutdown"]["action_results"], json!([]));
    assert_eq!(uninstall["result"]["uninstalled"], "test/demo");
    assert_eq!(remove["result"]["removed"], "fixture");

    stop_daemon(&shutdown, handle);
}

fn run_cli(root: &std::path::Path, arguments: &[&str]) -> JsonValue {
    let output = Command::new(env!("CARGO_BIN_EXE_weyriva"))
        .args(arguments)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_RUNTIME_DIR", root.join("runtime"))
        .output()
        .expect("CLI should execute");
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI should emit JSON")
}

fn path_text(path: &std::path::Path) -> &str {
    path.to_str().expect("fixture path should be UTF-8")
}

fn start_daemon(paths: Paths, host: PathBuf) -> (Arc<AtomicBool>, JoinHandle<Result<()>>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let daemon_shutdown = Arc::clone(&shutdown);
    let handle = std::thread::spawn(move || {
        let broker = Broker::with_host(paths.clone(), host);
        ipc::serve_until(&paths, broker, &daemon_shutdown)
    });
    (shutdown, handle)
}

fn wait_for_socket(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(path.exists(), "daemon socket should appear");
}

fn stop_daemon(shutdown: &AtomicBool, handle: JoinHandle<Result<()>>) {
    shutdown.store(true, Ordering::Relaxed);
    handle
        .join()
        .expect("daemon thread should not panic")
        .expect("daemon should stop cleanly");
}
