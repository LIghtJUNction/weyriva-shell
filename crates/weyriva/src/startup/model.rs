use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use crate::identity::IdentityProvider;
use crate::identity::SystemIdentity;
use crate::process::{ProcessRunner, SystemProcess};

use super::safety::{
    FileSystem, MutationObserver, NoopMutationObserver, Ownership, SystemFileSystem,
    SystemOwnership,
};
use super::time::{Clock, LocalTimezone, SystemClock, SystemLocalTimezone};

pub const WEYRIVA_UNITS: &[&str] = &[
    "weyriva-ipc.service",
    "weyriva-shell.service",
    "weyriva-session-failsafe.service",
];
pub const NIRI_WANTED_UNITS: &[&str] = &["weyriva-ipc.service", "weyriva-shell.service"];
pub const REQUIRED_COMMANDS: &[&str] = &[
    "niri",
    "niri-session",
    "quickshell",
    "cage",
    "foot",
    "weyriva-luau-host",
    "wl-copy",
    "notify-send",
];

#[derive(Clone, Debug)]
pub struct StartupLayout {
    pub packaged_config: PathBuf,
    pub packaged_data: PathBuf,
    pub packaged_shell: PathBuf,
    pub packaged_greeter: PathBuf,
    pub packaged_units: PathBuf,
    pub greetd_template: PathBuf,
    pub greetd_config: PathBuf,
    pub greetd_pam: PathBuf,
    pub greeter_env: PathBuf,
    pub greeter_session: PathBuf,
    pub greeter_state: PathBuf,
    pub display_manager: PathBuf,
    pub session_entry: PathBuf,
}

impl Default for StartupLayout {
    fn default() -> Self {
        Self {
            packaged_config: PathBuf::from("/usr/share/weyriva/config"),
            packaged_data: PathBuf::from("/usr/share/weyriva"),
            packaged_shell: PathBuf::from("/usr/share/weyriva/shell"),
            packaged_greeter: PathBuf::from("/usr/share/weyriva/greeter"),
            packaged_units: PathBuf::from("/usr/lib/systemd/user"),
            greetd_template: PathBuf::from("/usr/share/weyriva/greetd/config.toml"),
            greetd_config: PathBuf::from("/etc/greetd/config.toml"),
            greetd_pam: PathBuf::from("/etc/pam.d/greetd"),
            greeter_env: PathBuf::from("/usr/bin/env"),
            greeter_session: PathBuf::from("/usr/bin/cage"),
            greeter_state: PathBuf::from("/var/lib/weyriva-greeter"),
            display_manager: PathBuf::from("/etc/systemd/system/display-manager.service"),
            session_entry: PathBuf::from("/usr/share/wayland-sessions/weyriva.desktop"),
        }
    }
}

pub struct StartupContext {
    pub layout: StartupLayout,
    pub environment: BTreeMap<OsString, OsString>,
    pub identity: Arc<dyn IdentityProvider>,
    pub process: Arc<dyn ProcessRunner>,
    pub ownership: Arc<dyn Ownership>,
    pub filesystem: Arc<dyn FileSystem>,
    pub mutations: Arc<dyn MutationObserver>,
    pub clock: Arc<dyn Clock>,
    pub timezone: Arc<dyn LocalTimezone>,
}

impl StartupContext {
    #[must_use]
    pub fn system() -> Self {
        Self {
            layout: StartupLayout::default(),
            environment: std::env::vars_os().collect(),
            identity: Arc::new(SystemIdentity),
            process: Arc::new(SystemProcess),
            ownership: Arc::new(SystemOwnership),
            filesystem: Arc::new(SystemFileSystem),
            mutations: Arc::new(NoopMutationObserver),
            clock: Arc::new(SystemClock),
            timezone: Arc::new(SystemLocalTimezone),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayManagerState {
    Absent,
    Link(PathBuf),
}

#[derive(Clone, Debug)]
pub struct StartupPlan {
    pub target_user: String,
    pub user_home: PathBuf,
    pub user_uid: u32,
    pub user_gid: u32,
    pub greeter_uid: u32,
    pub greeter_gid: u32,
    pub niri_config: PathBuf,
    pub niri_backup: PathBuf,
    pub niri_changed: bool,
    pub greetd_backup: PathBuf,
    pub greetd_changed: bool,
    pub backup_root: PathBuf,
    pub unit_root: PathBuf,
    pub units_to_back_up: Vec<String>,
    pub display_manager_before: DisplayManagerState,
    pub display_manager_record: Option<PathBuf>,
    pub create_display_manager_record: bool,
}
