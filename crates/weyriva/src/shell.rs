use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value as JsonValue, json};

use crate::error::{Error, Result};
use crate::process::{CommandSpec, ExecRunner, ProcessRunner, SystemProcess, command_text, os};

const PACKAGED_SHELL_ROOT: &str = "/usr/share/weyriva/shell";

pub struct ShellController {
    environment: BTreeMap<OsString, OsString>,
    process: Arc<dyn ProcessRunner>,
    executor: Arc<dyn ExecRunner>,
}

impl ShellController {
    #[must_use]
    pub fn from_environment() -> Self {
        Self::new(
            std::env::vars_os(),
            Arc::new(SystemProcess),
            Arc::new(SystemProcess),
        )
    }

    #[must_use]
    pub fn new(
        environment: impl IntoIterator<Item = (OsString, OsString)>,
        process: Arc<dyn ProcessRunner>,
        executor: Arc<dyn ExecRunner>,
    ) -> Self {
        Self {
            environment: environment.into_iter().collect(),
            process,
            executor,
        }
    }

    #[must_use]
    pub fn root(&self) -> PathBuf {
        shell_root(&self.environment)
    }

    /// Replaces the current process with the Weyriva Quickshell entrypoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the shell process cannot be executed.
    pub fn run(&self) -> Result<()> {
        self.executor.exec(
            &CommandSpec::new("quickshell", [os("--path"), self.root().into_os_string()])
                .with_environment(self.environment.clone()),
        )
    }

    /// Calls one Weyriva Quickshell IPC function with fixed argv.
    ///
    /// # Errors
    ///
    /// Returns an error for process failure, nonzero exit, or invalid output.
    pub fn call(&self, function: &str, arguments: &[&str]) -> Result<JsonValue> {
        let mut argv = vec![
            os("--path"),
            self.root().into_os_string(),
            os("ipc"),
            os("call"),
            os("weyriva"),
            os(function),
        ];
        argv.extend(arguments.iter().map(os));
        let command = CommandSpec::new("quickshell", argv)
            .with_timeout(Duration::from_secs(5))
            .with_environment(self.environment.clone());
        let output = self.process.run(&command).map_err(|error| {
            Error::with_details(
                "unavailable",
                format!("Weyriva shell IPC unavailable: {error}"),
                json!({"command": function}),
            )
        })?;
        if output.code != 0 {
            let detail = command_text(&output);
            return Err(Error::new(
                "action_failed",
                if detail.is_empty() {
                    "Weyriva shell IPC failed".to_owned()
                } else {
                    detail
                },
            ));
        }
        Ok(json!({"command": function, "output": output.stdout.trim()}))
    }
}

#[must_use]
pub fn shell_root(environment: &BTreeMap<OsString, OsString>) -> PathBuf {
    let home = environment
        .get(OsStr::new("HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute());
    let data_home = environment
        .get(OsStr::new("XDG_DATA_HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|path| path.join(".local/share")));
    let Some(data_home) = data_home else {
        return Path::new(PACKAGED_SHELL_ROOT).to_path_buf();
    };
    let user_root = data_home.join("weyriva/shell");
    if user_root.join("shell.qml").is_file() {
        user_root
    } else {
        Path::new(PACKAGED_SHELL_ROOT).to_path_buf()
    }
}
