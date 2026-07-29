use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use crate::process::{CommandSpec, ProcessRunner, SystemProcess, command_text, find_command, os};
use crate::session::niri_config_path;

mod services;

const COMMANDS: &[&str] = &[
    "niri",
    "niri-session",
    "quickshell",
    "cage",
    "foot",
    "weyriva-luau-host",
    "wl-copy",
    "notify-send",
];
const REQUIRED_COMMANDS: &[&str] = &["niri", "niri-session", "quickshell", "cage"];
const UNITS: &[&str] = &[
    "weyriva-ipc.service",
    "weyriva-shell.service",
    "weyriva-session-failsafe.service",
];
#[derive(Clone, Debug)]
pub struct DiagnoseLayout {
    pub session_entry: PathBuf,
    pub greetd_config: PathBuf,
    pub packaged_units: PathBuf,
}

impl Default for DiagnoseLayout {
    fn default() -> Self {
        Self {
            session_entry: PathBuf::from("/usr/share/wayland-sessions/weyriva.desktop"),
            greetd_config: PathBuf::from("/etc/greetd/config.toml"),
            packaged_units: PathBuf::from("/usr/lib/systemd/user"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticReport {
    pub desktop: String,
    pub ok: bool,
    pub checks: Vec<DiagnosticCheck>,
}

pub struct Diagnoser {
    environment: BTreeMap<OsString, OsString>,
    layout: DiagnoseLayout,
    process: Arc<dyn ProcessRunner>,
}

impl Diagnoser {
    #[must_use]
    pub fn system() -> Self {
        Self::new(
            std::env::vars_os(),
            DiagnoseLayout::default(),
            Arc::new(SystemProcess),
        )
    }

    #[must_use]
    pub fn new(
        environment: impl IntoIterator<Item = (OsString, OsString)>,
        layout: DiagnoseLayout,
        process: Arc<dyn ProcessRunner>,
    ) -> Self {
        Self {
            environment: environment.into_iter().collect(),
            layout,
            process,
        }
    }

    #[must_use]
    pub fn run(&self) -> DiagnosticReport {
        let mut checks = self.command_checks();
        checks.push(self.niri_config_check());
        checks.push(self.session_entry_check());
        checks.push(self.greetd_check());
        checks.push(self.user_services_check());
        checks.push(self.user_manager_check());
        checks.push(self.niri_session_check());
        let ok = !checks.iter().any(|check| check.status == "fail");
        DiagnosticReport {
            desktop: "niri".to_owned(),
            ok,
            checks,
        }
    }

    fn command_checks(&self) -> Vec<DiagnosticCheck> {
        let path = self
            .environment
            .get(OsStr::new("PATH"))
            .map(OsString::as_os_str);
        COMMANDS
            .iter()
            .map(|command| match find_command(command, path) {
                Some(location) => check(
                    &format!("command:{command}"),
                    "ok",
                    &location.display().to_string(),
                ),
                None => check(
                    &format!("command:{command}"),
                    if REQUIRED_COMMANDS.contains(command) {
                        "fail"
                    } else {
                        "warn"
                    },
                    "not installed",
                ),
            })
            .collect()
    }

    fn niri_config_check(&self) -> DiagnosticCheck {
        let config = niri_config_path(&self.environment);
        if !config.is_file() {
            return check(
                "niri-config",
                "fail",
                &format!("missing: {}", config.display()),
            );
        }
        if !self.command_exists("niri") {
            return check(
                "niri-config",
                "warn",
                &format!("not validated; niri is missing ({})", config.display()),
            );
        }
        self.command_check(
            "niri-config",
            &CommandSpec::new(
                "niri",
                [os("validate"), os("-c"), config.as_os_str().to_os_string()],
            ),
            "fail",
            &config.display().to_string(),
        )
    }

    fn session_entry_check(&self) -> DiagnosticCheck {
        file_contains(
            "session-entry",
            &self.layout.session_entry,
            "Exec=/usr/bin/weyriva session start",
            "fail",
        )
    }

    fn greetd_check(&self) -> DiagnosticCheck {
        let path = &self.layout.greetd_config;
        let Ok(content) = fs::read_to_string(path) else {
            return check("greetd", "warn", &format!("missing: {}", path.display()));
        };
        if content.contains("start-hyprland") || content.contains("agreety") {
            return check("greetd", "fail", "still points to Hyprland/agreety");
        }
        let complete = [
            "HOME=/var/lib/weyriva-greeter",
            "XDG_STATE_HOME=/var/lib/weyriva-greeter/state",
            "XDG_CACHE_HOME=/var/lib/weyriva-greeter/cache",
            "XDG_CONFIG_HOME=/var/lib/weyriva-greeter/config",
            "/usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter",
            "user = \"greeter\"",
        ]
        .iter()
        .all(|marker| content.contains(marker));
        if complete {
            return check("greetd", "ok", "Weyriva Greeter is configured");
        }
        if content.contains("tuigreet") {
            return check(
                "greetd",
                "fail",
                "obsolete tuigreet login surface is configured",
            );
        }
        check(
            "greetd",
            "warn",
            "configured, but Weyriva login selection is unclear",
        )
    }

    fn user_services_check(&self) -> DiagnosticCheck {
        let config = if let Some(config) = self
            .environment
            .get(OsStr::new("XDG_CONFIG_HOME"))
            .filter(|value| !value.is_empty())
        {
            PathBuf::from(config)
        } else if let Some(home) = self
            .environment
            .get(OsStr::new("HOME"))
            .filter(|value| !value.is_empty())
        {
            PathBuf::from(home).join(".config")
        } else {
            return check(
                "user-services",
                "warn",
                "HOME and XDG_CONFIG_HOME are not set",
            );
        };
        let user_units = config.join("systemd/user");
        let missing: Vec<&str> = UNITS
            .iter()
            .copied()
            .filter(|name| {
                !user_units.join(name).is_file() && !self.layout.packaged_units.join(name).is_file()
            })
            .collect();
        if !missing.is_empty() {
            return check(
                "user-services",
                "warn",
                &format!("missing: {}", missing.join(", ")),
            );
        }
        let legacy = services::legacy_overrides(&user_units, &self.layout.packaged_units);
        if !legacy.is_empty() {
            return check(
                "user-services",
                "fail",
                &format!(
                    "legacy overrides: {}; run sudo weyriva startup ensure",
                    legacy.join(", ")
                ),
            );
        }
        let overrides: Vec<&str> = UNITS
            .iter()
            .copied()
            .filter(|name| user_units.join(name).is_file())
            .collect();
        let detail = if overrides.is_empty() {
            "packaged defaults".to_owned()
        } else {
            format!(
                "packaged defaults; user overrides: {}",
                overrides.join(", ")
            )
        };
        check("user-services", "ok", &detail)
    }

    fn user_manager_check(&self) -> DiagnosticCheck {
        if !self.environment.contains_key(OsStr::new("XDG_RUNTIME_DIR")) {
            return check("user-manager", "warn", "XDG_RUNTIME_DIR is not set");
        }
        self.command_check(
            "user-manager",
            &CommandSpec::new("systemctl", [os("--user"), os("is-system-running")])
                .with_timeout(Duration::from_secs(3)),
            "warn",
            "running",
        )
    }

    fn niri_session_check(&self) -> DiagnosticCheck {
        if !self.environment.contains_key(OsStr::new("NIRI_SOCKET")) {
            return check("niri-session", "warn", "not running in this environment");
        }
        self.command_check(
            "niri-session",
            &CommandSpec::new("niri", [os("msg"), os("-j"), os("version")])
                .with_timeout(Duration::from_secs(3)),
            "warn",
            "running",
        )
    }

    fn command_exists(&self, name: &str) -> bool {
        let path = self
            .environment
            .get(OsStr::new("PATH"))
            .map(OsString::as_os_str);
        find_command(name, path).is_some()
    }

    fn command_check(
        &self,
        name: &str,
        command: &CommandSpec,
        failure: &str,
        fallback: &str,
    ) -> DiagnosticCheck {
        match self.process.run(command) {
            Ok(output) => {
                let detail = command_text(&output);
                check(
                    name,
                    if output.code == 0 { "ok" } else { failure },
                    if detail.is_empty() { fallback } else { &detail },
                )
            }
            Err(error) => check(name, failure, &error.to_string()),
        }
    }
}

#[must_use]
pub fn render_plain(report: &DiagnosticReport) -> String {
    let mut output = String::new();
    for check in &report.checks {
        let symbol = match check.status.as_str() {
            "ok" => "OK",
            "warn" => "WARN",
            _ => "FAIL",
        };
        let _ = writeln!(output, "[{symbol:4}] {}: {}", check.name, check.detail);
    }
    let failures = report
        .checks
        .iter()
        .filter(|check| check.status == "fail")
        .count();
    let warnings = report
        .checks
        .iter()
        .filter(|check| check.status == "warn")
        .count();
    let _ = writeln!(
        output,
        "\nNiri diagnosis: {failures} failure(s), {warnings} warning(s)"
    );
    output
}

fn file_contains(name: &str, path: &Path, marker: &str, missing_status: &str) -> DiagnosticCheck {
    match fs::read_to_string(path) {
        Ok(content) => check(
            name,
            if content.contains(marker) {
                "ok"
            } else {
                "fail"
            },
            &path.display().to_string(),
        ),
        Err(_) => check(
            name,
            missing_status,
            &format!("missing: {}", path.display()),
        ),
    }
}

fn check(name: &str, status: &str, detail: &str) -> DiagnosticCheck {
    DiagnosticCheck {
        name: name.to_owned(),
        status: status.to_owned(),
        detail: detail.to_owned(),
    }
}
