use std::process::{Command, Output};

use tempfile::tempdir;

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_weyriva"))
        .args(arguments)
        .output()
        .expect("CLI should execute")
}

#[test]
fn version_is_exact() {
    let output = run(&["--version"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "weyriva 0.1.0\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn complete_command_matrix_parses() {
    let cases: &[&[&str]] = &[
        &["daemon", "--help"],
        &["status", "--help"],
        &["diagnose", "--help"],
        &["startup", "ensure", "--help"],
        &["ipc", "call", "weyriva.ping", "--help"],
        &["shell", "run", "--help"],
        &["shell", "reconcile-lock", "--help"],
        &["shell", "route", "launcher", "--help"],
        &["shell", "lock", "--help"],
        &["session", "start", "--help"],
        &["plugin", "source", "list", "--help"],
        &["plugin", "source", "add", "local", "/tmp/source", "--help"],
        &["plugin", "source", "remove", "local", "--help"],
        &["plugin", "install", "test/demo", "--help"],
        &["plugin", "status", "--help"],
        &["plugin", "enable", "test/demo", "--help"],
        &["plugin", "disable", "test/demo", "--help"],
        &["plugin", "reload", "test/demo", "--help"],
        &["plugin", "uninstall", "test/demo", "--help"],
        &["plugin", "query", "test/demo:main", "text", "--help"],
        &["plugin", "activate", "test/demo:main", "row", "--help"],
    ];

    for arguments in cases {
        let output = run(arguments);
        assert_eq!(
            output.status.code(),
            Some(0),
            "failed to parse {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn route_names_are_exact() {
    for route in [
        "launcher",
        "control-center",
        "calendar",
        "notifications",
        "wallpaper",
        "settings",
    ] {
        let output = run(&["shell", "route", route, "--help"]);
        assert_eq!(output.status.code(), Some(0), "{route}");
    }
    let invalid = run(&["shell", "route", "control"]);
    assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn retired_legacy_plugin_commands_and_idless_reload_exit_two() {
    for arguments in [
        &["plugin", "list"][..],
        &["plugin", "validate", "plugin.json"][..],
        &["plugin", "reload"][..],
    ] {
        let output = run(arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
    }
}

#[test]
fn invalid_json_is_a_usage_error() {
    let output = run(&["ipc", "call", "weyriva.ping", "--params", "{bad"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON"));
}

#[test]
fn only_daemon_startup_runs_legacy_migration() {
    let temporary = tempdir().expect("temporary directory should be created");
    let root = temporary.path();
    let legacy_data = root.join("data/weyriva/plugins-v5");
    std::fs::create_dir_all(&legacy_data).expect("legacy data should be created");
    std::fs::write(legacy_data.join("orphan"), "fixture\n")
        .expect("orphan legacy data should be written");
    let configure = |command: &mut Command| {
        command
            .env_clear()
            .env("HOME", root.join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("XDG_STATE_HOME", root.join("state"))
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_RUNTIME_DIR", root.join("runtime"));
    };
    let mut status = Command::new(env!("CARGO_BIN_EXE_weyriva"));
    configure(&mut status);
    let status = status
        .arg("status")
        .output()
        .expect("status should execute");
    let mut daemon = Command::new(env!("CARGO_BIN_EXE_weyriva"));
    configure(&mut daemon);
    let daemon = daemon
        .arg("daemon")
        .output()
        .expect("daemon should execute");

    assert_eq!(status.status.code(), Some(1));
    assert!(
        !String::from_utf8_lossy(&status.stderr).contains("legacy plugin data exists"),
        "client commands must not migrate"
    );
    assert_eq!(daemon.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&daemon.stderr).contains("legacy plugin data exists"));
}

#[test]
fn missing_home_without_xdg_roots_is_an_explicit_error() {
    let temporary = tempdir().expect("temporary directory should be created");
    let output = Command::new(env!("CARGO_BIN_EXE_weyriva"))
        .arg("status")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_RUNTIME_DIR", temporary.path())
        .output()
        .expect("status should execute");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("XDG_CONFIG_HOME and HOME are not set")
    );
}
