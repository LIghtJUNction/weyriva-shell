#![allow(dead_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::{TempDir, tempdir};
use weyriva::identity::{IdentityProvider, UserAccount};
use weyriva::process::{CommandSpec, ProcessOutput, ProcessRunner};
use weyriva::startup::{
    MutationObserver, Ownership, StartupContext, StartupLayout, StartupPlan, SystemClock,
    SystemFileSystem, SystemLocalTimezone, preflight,
};
use weyriva::{Error, Result};

mod filesystem;
mod snapshot;

pub use filesystem::TestFileSystem;
pub use snapshot::Snapshot;

const GREETD_TARGET: &str = "/usr/lib/systemd/system/greetd.service";
const TIMESTAMP: &str = "20260730-010203";

pub struct Fixture {
    pub _temporary: TempDir,
    pub context: StartupContext,
    pub process: Arc<StateProcess>,
    pub ownership: Arc<StateOwnership>,
    pub observer: Arc<FaultObserver>,
    pub root: PathBuf,
    pub old_target: PathBuf,
}

impl Fixture {
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let temporary = tempdir().expect("temporary directory should be created");
        let root = temporary.path().to_path_buf();
        let home = root.join("home/tester");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("binary directory should be created");
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
            executable(&bin.join(name));
        }
        let layout = layout(&root);
        write(
            &layout.packaged_config.join("niri/config.kdl"),
            "new niri\n",
        );
        write(&layout.packaged_shell.join("shell.qml"), "shell\n");
        write(&layout.packaged_greeter.join("shell.qml"), "greeter\n");
        for wallpaper in ["light/weyriva-cactus.png", "dark/weyriva-cactus-dark.png"] {
            write(
                &layout.packaged_data.join("wallpapers").join(wallpaper),
                "png\n",
            );
        }
        for unit in [
            "weyriva-ipc.service",
            "weyriva-shell.service",
            "weyriva-session-failsafe.service",
        ] {
            write(&layout.packaged_units.join(unit), "unit\n");
        }
        for unit in ["weyriva-ipc.service", "weyriva-shell.service"] {
            let link = layout.packaged_units.join("niri.service.wants").join(unit);
            fs::create_dir_all(link.parent().expect("link should have a parent"))
                .expect("wants directory should be created");
            symlink(format!("../{unit}"), link).expect("wants link should be created");
        }
        write(
            &layout.session_entry,
            "[Desktop Entry]\nName=Weyriva\nExec=/usr/bin/weyriva session start\n",
        );
        write(
            &layout.greetd_template,
            "command = \"/usr/bin/env HOME=/var/lib/weyriva-greeter XDG_STATE_HOME=/var/lib/weyriva-greeter/state XDG_CACHE_HOME=/var/lib/weyriva-greeter/cache XDG_CONFIG_HOME=/var/lib/weyriva-greeter/config /usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter\"\nuser = \"greeter\"\n",
        );
        write(&layout.greetd_config, "old greetd\n");
        write(&layout.greetd_pam, "session include system-login\n");
        executable(&layout.greeter_env);
        executable(&layout.greeter_session);
        fs::create_dir_all(&layout.greeter_state).expect("greeter state should be created");
        fs::set_permissions(&layout.greeter_state, fs::Permissions::from_mode(0o755))
            .expect("greeter mode should be set");
        write(&home.join(".config/niri/config.kdl"), "old niri\n");
        write(
            &home.join(".config/systemd/user/weyriva-waybar.service"),
            "ExecStart=/usr/bin/waybar\n",
        );
        let old_target = layout
            .display_manager
            .parent()
            .expect("display manager should have a parent")
            .join("old.service");
        write(&old_target, "old display manager\n");
        symlink("old.service", &layout.display_manager)
            .expect("display manager link should be created");
        let backup_root = home
            .join(".local/state/weyriva/startup-backups")
            .join(TIMESTAMP);
        fs::create_dir_all(&backup_root).expect("backup root should be created");
        symlink(&old_target, backup_root.join("unrelated-link"))
            .expect("backup symlink should be created");

        let ownership = Arc::new(StateOwnership::default());
        let process = Arc::new(StateProcess::new(layout.display_manager.clone()));
        let observer = Arc::new(FaultObserver::default());
        let context = StartupContext {
            layout,
            environment: BTreeMap::from([
                (OsString::from("PATH"), bin.into_os_string()),
                (
                    OsString::from("WEYRIVA_STARTUP_TIMESTAMP"),
                    OsString::from(TIMESTAMP),
                ),
            ]),
            identity: Arc::new(FakeIdentity { home }),
            process: process.clone(),
            ownership: ownership.clone(),
            filesystem: Arc::new(SystemFileSystem),
            mutations: observer.clone(),
            clock: Arc::new(SystemClock),
            timezone: Arc::new(SystemLocalTimezone),
        };
        Self {
            _temporary: temporary,
            context,
            process,
            ownership,
            observer,
            root,
            old_target,
        }
    }

    pub fn plan(&self) -> StartupPlan {
        preflight(&self.context, "tester").expect("fixture preflight should pass")
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::capture(&self.root, &self.ownership, self.process.enabled())
    }
}

