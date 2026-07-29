#[path = "support/process.rs"]
mod process;

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use process::RecordingProcess;
use tempfile::tempdir;
use weyriva::Result;
use weyriva::diagnose::{DiagnoseLayout, Diagnoser, render_plain};
use weyriva::identity::{IdentityProvider, UserAccount};
use weyriva::lock::{LockReconciler, valid_session_id};
use weyriva::session::{SessionController, niri_config_path, session_environment};

#[derive(Default)]
struct FakeIdentity;

impl IdentityProvider for FakeIdentity {
    fn effective_uid(&self) -> u32 {
        1000
    }

    fn current_uid(&self) -> u32 {
        1000
    }

    fn user(&self, _name: &str) -> Result<Option<UserAccount>> {
        Ok(None)
    }

    fn group_gid(&self, _name: &str) -> Result<Option<u32>> {
        Ok(None)
    }
}

#[test]
fn lock_succeeds_only_for_exact_unlocked_hint() {
    for (stdout, expected) in [
        ("no\n", true),
        ("NO\n", true),
        ("yes\n", false),
        ("unknown\n", false),
        ("", false),
    ] {
        let process = Arc::new(RecordingProcess::default());
        process.push(0, stdout, "");
        let reconciler = LockReconciler::new(process, Arc::new(FakeIdentity));
        let environment =
            BTreeMap::from([(OsString::from("XDG_SESSION_ID"), OsString::from("c2"))]);

        assert_eq!(reconciler.reconcile(&environment), expected, "{stdout:?}");
    }
}

#[test]
fn lock_fails_closed_for_error_timeout_and_unsafe_id() {
    let process = Arc::new(RecordingProcess::default());
    process.fail("command_timeout", "injected timeout");
    let reconciler = LockReconciler::new(process.clone(), Arc::new(FakeIdentity));
    let environment = BTreeMap::from([(OsString::from("XDG_SESSION_ID"), OsString::from("c3"))]);
    assert!(!reconciler.reconcile(&environment));

    for value in ["", " c3", "c3 ", "/session", "a:b", &"x".repeat(129)] {
        assert!(!valid_session_id(value), "{value:?}");
    }
}

#[test]
fn lock_fallback_uses_uid_then_validated_session_id() {
    let process = Arc::new(RecordingProcess::default());
    process.push(0, "c9\n", "");
    process.push(0, "no\n", "");
    let reconciler = LockReconciler::new(process.clone(), Arc::new(FakeIdentity));

    assert!(reconciler.reconcile(&BTreeMap::new()));
    let commands = process.commands();
    assert_eq!(
        commands[0].arguments,
        ["show-user", "1000", "-p", "Display", "--value"]
            .map(OsString::from)
            .to_vec()
    );
    assert_eq!(
        commands[1].arguments,
        ["show-session", "c9", "-p", "LockedHint", "--value"]
            .map(OsString::from)
            .to_vec()
    );
}

#[test]
fn niri_config_precedence_is_explicit_user_then_packaged() {
    let temporary = tempdir().expect("temporary directory should be created");
    let home = temporary.path();
    let user = home.join(".config/niri/config.kdl");
    fs::create_dir_all(user.parent().expect("config parent should exist"))
        .expect("config directory should be created");
    fs::write(&user, "user\n").expect("user config should be written");
    let environment = BTreeMap::from([(OsString::from("HOME"), home.as_os_str().to_os_string())]);
    assert_eq!(niri_config_path(&environment), user);

    let explicit = home.join("explicit.kdl");
    let explicit_environment = BTreeMap::from([
        (OsString::from("HOME"), home.as_os_str().to_os_string()),
        (
            OsString::from("NIRI_CONFIG"),
            explicit.as_os_str().to_os_string(),
        ),
    ]);
    assert_eq!(niri_config_path(&explicit_environment), explicit);
}

