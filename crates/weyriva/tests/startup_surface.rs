#[path = "support/process.rs"]
mod process;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use process::RecordingProcess;
use tempfile::{TempDir, tempdir};
use weyriva::Result;
use weyriva::identity::{IdentityProvider, UserAccount};
use weyriva::startup::{NoopMutationObserver, SystemClock, SystemFileSystem, SystemLocalTimezone};
use weyriva::startup::{Ownership, StartupContext, StartupLayout, apply, preflight};

struct Fixture {
    _temporary: TempDir,
    context: StartupContext,
    process: Arc<RecordingProcess>,
    ownership: Arc<RecordingOwnership>,
    home: PathBuf,
    packaged_niri: PathBuf,
    user_niri: PathBuf,
}

impl Fixture {
    #[allow(clippy::too_many_lines)]
    fn new(user_niri: &str) -> Self {
        let temporary = tempdir().expect("temporary directory should be created");
        let root = temporary.path();
        let home = root.join("home/tester");
        fs::create_dir_all(&home).expect("home should be created");
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
            write_executable(&bin.join(name));
        }

        let packaged_config = root.join("share/config");
        let packaged_data = root.join("share/data");
        let packaged_shell = root.join("share/shell");
        let packaged_greeter = root.join("share/greeter");
        let packaged_units = root.join("share/systemd");
        let greetd_template = root.join("share/greetd.toml");
        let greetd_config = root.join("etc/greetd/config.toml");
        let greetd_pam = root.join("etc/pam.d/greetd");
        let greeter_env = root.join("usr/bin/env");
        let greeter_session = root.join("usr/bin/cage");
        let greeter_state = root.join("var/lib/weyriva-greeter");
        let display_manager = root.join("etc/systemd/display-manager.service");
        let session_entry = root.join("share/weyriva.desktop");
        let packaged_niri = packaged_config.join("niri/config.kdl");
        let user_niri_path = home.join(".config/niri/config.kdl");

        write(&packaged_niri, "packaged niri\n");
        write(&packaged_shell.join("shell.qml"), "shell\n");
        write(&packaged_greeter.join("shell.qml"), "greeter\n");
        for wallpaper in ["light/weyriva-cactus.png", "dark/weyriva-cactus-dark.png"] {
            write(&packaged_data.join("wallpapers").join(wallpaper), "png\n");
        }
        for unit in [
            "weyriva-ipc.service",
            "weyriva-shell.service",
            "weyriva-session-failsafe.service",
        ] {
            write(&packaged_units.join(unit), "unit\n");
        }
        for unit in ["weyriva-ipc.service", "weyriva-shell.service"] {
            let link = packaged_units.join("niri.service.wants").join(unit);
            fs::create_dir_all(link.parent().expect("link parent should exist"))
                .expect("wants directory should be created");
            symlink(format!("../{unit}"), link).expect("wants link should be created");
        }
        write(
            &session_entry,
            "[Desktop Entry]\nName=Weyriva\nExec=/usr/bin/weyriva session start\n",
        );
        let greetd = "command = \"/usr/bin/env HOME=/var/lib/weyriva-greeter XDG_STATE_HOME=/var/lib/weyriva-greeter/state XDG_CACHE_HOME=/var/lib/weyriva-greeter/cache XDG_CONFIG_HOME=/var/lib/weyriva-greeter/config /usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter\"\nuser = \"greeter\"\n";
        write(&greetd_template, greetd);
        write(&greetd_config, greetd);
        write(&greetd_pam, "session include system-login\n");
        write_executable(&greeter_env);
        write_executable(&greeter_session);
        fs::create_dir_all(greeter_state.parent().expect("state parent should exist"))
            .expect("greeter state parent should be created");
        fs::create_dir_all(
            display_manager
                .parent()
                .expect("display manager parent should exist"),
        )
        .expect("display manager parent should be created");
        symlink("/usr/lib/systemd/system/greetd.service", &display_manager)
            .expect("display manager link should be created");
        match user_niri {
            "changed" => write(&user_niri_path, "old user niri\n"),
            "identical" => write(&user_niri_path, "packaged niri\n"),
            "missing" => {}
            _ => panic!("unknown fixture state"),
        }

