use std::env;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub data_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

impl Paths {
    /// Resolves the XDG roots for the current user.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime root or an XDG/home fallback is unavailable.
    pub fn from_env() -> Result<Self> {
        let home = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let config = xdg_root("XDG_CONFIG_HOME", home.as_deref(), ".config")?;
        let state = xdg_root("XDG_STATE_HOME", home.as_deref(), ".local/state")?;
        let data = xdg_root("XDG_DATA_HOME", home.as_deref(), ".local/share")?;
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| Error::new("runtime_unavailable", "XDG_RUNTIME_DIR is not set"))?;
        for (name, path) in [
            ("XDG_CONFIG_HOME", &config),
            ("XDG_STATE_HOME", &state),
            ("XDG_DATA_HOME", &data),
            ("XDG_RUNTIME_DIR", &runtime),
        ] {
            if !path.is_absolute() {
                return Err(Error::new(
                    "invalid_xdg_root",
                    format!("{name} must resolve to an absolute path"),
                ));
            }
        }
        Ok(Self::new(&config, &state, &data, &runtime))
    }

    #[must_use]
    pub fn new(config: &Path, state: &Path, data: &Path, runtime: &Path) -> Self {
        Self {
            config_dir: config.join("weyriva/plugins"),
            state_dir: state.join("weyriva/plugins"),
            data_dir: data.join("weyriva/plugins"),
            runtime_dir: runtime.join("weyriva"),
        }
    }

    #[must_use]
    pub fn sources_file(&self) -> PathBuf {
        self.config_dir.join("sources.json")
    }

    #[must_use]
    pub fn state_file(&self) -> PathBuf {
        self.state_dir.join("state.json")
    }

    #[must_use]
    pub fn socket_file(&self) -> PathBuf {
        self.runtime_dir.join("weyriva.sock")
    }

    #[must_use]
    pub fn daemon_lock_file(&self) -> PathBuf {
        self.runtime_dir.join("daemon.lock")
    }
}

fn xdg_root(name: &str, home: Option<&Path>, fallback: &str) -> Result<PathBuf> {
    if let Some(value) = env::var_os(name).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }
    home.map(|path| path.join(fallback))
        .ok_or_else(|| Error::new("home_unavailable", format!("{name} and HOME are not set")))
}
