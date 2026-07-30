use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsFd;
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nix::poll::{PollFd, PollFlags, poll};
use serde_json::{Value as JsonValue, json};

use crate::error::{Error, Result};
use crate::model::{HOST_PROTOCOL, HostEvent, HostRequest, HostResponse, MAX_LINE_BYTES, Provider};

const HOST_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_HOST_RESPONSE_BYTES: usize = 1024 * 1024;

pub struct HostSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: u64,
    process_control: Arc<dyn ProcessControl>,
    startup_actions: Vec<JsonValue>,
}

/// Fallible child-process operations used to prove host termination.
pub trait ProcessControl: Send + Sync {
    /// Polls the child without blocking.
    ///
    /// # Errors
    ///
    /// Returns the operating-system error when status cannot be inspected.
    fn try_wait(&self, child: &mut Child) -> std::io::Result<Option<ExitStatus>>;

    /// Requests forced child termination.
    ///
    /// # Errors
    ///
    /// Returns the operating-system error when the signal cannot be delivered.
    fn kill(&self, child: &mut Child) -> std::io::Result<()>;

    /// Waits for and reaps the child.
    ///
    /// # Errors
    ///
    /// Returns the operating-system error when exit cannot be confirmed.
    fn wait(&self, child: &mut Child) -> std::io::Result<ExitStatus>;
}

#[derive(Debug, Default)]
pub struct SystemProcessControl;

impl ProcessControl for SystemProcessControl {
    fn try_wait(&self, child: &mut Child) -> std::io::Result<Option<ExitStatus>> {
        child.try_wait()
    }

    fn kill(&self, child: &mut Child) -> std::io::Result<()> {
        child.kill()
    }

    fn wait(&self, child: &mut Child) -> std::io::Result<ExitStatus> {
        child.wait()
    }
}

impl HostSession {
    /// Starts one persistent launcher provider and verifies its ready event.
    ///
    /// # Errors
    ///
    /// Returns an error when the executable is missing, the child cannot be
    /// spawned, or its bounded ready event is invalid.
    pub fn start(
        executable: &Path,
        plugin_dir: &Path,
        provider: &Provider,
        settings: &JsonValue,
    ) -> Result<Self> {
        Self::start_with_process_control(
            executable,
            plugin_dir,
            provider,
            settings,
            Arc::new(SystemProcessControl),
        )
    }

