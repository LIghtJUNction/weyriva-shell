use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::niri::NiriClient;
use crate::process::{CommandSpec, ExecRunner, ProcessRunner, SystemProcess, find_command};

const PACKAGED_NIRI_CONFIG: &str = "/usr/share/weyriva/config/niri/config.kdl";

pub struct SessionController {
    environment: BTreeMap<OsString, OsString>,
    argv0: OsString,
    process: Arc<dyn ProcessRunner>,
    executor: Arc<dyn ExecRunner>,
}

impl SessionController {
    #[must_use]
    pub fn from_environment(argv0: OsString) -> Self {
        Self::new(
            argv0,
            std::env::vars_os(),
            Arc::new(SystemProcess),
            Arc::new(SystemProcess),
        )
    }

    #[must_use]
    pub fn new(
        argv0: OsString,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
        process: Arc<dyn ProcessRunner>,
        executor: Arc<dyn ExecRunner>,
    ) -> Self {
        Self {
            argv0,
            environment: environment.into_iter().collect(),
            process,
            executor,
        }
    }

    /// Validates and replaces the current process with `niri-session`.
    ///
    /// # Errors
    ///
    /// Returns an error for missing commands, invalid configuration, or exec failure.
    pub fn start(&self) -> Result<()> {
        let path = self
            .environment
            .get(OsStr::new("PATH"))
            .map(OsString::as_os_str);
        if find_command("niri-session", path).is_none() {
            return Err(Error::new(
                "command_missing",
                "niri-session is not installed; install niri before starting the session",
            ));
        }
        if find_command("niri", path).is_none() {
            return Err(Error::new(
                "command_missing",
                "niri is not installed; install niri before starting the session",
            ));
        }
        let config = niri_config_path(&self.environment);
        NiriClient::new(Arc::clone(&self.process)).validate(&config)?;
        let mut environment = session_environment(&self.argv0, &self.environment);
        environment.insert(OsString::from("NIRI_CONFIG"), config.into_os_string());
        self.executor.exec(
            &CommandSpec::new("niri-session", std::iter::empty()).with_environment(environment),
        )
    }
}

#[must_use]
pub fn niri_config_path(environment: &BTreeMap<OsString, OsString>) -> PathBuf {
    if let Some(selected) = environment.get(OsStr::new("NIRI_CONFIG"))
        && !selected.is_empty()
    {
        return PathBuf::from(selected);
    }
    let home = environment
        .get(OsStr::new("HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute());
    let config_home = environment
        .get(OsStr::new("XDG_CONFIG_HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|path| path.join(".config")));
    let Some(config_home) = config_home else {
        return Path::new(PACKAGED_NIRI_CONFIG).to_path_buf();
    };
    let user_config = config_home.join("niri/config.kdl");
    if user_config.is_file() {
        user_config
    } else {
        Path::new(PACKAGED_NIRI_CONFIG).to_path_buf()
    }
}

#[must_use]
pub fn session_environment(
    argv0: &OsStr,
    environment: &BTreeMap<OsString, OsString>,
) -> BTreeMap<OsString, OsString> {
    let path = environment.get(OsStr::new("PATH")).map(OsString::as_os_str);
    let executable = if Path::new(argv0).parent() == Some(Path::new("")) {
        find_command(&argv0.to_string_lossy(), path).unwrap_or_else(|| PathBuf::from(argv0))
    } else {
        PathBuf::from(argv0)
    };
    let directory = executable
        .canonicalize()
        .unwrap_or(executable)
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut child = environment.clone();
    let mut entries: Vec<PathBuf> = path
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect();
    if !entries.iter().any(|entry| entry == &directory) {
        entries.insert(0, directory);
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        child.insert(OsString::from("PATH"), joined);
    }
    child
}
