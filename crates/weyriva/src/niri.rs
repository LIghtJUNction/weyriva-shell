use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value as JsonValue;

use crate::error::{Error, Result};
use crate::process::{CommandSpec, ProcessRunner, SystemProcess, command_text, os};

pub struct NiriClient {
    process: Arc<dyn ProcessRunner>,
}

impl NiriClient {
    #[must_use]
    pub fn system() -> Self {
        Self::new(Arc::new(SystemProcess))
    }

    #[must_use]
    pub fn new(process: Arc<dyn ProcessRunner>) -> Self {
        Self { process }
    }

    /// Queries one supported Niri JSON endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported operations, process failure, or invalid JSON.
    pub fn json(&self, operation: &str) -> Result<JsonValue> {
        if !matches!(operation, "outputs" | "windows") {
            return Err(Error::new("invalid_operation", "unsupported Niri query"));
        }
        let command = CommandSpec::new("niri", [os("msg"), os("-j"), OsString::from(operation)])
            .with_timeout(Duration::from_secs(3));
        let output = self
            .process
            .run(&command)
            .map_err(|error| Error::new("unavailable", format!("niri msg unavailable: {error}")))?;
        if output.code != 0 {
            return Err(Error::new(
                "action_failed",
                if output.stderr.trim().is_empty() {
                    "niri msg failed".to_owned()
                } else {
                    output.stderr.trim().to_owned()
                },
            ));
        }
        serde_json::from_str(&output.stdout)
            .map_err(|_| Error::new("action_failed", "niri msg returned invalid JSON"))
    }

    /// Validates a Niri configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error when Niri cannot run or rejects the configuration.
    pub fn validate(&self, path: &std::path::Path) -> Result<()> {
        let command = CommandSpec::new(
            "niri",
            [os("validate"), os("-c"), path.as_os_str().to_os_string()],
        );
        let output = self.process.run(&command)?;
        if output.code == 0 {
            Ok(())
        } else {
            let detail = command_text(&output);
            Err(Error::new(
                "invalid_niri_config",
                if detail.is_empty() {
                    format!("Niri config is invalid: {}", path.display())
                } else {
                    format!("Niri config is invalid ({}): {detail}", path.display())
                },
            ))
        }
    }
}
