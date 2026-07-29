#[path = "support/process.rs"]
mod process;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use process::RecordingProcess;
use tempfile::{TempDir, tempdir};
use weyriva::diagnose::{DiagnoseLayout, Diagnoser};

struct Fixture {
    _temporary: TempDir,
    environment: Vec<(OsString, OsString)>,
    layout: DiagnoseLayout,
    user_units: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempdir().expect("temporary directory should be created");
        let root = temporary.path();
        let bin = root.join("bin");
        fs::create_dir(&bin).expect("binary directory should be created");
        for name in [
            "niri",
            "niri-session",
            "quickshell",
            "cage",
            "foot",
            "weyriva-luau-host",
            "wl-copy",
            "notify-send",
        ] {
            let path = bin.join(name);
            fs::write(&path, "#!/bin/true\n").expect("command should be written");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("command should be executable");
        }
        let config = root.join("config.kdl");
        fs::write(&config, "fixture\n").expect("niri config should be written");
        let session_entry = root.join("weyriva.desktop");
        fs::write(&session_entry, "Exec=/usr/bin/weyriva session start\n")
            .expect("session entry should be written");
        let greetd_config = root.join("greetd.toml");
        fs::write(&greetd_config, complete_greetd()).expect("greetd config should be written");
        let config_home = root.join("config");
        let user_units = config_home.join("systemd/user");
        fs::create_dir_all(&user_units).expect("user unit root should be created");
        let packaged_units = root.join("packaged");
        fs::create_dir(&packaged_units).expect("packaged unit root should be created");
        Self {
            environment: vec![
                (OsString::from("PATH"), bin.into_os_string()),
                (OsString::from("NIRI_CONFIG"), config.into_os_string()),
                (
                    OsString::from("XDG_CONFIG_HOME"),
                    config_home.into_os_string(),
                ),
            ],
            layout: DiagnoseLayout {
                session_entry,
                greetd_config,
                packaged_units,
            },
            user_units,
            _temporary: temporary,
        }
    }

    fn report(&self) -> weyriva::diagnose::DiagnosticReport {
        Diagnoser::new(
            self.environment.clone(),
            self.layout.clone(),
            Arc::new(RecordingProcess::default()),
        )
        .run()
    }
}

#[test]
fn missing_packaged_unit_precedes_detected_legacy_override() {
    let fixture = Fixture::new();
    for name in ["weyriva-ipc.service", "weyriva-session-failsafe.service"] {
        fs::write(fixture.layout.packaged_units.join(name), "packaged\n")
            .expect("packaged unit should be written");
    }
    fs::write(
        fixture.user_units.join("weyriva-ipc.service"),
        "ExecStart=/usr/bin/weyriva daemon\n",
    )
    .expect("legacy override should be written");

    let report = fixture.report();
    let services = report
        .checks
        .iter()
        .find(|check| check.name == "user-services")
        .expect("user-services check should exist");

    assert_eq!(services.status, "warn");
    assert_eq!(services.detail, "missing: weyriva-shell.service");
    assert!(
        report.ok,
        "warning-only parity must keep diagnose successful"
    );
}

#[test]
fn complete_greetd_precedes_obsolete_tuigreet_marker() {
    let fixture = Fixture::new();
    fs::write(
        &fixture.layout.greetd_config,
        format!("{}\n# tuigreet", complete_greetd()),
    )
    .expect("combined greetd config should be written");

    let report = fixture.report();
    let greetd = report
        .checks
        .iter()
        .find(|check| check.name == "greetd")
        .expect("greetd check should exist");

    assert_eq!(greetd.status, "ok");
    assert_eq!(greetd.detail, "Weyriva Greeter is configured");
}

#[test]
fn legacy_override_requires_a_matching_packaged_unit() {
    let fixture = Fixture::new();
    for name in [
        "weyriva-ipc.service",
        "weyriva-shell.service",
        "weyriva-session-failsafe.service",
    ] {
        fs::write(fixture.layout.packaged_units.join(name), "packaged\n")
            .expect("packaged unit should be written");
    }
    fs::write(
        fixture.user_units.join("weyriva-waybar.service"),
        "ExecStart=/usr/bin/waybar\n",
    )
    .expect("obsolete unmatched override should be written");

    let report = fixture.report();
    let services = report
        .checks
        .iter()
        .find(|check| check.name == "user-services")
        .expect("user-services check should exist");

    assert_eq!(services.status, "ok");
    assert_eq!(services.detail, "packaged defaults");
    assert!(report.ok);
}

fn complete_greetd() -> &'static str {
    r#"HOME=/var/lib/weyriva-greeter
XDG_STATE_HOME=/var/lib/weyriva-greeter/state
XDG_CACHE_HOME=/var/lib/weyriva-greeter/cache
XDG_CONFIG_HOME=/var/lib/weyriva-greeter/config
/usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter
user = "greeter"
"#
}
