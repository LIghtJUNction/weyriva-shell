use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub environment: BTreeMap<OsString, OsString>,
    pub timeout: Duration,
}

impl CommandSpec {
    #[must_use]
    pub fn new(
        program: impl Into<OsString>,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Self {
        Self {
            program: program.into(),
            arguments: arguments.into_iter().collect(),
            environment: BTreeMap::new(),
            timeout: Duration::from_secs(5),
        }
    }

    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_environment(
        mut self,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Self {
        self.environment = environment.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner: Send + Sync {
    /// Runs one fixed-argv command to completion.
    ///
    /// # Errors
    ///
    /// Returns an error when spawning, waiting, reading output, or enforcing
    /// the timeout fails.
    fn run(&self, command: &CommandSpec) -> Result<ProcessOutput>;
}

pub trait ExecRunner: Send + Sync {
    /// Replaces the current process with one fixed-argv command.
    ///
    /// # Errors
    ///
    /// Returns only when the operating-system exec operation fails.
    fn exec(&self, command: &CommandSpec) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemProcess;

impl ProcessRunner for SystemProcess {
    fn run(&self, command: &CommandSpec) -> Result<ProcessOutput> {
        let mut child = build_command(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| Error::io("cannot start command", &error))?;
        let deadline = Instant::now() + command.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::new(
                        "command_timeout",
                        format!("command timed out: {}", command.program.to_string_lossy()),
                    ));
                }
                Err(error) => return Err(Error::io("cannot inspect command", &error)),
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|error| Error::io("cannot collect command output", &error))?;
        Ok(ProcessOutput {
            code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8(output.stdout)
                .map_err(|_| Error::new("invalid_output", "command stdout is not UTF-8"))?,
            stderr: String::from_utf8(output.stderr)
                .map_err(|_| Error::new("invalid_output", "command stderr is not UTF-8"))?,
        })
    }
}

impl ExecRunner for SystemProcess {
    fn exec(&self, command: &CommandSpec) -> Result<()> {
        use std::os::unix::process::CommandExt;

        let error = build_command(command).exec();
        Err(Error::io("cannot exec command", &error))
    }
}

fn build_command(command: &CommandSpec) -> Command {
    let mut process = Command::new(&command.program);
    process.args(&command.arguments);
    if !command.environment.is_empty() {
        process.env_clear();
        process.envs(&command.environment);
    }
    process
}

#[must_use]
pub fn command_text(output: &ProcessOutput) -> String {
    let stdout = output.stdout.trim();
    if stdout.is_empty() {
        output.stderr.trim().to_owned()
    } else {
        stdout.to_owned()
    }
}

#[must_use]
pub fn os(value: impl AsRef<OsStr>) -> OsString {
    value.as_ref().to_os_string()
}

#[must_use]
pub fn find_command(name: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return executable(candidate).then(|| candidate.to_path_buf());
    }
    std::env::split_paths(path.unwrap_or_else(|| OsStr::new("")))
        .map(|directory| directory.join(name))
        .find(|candidate| executable(candidate))
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