        let identity = Arc::new(FakeIdentity {
            effective_uid: 0,
            home: home.clone(),
        });
        let process = Arc::new(RecordingProcess::default());
        let ownership = Arc::new(RecordingOwnership::default());
        let context = StartupContext {
            layout: StartupLayout {
                packaged_config,
                packaged_data,
                packaged_shell,
                packaged_greeter,
                packaged_units,
                greetd_template,
                greetd_config,
                greetd_pam,
                greeter_env,
                greeter_session,
                greeter_state,
                display_manager,
                session_entry,
            },
            environment: BTreeMap::from([
                (OsString::from("PATH"), bin.into_os_string()),
                (
                    OsString::from("WEYRIVA_STARTUP_TIMESTAMP"),
                    OsString::from("20260730-010203"),
                ),
            ]),
            identity,
            process: process.clone(),
            ownership: ownership.clone(),
            filesystem: Arc::new(SystemFileSystem),
            mutations: Arc::new(NoopMutationObserver),
            clock: Arc::new(SystemClock),
            timezone: Arc::new(SystemLocalTimezone),
        };
        Self {
            _temporary: temporary,
            context,
            process,
            ownership,
            home,
            packaged_niri,
            user_niri: user_niri_path,
        }
    }
}

#[derive(Clone)]
struct FakeIdentity {
    effective_uid: u32,
    home: PathBuf,
}

impl IdentityProvider for FakeIdentity {
    fn effective_uid(&self) -> u32 {
        self.effective_uid
    }

    fn current_uid(&self) -> u32 {
        1001
    }

    fn user(&self, name: &str) -> Result<Option<UserAccount>> {
        Ok(match name {
            "tester" => Some(UserAccount {
                name: name.to_owned(),
                uid: 1001,
                gid: 1002,
                home: self.home.clone(),
            }),
            "greeter" => Some(UserAccount {
                name: name.to_owned(),
                uid: 980,
                gid: 980,
                home: PathBuf::from("/var/lib/weyriva-greeter"),
            }),
            _ => None,
        })
    }

    fn group_gid(&self, name: &str) -> Result<Option<u32>> {
        Ok((name == "greeter").then_some(980))
    }
}

#[derive(Default)]
struct RecordingOwnership {
    paths: Mutex<Vec<(PathBuf, u32, u32)>>,
}

impl Ownership for RecordingOwnership {
    fn set_owner(&self, path: &Path, uid: u32, gid: u32) -> Result<()> {
        self.paths
            .lock()
            .expect("ownership lock should remain available")
            .push((path.to_path_buf(), uid, gid));
        Ok(())
    }
}

#[test]
fn startup_requires_root_and_real_non_root_user() {
    let mut fixture = Fixture::new("changed");
    fixture.context.identity = Arc::new(FakeIdentity {
        effective_uid: 1000,
        home: fixture.home.clone(),
    });
    assert_eq!(
        preflight(&fixture.context, "tester")
            .expect_err("non-root should be rejected")
            .code(),
        "root_required"
    );

    fixture.context.identity = Arc::new(FakeIdentity {
        effective_uid: 0,
        home: fixture.home.clone(),
    });
    assert_eq!(
        preflight(&fixture.context, "root")
            .expect_err("root target should be rejected")
            .code(),
        "invalid_user"
    );
}