    /// Starts a host using an injectable process-control boundary.
    ///
    /// # Errors
    ///
    /// Returns an error when startup or ready-event validation fails.
    pub fn start_with_process_control(
        executable: &Path,
        plugin_dir: &Path,
        provider: &Provider,
        settings: &JsonValue,
        process_control: Arc<dyn ProcessControl>,
    ) -> Result<Self> {
        let settings_json = serde_json::to_string(settings)?;
        let mut command = Command::new(executable);
        command
            .arg("--plugin-dir")
            .arg(plugin_dir)
            .arg("--entry")
            .arg(&provider.entry)
            .arg("--entry-id")
            .arg(&provider.entry_id)
            .arg("--kind")
            .arg("launcher_provider")
            .arg("--settings-json")
            .arg(&settings_json);
        if let Some(service) = &provider.service {
            command
                .arg("--service-id")
                .arg(&service.id)
                .arg("--service-entry")
                .arg(&service.entry);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| Error::io("cannot start weyriva-luau-host", &error))?;
        let Some(stdin) = child.stdin.take() else {
            terminate_child_best_effort(&mut child);
            return Err(Error::new("host_start_failed", "host stdin was not piped"));
        };
        let Some(stdout) = child.stdout.take() else {
            terminate_child_best_effort(&mut child);
            return Err(Error::new("host_start_failed", "host stdout was not piped"));
        };
        let mut session = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: 0,
            process_control,
            startup_actions: Vec::new(),
        };
        let ready: HostEvent = session.read_json(HOST_TIMEOUT)?;
        if ready.protocol != HOST_PROTOCOL || ready.event != "ready" {
            let startup_error = ready.error.map_or_else(
                || {
                    Error::new(
                        "host_protocol",
                        "plugin host did not emit the expected ready event",
                    )
                },
                |error| match error.details {
                    Some(details) => Error::with_details(error.code, error.message, details),
                    None => Error::new(error.code, error.message),
                },
            );
            session.terminate()?;
            return Err(startup_error);
        }
        session.startup_actions = ready.actions;
        Ok(session)
    }

    pub fn take_startup_actions(&mut self) -> JsonValue {
        JsonValue::Array(self.startup_actions.drain(..).collect())
    }

    /// Polls the child without blocking.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when child status cannot be inspected.
    pub fn has_exited(&mut self) -> Result<bool> {
        self.process_control
            .try_wait(&mut self.child)
            .map(|status| status.is_some())
            .map_err(|error| process_error("try_wait", "cannot inspect plugin host", &error))
    }

    /// Sends one bounded protocol request.
    ///
    /// # Errors
    ///
    /// Returns structured host, framing, I/O, or timeout failures.
    pub fn request(&mut self, method: &str, params: JsonValue) -> Result<JsonValue> {
        if self.has_exited()? {
            return Err(Error::new("host_unavailable", "plugin host is not running"));
        }
        self.request_id = self.request_id.saturating_add(1);
        let request = HostRequest {
            protocol: HOST_PROTOCOL.to_owned(),
            id: self.request_id,
            method: method.to_owned(),
            params,
        };
        let mut bytes = serde_json::to_vec(&request)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_LINE_BYTES {
            return Err(Error::new(
                "host_protocol",
                "plugin host request exceeds 64 KiB",
            ));
        }
        self.stdin
            .write_all(&bytes)
            .and_then(|()| self.stdin.flush())
            .map_err(|error| Error::io("cannot write plugin host request", &error))?;
        let response: HostResponse = self.read_json(HOST_TIMEOUT)?;
        if response.protocol != HOST_PROTOCOL || response.id != self.request_id {
            return Err(Error::new(
                "host_protocol",
                "plugin host response protocol or id does not match",
            ));
        }
        if let Some(error) = response.error {
            return Err(Error::new(error.code, error.message));
        }
        response
            .result
            .ok_or_else(|| Error::new("host_protocol", "plugin host response has no result"))
    }

    /// Requests graceful shutdown and returns only confirmed callback evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when the shutdown response is malformed.
    pub fn shutdown(&mut self) -> Result<JsonValue> {
        if self.has_exited()? {
            return Ok(json!({"onExit": false, "actions": []}));
        }
        let result = self.request("shutdown", json!({}))?;
        let on_exit = result
            .get("exit_callback_called")
            .and_then(JsonValue::as_bool)
            .ok_or_else(|| Error::new("host_protocol", "shutdown lacks callback evidence"))?;
        let actions = result
            .get("actions")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| Error::new("host_protocol", "shutdown actions are invalid"))?;
        if !wait_child(&mut self.child, self.process_control.as_ref(), HOST_TIMEOUT)? {
            self.terminate()?;
        }
        Ok(json!({"onExit": on_exit, "actions": actions}))
    }

    /// Forces the host to stop and returns only after its exit is confirmed.
    ///
    /// # Errors
    ///
    /// Reports the exact `try_wait`, `kill`, or `wait` operation that prevented
    /// termination from being proved.
    pub fn terminate(&mut self) -> Result<()> {
        match self.process_control.try_wait(&mut self.child) {
            Ok(Some(_status)) => return Ok(()),
            Ok(None) => {}
            Err(error) => {
                return Err(process_error(
                    "try_wait",
                    "cannot inspect plugin host before termination",
                    &error,
                ));
            }
        }
        if let Err(kill_error) = self.process_control.kill(&mut self.child) {
            return match self.process_control.try_wait(&mut self.child) {
                Ok(Some(_status)) => Ok(()),
                Ok(None) => Err(process_error(
                    "kill",
                    "cannot terminate plugin host",
                    &kill_error,
                )),
                Err(recheck_error) => Err(Error::with_details(
                    "host_termination_failed",
                    format!("cannot terminate plugin host: {kill_error}"),
                    json!({
                        "stage": "kill",
                        "io_error": kill_error.to_string(),
                        "recheck_error": recheck_error.to_string(),
                    }),
                )),
            };
        }
        self.process_control
            .wait(&mut self.child)
            .map(|_status| ())
            .map_err(|error| process_error("wait", "cannot reap terminated plugin host", &error))
    }

    fn read_json<T: serde::de::DeserializeOwned>(&mut self, timeout: Duration) -> Result<T> {
        let mut line = Vec::new();
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::new("host_timeout", "plugin host timed out"));
            }
            let mut descriptors = [PollFd::new(
                self.stdout.get_ref().as_fd(),
                PollFlags::POLLIN,
            )];
            let milliseconds = u16::try_from(remaining.as_millis().max(1)).unwrap_or(u16::MAX);
            let ready = poll(&mut descriptors, milliseconds).map_err(|error| {
                Error::new("host_protocol", format!("host poll failed: {error}"))
            })?;
            if ready == 0 {
                return Err(Error::new("host_timeout", "plugin host timed out"));
            }
            let available = self
                .stdout
                .fill_buf()
                .map_err(|error| Error::io("cannot read plugin host response", &error))?;
            if available.is_empty() {
                return Err(Error::new(
                    "host_exited",
                    "plugin host exited without a response",
                ));
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(available.len(), |index| index + 1);
            if line.len().saturating_add(consumed) > MAX_HOST_RESPONSE_BYTES {
                return Err(Error::new(
                    "host_protocol",
                    "plugin host response exceeds 1 MiB",
                ));
            }
            line.extend_from_slice(&available[..consumed]);
            self.stdout.consume(consumed);
            if newline.is_some() {
                break;
            }
        }
        serde_json::from_slice(&line)
            .map_err(|error| Error::new("host_protocol", format!("invalid host JSON: {error}")))
    }
}

impl Drop for HostSession {
    fn drop(&mut self) {
        terminate_child_best_effort(&mut self.child);
    }
}

fn wait_child(
    child: &mut Child,
    process_control: &dyn ProcessControl,
    timeout: Duration,
) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if process_control
            .try_wait(child)
            .map_err(|error| process_error("try_wait", "cannot inspect plugin host", &error))?
            .is_some()
        {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_child_best_effort(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn process_error(stage: &str, context: &str, error: &std::io::Error) -> Error {
    Error::with_details(
        "host_termination_failed",
        format!("{context}: {error}"),
        json!({"stage": stage, "io_error": error.to_string()}),
    )
}