#[test]
fn session_validates_fixed_argv_and_execs_with_augmented_path() {
    let temporary = tempdir().expect("temporary directory should be created");
    let bin = temporary.path().join("bin");
    fs::create_dir(&bin).expect("binary directory should be created");
    for name in ["niri", "niri-session", "weyriva"] {
        let path = bin.join(name);
        fs::write(&path, "#!/bin/true\n").expect("fixture executable should be written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("fixture executable should be executable");
    }
    let config = temporary.path().join("config.kdl");
    fs::write(&config, "fixture\n").expect("fixture config should be written");
    let environment = BTreeMap::from([
        (OsString::from("PATH"), bin.as_os_str().to_os_string()),
        (
            OsString::from("NIRI_CONFIG"),
            config.as_os_str().to_os_string(),
        ),
    ]);
    let process = Arc::new(RecordingProcess::default());
    process.push(0, "", "");
    let session = SessionController::new(
        bin.join("weyriva").into_os_string(),
        environment,
        process.clone(),
        process.clone(),
    );

    let error = session
        .start()
        .expect_err("test exec should be intercepted");
    let commands = process.commands();

    assert_eq!(error.code(), "exec_intercepted");
    assert_eq!(commands[0].program, OsStr::new("niri"));
    assert_eq!(
        commands[0].arguments,
        [
            OsString::from("validate"),
            OsString::from("-c"),
            config.into_os_string()
        ]
    );
    assert_eq!(commands[1].program, OsStr::new("niri-session"));
    assert_eq!(
        commands[1].environment.get(OsStr::new("NIRI_CONFIG")),
        commands[0].arguments.get(2)
    );
}

#[test]
fn session_environment_prepends_invoked_binary_directory_once() {
    let environment = BTreeMap::from([(OsString::from("PATH"), OsString::from("/usr/bin:/bin"))]);
    let child = session_environment(OsStr::new("/opt/weyriva/bin/weyriva"), &environment);

    assert_eq!(
        child
            .get(OsStr::new("PATH"))
            .expect("PATH should exist")
            .to_string_lossy(),
        "/opt/weyriva/bin:/usr/bin:/bin"
    );
}

#[test]
fn diagnose_renders_ok_warn_fail_and_warning_only_is_success() {
    let temporary = tempdir().expect("temporary directory should be created");
    let root = temporary.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("binary directory should be created");
    for name in ["niri", "niri-session", "quickshell", "cage"] {
        let path = bin.join(name);
        fs::write(&path, "#!/bin/true\n").expect("command should be written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("command should be executable");
    }
    let session = root.join("weyriva.desktop");
    let greetd = root.join("greetd.toml");
    let units = root.join("units");
    let config = root.join("config.kdl");
    fs::create_dir(&units).expect("units directory should be created");
    fs::write(&session, "Exec=/usr/bin/weyriva session start\n")
        .expect("session should be written");
    fs::write(
        &greetd,
        "HOME=/var/lib/weyriva-greeter\nXDG_STATE_HOME=/var/lib/weyriva-greeter/state\nXDG_CACHE_HOME=/var/lib/weyriva-greeter/cache\nXDG_CONFIG_HOME=/var/lib/weyriva-greeter/config\n/usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter\nuser = \"greeter\"\n",
    )
    .expect("greetd should be written");
    fs::write(&config, "fixture\n").expect("config should be written");
    for name in [
        "weyriva-waybar.service",
        "weyriva-mako.service",
        "weyriva-wallpaper.service",
        "weyriva-idle.service",
        "weyriva-ipc.service",
        "weyriva-shell.service",
        "weyriva-session-failsafe.service",
    ] {
        fs::write(units.join(name), "fixture\n").expect("unit should be written");
    }
    let process = Arc::new(RecordingProcess::default());
    process.push(0, "", "");
    let report = Diagnoser::new(
        [
            (OsString::from("PATH"), bin.into_os_string()),
            (OsString::from("NIRI_CONFIG"), config.into_os_string()),
            (
                OsString::from("XDG_CONFIG_HOME"),
                root.join("config").into_os_string(),
            ),
        ],
        DiagnoseLayout {
            session_entry: session,
            greetd_config: greetd,
            packaged_units: units,
        },
        process,
    )
    .run();
    let rendered = render_plain(&report);

    assert!(report.ok);
    assert!(report.checks.iter().any(|check| check.status == "warn"));
    assert!(rendered.contains("[OK  ] command:niri:"));
    assert!(rendered.contains("[WARN] command:foot: not installed"));
    assert!(rendered.ends_with("Niri diagnosis: 0 failure(s), 6 warning(s)\n"));
}

#[test]
fn diagnose_rejects_exact_legacy_user_service_override() {
    let temporary = tempdir().expect("temporary directory should be created");
    let config = temporary.path().join("config");
    let user_units = config.join("systemd/user");
    let packaged_units = temporary.path().join("packaged-units");
    fs::create_dir_all(&user_units).expect("user unit directory should be created");
    fs::create_dir(&packaged_units).expect("packaged unit directory should be created");
    for name in [
        "weyriva-waybar.service",
        "weyriva-mako.service",
        "weyriva-wallpaper.service",
        "weyriva-idle.service",
        "weyriva-ipc.service",
        "weyriva-shell.service",
        "weyriva-session-failsafe.service",
    ] {
        fs::write(packaged_units.join(name), "packaged\n")
            .expect("packaged unit should be written");
    }
    for (name, marker) in [
        (
            "weyriva-waybar.service",
            "ExecStart=%h/.local/bin/weyriva component waybar\n",
        ),
        (
            "weyriva-mako.service",
            "ExecStart=%h/.local/bin/weyriva component mako\n",
        ),
        (
            "weyriva-wallpaper.service",
            "ExecStart=%h/.local/bin/weyriva wallpaper\n",
        ),
        (
            "weyriva-idle.service",
            "ExecStart=%h/.local/bin/weyriva idle\n",
        ),
        (
            "weyriva-ipc.service",
            "ExecStart=%h/.local/bin/weyriva daemon\n",
        ),
        (
            "weyriva-shell.service",
            "ExecStart=%h/.local/bin/weyriva shell run\n",
        ),
        (
            "weyriva-session-failsafe.service",
            "ExecStart=/usr/bin/niri msg action quit --skip-confirmation\n",
        ),
    ] {
        fs::write(user_units.join(name), marker).expect("legacy override should be written");
    }

    let report = Diagnoser::new(
        [(OsString::from("XDG_CONFIG_HOME"), config.into_os_string())],
        DiagnoseLayout {
            session_entry: temporary.path().join("missing-session"),
            greetd_config: temporary.path().join("missing-greetd"),
            packaged_units,
        },
        Arc::new(RecordingProcess::default()),
    )
    .run();
    let services = report
        .checks
        .iter()
        .find(|check| check.name == "user-services")
        .expect("user service check should exist");

    assert_eq!(services.status, "fail");
    assert_eq!(
        services.detail,
        "legacy overrides: weyriva-waybar.service, weyriva-mako.service, weyriva-wallpaper.service, weyriva-idle.service, weyriva-ipc.service, weyriva-shell.service, weyriva-session-failsafe.service; run sudo weyriva startup ensure"
    );
}

#[test]
fn diagnose_missing_home_never_inspects_a_relative_user_unit_root() {
    let temporary = tempdir().expect("temporary directory should be created");
    let report = Diagnoser::new(
        std::iter::empty(),
        DiagnoseLayout {
            session_entry: temporary.path().join("missing-session"),
            greetd_config: temporary.path().join("missing-greetd"),
            packaged_units: temporary.path().join("missing-units"),
        },
        Arc::new(RecordingProcess::default()),
    )
    .run();
    let services = report
        .checks
        .iter()
        .find(|check| check.name == "user-services")
        .expect("user service check should exist");

    assert_eq!(services.status, "warn");
    assert_eq!(services.detail, "HOME and XDG_CONFIG_HOME are not set");
}