#[test]
fn changed_niri_is_backed_up_replaced_owned_and_never_restarts_greetd() {
    let fixture = Fixture::new("changed");
    let plan = preflight(&fixture.context, "tester").expect("preflight should pass");
    queue_apply_success(&fixture.process);
    let result = apply(&fixture.context, &plan).expect("apply should pass");
    let backup = fixture
        .home
        .join(".local/state/weyriva/startup-backups/20260730-010203/niri/config.kdl");

    assert_eq!(
        fs::read(&fixture.user_niri).expect("user Niri should read"),
        fs::read(&fixture.packaged_niri).expect("packaged Niri should read")
    );
    assert_eq!(
        fs::read_to_string(backup).expect("backup should read"),
        "old user niri\n"
    );
    assert!(result.niri_changed);
    assert!(result.output.contains("greetd was not restarted"));
    let commands = fixture.process.commands();
    assert!(commands.iter().any(|command| {
        command.program == "systemctl"
            && command.arguments
                == ["enable", "--force", "greetd.service"]
                    .map(OsString::from)
                    .to_vec()
    }));
    assert!(!commands.iter().any(|command| {
        command
            .arguments
            .iter()
            .any(|argument| argument == "restart")
    }));
    assert!(
        fixture
            .ownership
            .paths
            .lock()
            .expect("ownership lock should remain available")
            .iter()
            .any(|(path, uid, gid)| path == &fixture.user_niri && (*uid, *gid) == (1001, 1002))
    );
}

#[test]
fn identical_niri_is_idempotent_without_backup() {
    let fixture = Fixture::new("identical");
    let plan = preflight(&fixture.context, "tester").expect("preflight should pass");
    queue_apply_success(&fixture.process);
    let result = apply(&fixture.context, &plan).expect("apply should pass");

    assert!(!plan.niri_changed);
    assert!(!result.niri_changed);
    assert!(!plan.niri_backup.exists());
}

#[test]
fn exact_legacy_units_are_moved_and_similar_units_remain() {
    let fixture = Fixture::new("identical");
    let unit_root = fixture.home.join(".config/systemd/user");
    write(
        &unit_root.join("weyriva-waybar.service"),
        "ExecStart=%h/.local/bin/weyriva component waybar\n",
    );
    write(
        &unit_root.join("weyriva-waybar-custom.service"),
        "ExecStart=%h/.local/bin/weyriva component waybar\n",
    );
    let plan = preflight(&fixture.context, "tester").expect("preflight should pass");
    queue_apply_success(&fixture.process);
    let result = apply(&fixture.context, &plan).expect("apply should pass");

    assert_eq!(result.moved_units, ["weyriva-waybar.service"]);
    assert!(!unit_root.join("weyriva-waybar.service").exists());
    assert!(unit_root.join("weyriva-waybar-custom.service").exists());
}

#[test]
fn symlink_destination_and_backup_collision_are_rejected_before_apply() {
    let fixture = Fixture::new("missing");
    let unsafe_target = fixture.home.join("unsafe");
    write(&unsafe_target, "unsafe\n");
    fs::create_dir_all(
        fixture
            .user_niri
            .parent()
            .expect("Niri parent should exist"),
    )
    .expect("Niri parent should be created");
    symlink(&unsafe_target, &fixture.user_niri).expect("unsafe symlink should be created");
    assert!(preflight(&fixture.context, "tester").is_err());

    fs::remove_file(&fixture.user_niri).expect("unsafe symlink should be removed");
    write(&fixture.user_niri, "changed\n");
    let collision = fixture
        .home
        .join(".local/state/weyriva/startup-backups/20260730-010203/niri/config.kdl");
    write(&collision, "collision\n");
    assert_eq!(
        preflight(&fixture.context, "tester")
            .expect_err("backup collision should fail")
            .code(),
        "backup_collision"
    );
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("file parent should exist"))
        .expect("file parent should be created");
    fs::write(path, content).expect("fixture file should be written");
}

fn queue_apply_success(process: &RecordingProcess) {
    process.push(0, "enabled\n", "");
    process.push(0, "", "");
    process.push(0, "enabled\n", "");
}

fn write_executable(path: &Path) {
    write(path, "#!/bin/true\n");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("fixture should be executable");
}