fn layout(root: &Path) -> StartupLayout {
    StartupLayout {
        packaged_config: root.join("share/config"),
        packaged_data: root.join("share/data"),
        packaged_shell: root.join("share/shell"),
        packaged_greeter: root.join("share/greeter"),
        packaged_units: root.join("share/systemd"),
        greetd_template: root.join("share/greetd.toml"),
        greetd_config: root.join("etc/greetd/config.toml"),
        greetd_pam: root.join("etc/pam.d/greetd"),
        greeter_env: root.join("usr/bin/env"),
        greeter_session: root.join("usr/bin/cage"),
        greeter_state: root.join("var/lib/weyriva-greeter"),
        display_manager: root.join("etc/systemd/display-manager.service"),
        session_entry: root.join("share/weyriva.desktop"),
    }
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("file should have a parent"))
        .expect("file parent should be created");
    fs::write(path, content).expect("file should be written");
}

fn executable(path: &Path) {
    write(path, "#!/bin/sh\n");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("executable mode should be set");
}

struct FakeIdentity {
    home: PathBuf,
}

impl IdentityProvider for FakeIdentity {
    fn effective_uid(&self) -> u32 {
        0
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
pub struct StateOwnership {
    owners: Mutex<BTreeMap<PathBuf, (u32, u32)>>,
}

impl StateOwnership {
    pub fn owner(&self, path: &Path, metadata: &fs::Metadata) -> (u32, u32) {
        self.owners
            .lock()
            .expect("ownership lock should remain available")
            .get(path)
            .copied()
            .unwrap_or((metadata.uid(), metadata.gid()))
    }
}

impl Ownership for StateOwnership {
    fn set_owner(&self, path: &Path, uid: u32, gid: u32) -> Result<()> {
        self.owners
            .lock()
            .expect("ownership lock should remain available")
            .insert(path.to_path_buf(), (uid, gid));
        Ok(())
    }
}

pub struct StateProcess {
    display_manager: PathBuf,
    enabled: AtomicBool,
    fail_enable: AtomicBool,
    commands: Mutex<Vec<CommandSpec>>,
}

impl StateProcess {
    fn new(display_manager: PathBuf) -> Self {
        Self {
            display_manager,
            enabled: AtomicBool::new(false),
            fail_enable: AtomicBool::new(false),
            commands: Mutex::new(Vec::new()),
        }
    }

    pub fn fail_next_enable_after_mutation(&self) {
        self.fail_enable.store(true, Ordering::SeqCst);
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn commands(&self) -> Vec<CommandSpec> {
        self.commands
            .lock()
            .expect("command lock should remain available")
            .clone()
    }

    fn select_greetd(&self) -> Result<()> {
        match fs::remove_file(&self.display_manager) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("cannot replace test display manager", &error)),
        }
        symlink(GREETD_TARGET, &self.display_manager)
            .map_err(|error| Error::io("cannot select test display manager", &error))
    }
}

impl ProcessRunner for StateProcess {
    fn run(&self, command: &CommandSpec) -> Result<ProcessOutput> {
        self.commands
            .lock()
            .expect("command lock should remain available")
            .push(command.clone());
        let program = command.program.to_string_lossy();
        let args: Vec<_> = command
            .arguments
            .iter()
            .map(|value| value.to_string_lossy())
            .collect();
        if program == "niri" || args.first().is_some_and(|value| value == "cat") {
            return Ok(output(0, ""));
        }
        match args.first().map(AsRef::as_ref) {
            Some("is-enabled") => Ok(output(
                i32::from(!self.enabled()),
                if self.enabled() {
                    "enabled\n"
                } else {
                    "disabled\n"
                },
            )),
            Some("enable") => {
                self.select_greetd()?;
                self.enabled.store(true, Ordering::SeqCst);
                if self.fail_enable.swap(false, Ordering::SeqCst) {
                    Ok(output(1, "injected enable failure"))
                } else {
                    Ok(output(0, ""))
                }
            }
            Some("disable") => {
                self.enabled.store(false, Ordering::SeqCst);
                Ok(output(0, ""))
            }
            _ => Err(Error::new(
                "unexpected_command",
                format!("{program} {args:?}"),
            )),
        }
    }
}

fn output(code: i32, stdout: &str) -> ProcessOutput {
    ProcessOutput {
        code,
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

#[derive(Default)]
pub struct FaultObserver {
    applied: AtomicUsize,
    rollback: AtomicUsize,
    fail_applied: AtomicUsize,
    fail_rollback: AtomicUsize,
    labels: Mutex<Vec<String>>,
}

impl FaultObserver {
    pub fn fail_applied(&self, index: usize) {
        self.fail_applied.store(index, Ordering::SeqCst);
    }

    pub fn fail_rollback(&self, index: usize) {
        self.fail_rollback.store(index, Ordering::SeqCst);
    }

    pub fn labels(&self) -> Vec<String> {
        self.labels
            .lock()
            .expect("label lock should remain available")
            .clone()
    }
}

impl MutationObserver for FaultObserver {
    fn applied(&self, operation: &str) -> Result<()> {
        self.labels
            .lock()
            .expect("label lock should remain available")
            .push(operation.to_owned());
        let index = self.applied.fetch_add(1, Ordering::SeqCst) + 1;
        if index == self.fail_applied.load(Ordering::SeqCst) {
            Err(Error::new("injected_apply_failure", operation))
        } else {
            Ok(())
        }
    }

    fn before_rollback(&self, operation: &str) -> Result<()> {
        let index = self.rollback.fetch_add(1, Ordering::SeqCst) + 1;
        if index == self.fail_rollback.load(Ordering::SeqCst) {
            Err(Error::new("injected_rollback_failure", operation))
        } else {
            Ok(())
        }
    }
}
