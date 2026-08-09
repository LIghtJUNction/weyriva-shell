use std::ffi::OsString;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::error::{Error, ErrorBody, Result};
use crate::host_session::{ProcessControl, SystemProcessControl};
use crate::model::{V4_COMPATIBILITY_PROFILE, V4Runtime};
use crate::process::{CommandSpec, ProcessRunner, SystemProcess, command_text, os};

const V4_HOST_PROTOCOL: &str = "weyriva-v4-host/1";
const HOST_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
static RUNTIME_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct V4HostSession {
    child: Child,
    quickshell: PathBuf,
    runtime_dir: PathBuf,
    handler_target: String,
    request_id: u64,
    process_control: Arc<dyn ProcessControl>,
    process_runner: Arc<dyn ProcessRunner>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap<'a> {
    protocol: &'static str,
    profile: &'static str,
    plugin_id: &'a str,
    plugin_dir: &'a Path,
    main: &'a str,
    launcher_provider: &'a Path,
    handler_target: &'a str,
}

#[derive(Serialize)]
struct Request<'a> {
    protocol: &'static str,
    id: u64,
    method: &'a str,
    params: JsonValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    ok: bool,
    id: Option<u64>,
    #[serde(default)]
    result: Option<JsonValue>,
    #[serde(default)]
    error: Option<ErrorBody>,
}

impl V4HostSession {
    /// Starts one pinned v4 plugin in its own Quickshell process.
    ///
    /// # Errors
    ///
    /// Returns an error when private bootstrap creation, process launch, or the
    /// bounded ready exchange fails.
    pub fn start(
        quickshell: &Path,
        host_root: &Path,
        runtime_root: &Path,
        plugin_id: &str,
        plugin_dir: &Path,
        runtime: &V4Runtime,
    ) -> Result<Self> {
        Self::start_with_boundaries(
            quickshell,
            host_root,
            runtime_root,
            plugin_id,
            plugin_dir,
            runtime,
            Arc::new(SystemProcessControl),
            Arc::new(SystemProcess),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the testable process boundary keeps launch authority explicit"
    )]
    pub(crate) fn start_with_boundaries(
        quickshell: &Path,
        host_root: &Path,
        runtime_root: &Path,
        plugin_id: &str,
        plugin_dir: &Path,
        runtime: &V4Runtime,
        process_control: Arc<dyn ProcessControl>,
        process_runner: Arc<dyn ProcessRunner>,
    ) -> Result<Self> {
        validate_host_root(host_root)?;
        let runtime_dir = create_runtime_dir(runtime_root, plugin_id)?;
        let translated_launcher = translate_launcher(plugin_dir, runtime, &runtime_dir)?;
        let bootstrap = Bootstrap {
            protocol: V4_HOST_PROTOCOL,
            profile: V4_COMPATIBILITY_PROFILE,
            plugin_id,
            plugin_dir,
            main: &runtime.main,
            launcher_provider: &translated_launcher,
            handler_target: &runtime.handler_target,
        };
        if let Err(error) = write_bootstrap(&runtime_dir, &bootstrap) {
            let _ = remove_runtime_tree(&runtime_dir);
            return Err(error);
        }
        let import_path = host_root.join("facade");
        let mut command = Command::new(quickshell);
        command
            .arg("--path")
            .arg(host_root)
            .env("XDG_RUNTIME_DIR", &runtime_dir)
            .env("QML2_IMPORT_PATH", prepend_import_path(&import_path))
            .env("QML_IMPORT_PATH", prepend_qml_import_path(&import_path))
            .env("QT_QPA_PLATFORM", "offscreen")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = remove_runtime_tree(&runtime_dir);
                return Err(Error::io("cannot start v4 Quickshell host", &error));
            }
        };
        let mut session = Self {
            child,
            quickshell: quickshell.to_path_buf(),
            runtime_dir,
            handler_target: runtime.handler_target.clone(),
            request_id: 0,
            process_control,
            process_runner,
        };
        if let Err(error) = session.wait_ready() {
            let termination = session.terminate();
            return match termination {
                Ok(()) => Err(error),
                Err(termination) => Err(Error::with_details(
                    "host_termination_failed",
                    "v4 host startup failed and termination was not confirmed",
                    json!({
                        "startup": error.body(),
                        "termination": termination.body(),
                    }),
                )),
            };
        }
        Ok(session)
    }

    #[must_use]
    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    /// Sends one bounded strict-JSON request through the owned host IPC target.
    ///
    /// # Errors
    ///
    /// Returns an error when the child exited, the IPC command fails, or the
    /// response violates the v4 host envelope.
    pub fn request(&mut self, method: &str, params: JsonValue) -> Result<JsonValue> {
        if self.has_exited()? {
            return Err(Error::new(
                "host_unavailable",
                "v4 plugin host is not running",
            ));
        }
        if method == "ipc" {
            return self.plugin_ipc(&params);
        }
        self.request_id = self.request_id.saturating_add(1);
        let request = Request {
            protocol: V4_HOST_PROTOCOL,
            id: self.request_id,
            method,
            params,
        };
        let encoded = serde_json::to_string(&request)?;
        if encoded.len() > MAX_MESSAGE_BYTES {
            return Err(Error::new("host_protocol", "v4 host request exceeds 1 MiB"));
        }
        let response = self.invoke("weyriva-v4-host", "request", &[&encoded])?;
        decode_response(&response, self.request_id)
    }

    pub fn take_startup_actions(&mut self) -> JsonValue {
        JsonValue::Array(Vec::new())
    }

    /// Returns whether the isolated process has exited.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when process status cannot be inspected.
    pub fn has_exited(&mut self) -> Result<bool> {
        self.process_control
            .try_wait(&mut self.child)
            .map(|status| status.is_some())
            .map_err(|error| Error::io("cannot inspect v4 plugin host", &error))
    }

    /// Gracefully stops the host and returns shutdown evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when shutdown or confirmed termination fails.
    pub fn shutdown(&mut self) -> Result<JsonValue> {
        if self.has_exited()? {
            self.cleanup_runtime()?;
            return Ok(json!({"onExit": false, "actions": []}));
        }
        let evidence = self.request("shutdown", json!({}))?;
        self.terminate()?;
        Ok(evidence)
    }

    /// Forces the process to stop, proves exit, and removes its private runtime.
    ///
    /// # Errors
    ///
    /// Returns an error when status, kill, wait, or runtime cleanup fails.
    pub fn terminate(&mut self) -> Result<()> {
        match self.process_control.try_wait(&mut self.child) {
            Ok(Some(_)) => {}
            Ok(None) => {
                self.process_control
                    .kill(&mut self.child)
                    .map_err(|error| Error::io("cannot terminate v4 plugin host", &error))?;
                self.process_control
                    .wait(&mut self.child)
                    .map_err(|error| Error::io("cannot reap v4 plugin host", &error))?;
            }
            Err(error) => return Err(Error::io("cannot inspect v4 plugin host", &error)),
        }
        self.cleanup_runtime()
    }

    fn wait_ready(&mut self) -> Result<()> {
        let deadline = Instant::now() + HOST_TIMEOUT;
        loop {
            match self.request("ready", json!({})) {
                Ok(result) if result.get("ready").and_then(JsonValue::as_bool) == Some(true) => {
                    return Ok(());
                }
                Ok(_) => {
                    return Err(Error::new(
                        "host_protocol",
                        "v4 host ready response is invalid",
                    ));
                }
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }

    fn invoke(&self, target: &str, method: &str, arguments: &[&str]) -> Result<String> {
        let mut argv = vec![
            os("ipc"),
            os("--pid"),
            OsString::from(self.child.id().to_string()),
            os("call"),
            os(target),
            os(method),
        ];
        argv.extend(arguments.iter().map(os));
        let environment = std::env::vars_os()
            .chain(std::iter::once((
                OsString::from("XDG_RUNTIME_DIR"),
                self.runtime_dir.as_os_str().to_os_string(),
            )))
            .collect::<Vec<_>>();
        let command = CommandSpec::new(self.quickshell.as_os_str(), argv)
            .with_timeout(HOST_TIMEOUT)
            .with_environment(environment);
        let output = self.process_runner.run(&command)?;
        if output.code != 0 {
            return Err(Error::new(
                "host_protocol",
                format!("v4 host IPC failed: {}", command_text(&output)),
            ));
        }
        let text = command_text(&output);
        if text.len() > MAX_MESSAGE_BYTES {
            return Err(Error::new(
                "host_protocol",
                "v4 host response exceeds 1 MiB",
            ));
        }
        Ok(text)
    }

    fn plugin_ipc(&mut self, params: &JsonValue) -> Result<JsonValue> {
        let object = params
            .as_object()
            .ok_or_else(|| Error::new("invalid_params", "v4 IPC params must be an object"))?;
        if object.len() != 3
            || !object.contains_key("entry")
            || !object.contains_key("event")
            || !object.contains_key("payload")
        {
            return Err(Error::new("invalid_params", "v4 IPC params are invalid"));
        }
        if object.get("entry").and_then(JsonValue::as_str) != Some("launcher") {
            return Err(Error::new(
                "invalid_params",
                "v4 IPC entry must be launcher",
            ));
        }
        let event = object
            .get("event")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| Error::new("invalid_params", "v4 IPC event must be a string"))?;
        if event != "toggle"
            || !object
                .get("payload")
                .is_some_and(|payload| payload.as_object().is_some_and(serde_json::Map::is_empty))
        {
            return Err(Error::new(
                "unsupported_ipc",
                "pinned v4 host supports only toggle with an empty payload",
            ));
        }
        let raw_value = self.invoke(&self.handler_target, event, &[])?;
        let value = decode_ipc_value(&raw_value);
        let actions = self.request("drain_actions", json!({}))?;
        let actions = actions
            .get("actions")
            .cloned()
            .ok_or_else(|| Error::new("host_protocol", "v4 IPC actions are invalid"))?;
        Ok(json!({"value": value, "actions": actions}))
    }

    fn cleanup_runtime(&self) -> Result<()> {
        remove_runtime_tree(&self.runtime_dir)
    }
}

impl Drop for V4HostSession {
    fn drop(&mut self) {
        if self
            .process_control
            .try_wait(&mut self.child)
            .ok()
            .flatten()
            .is_none()
        {
            let _ = self.process_control.kill(&mut self.child);
            let _ = self.process_control.wait(&mut self.child);
        }
        let _ = self.cleanup_runtime();
    }
}

fn validate_host_root(host_root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(host_root.join("shell.qml"))
        .map_err(|error| Error::io("cannot inspect owned v4 host", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::new(
            "host_unavailable",
            "owned v4 host shell.qml is not a regular file",
        ));
    }
    Ok(())
}

fn create_runtime_dir(root: &Path, plugin_id: &str) -> Result<PathBuf> {
    fs::create_dir_all(root).map_err(|error| Error::io("cannot create v4 runtime root", &error))?;
    let unique = RUNTIME_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = root.join(format!("v4-{plugin_id}-{}-{unique}", std::process::id()));
    DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .map_err(|error| Error::io("cannot create private v4 runtime", &error))?;
    Ok(path)
}

fn write_bootstrap(path: &Path, bootstrap: &Bootstrap<'_>) -> Result<()> {
    let encoded = serde_json::to_vec(bootstrap)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path.join("bootstrap.json"))
        .map_err(|error| Error::io("cannot create v4 bootstrap", &error))?;
    file.write_all(&encoded)
        .and_then(|()| file.sync_all())
        .map_err(|error| Error::io("cannot persist v4 bootstrap", &error))
}

fn translate_launcher(
    plugin_dir: &Path,
    runtime: &V4Runtime,
    runtime_dir: &Path,
) -> Result<PathBuf> {
    let source = fs::read_to_string(plugin_dir.join(&runtime.launcher_provider))
        .map_err(|error| Error::io("cannot read v4 launcher source", &error))?;
    if !source.contains("import Quickshell") || !source.contains("Quickshell.execDetached") {
        return Err(Error::new(
            "unsupported_plugin",
            "pinned v4 launcher does not expose the expected Quickshell copy action",
        ));
    }
    let translated = source
        .replace("import Quickshell\n", "import Compat as WeyrivaCompat\n")
        .replace(
            "Quickshell.execDetached",
            "WeyrivaCompat.Quickshell.execDetached",
        );
    if translated == source || translated.contains("import Quickshell\n") {
        return Err(Error::new(
            "unsupported_plugin",
            "pinned v4 launcher import translation failed closed",
        ));
    }
    let path = runtime_dir.join("LauncherProvider.qml");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| Error::io("cannot create translated v4 launcher", &error))?;
    file.write_all(translated.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| Error::io("cannot persist translated v4 launcher", &error))?;
    Ok(path)
}

fn prepend_import_path(facade: &Path) -> OsString {
    let mut paths = vec![facade.to_path_buf()];
    if let Some(existing) = std::env::var_os("QML2_IMPORT_PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap_or_else(|_| facade.as_os_str().to_os_string())
}

fn prepend_qml_import_path(facade: &Path) -> OsString {
    let mut paths = vec![facade.to_path_buf()];
    if let Some(existing) = std::env::var_os("QML_IMPORT_PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).unwrap_or_else(|_| facade.as_os_str().to_os_string())
}

fn decode_response(encoded: &str, expected_id: u64) -> Result<JsonValue> {
    let value: JsonValue = serde_json::from_str(encoded)
        .map_err(|error| Error::new("host_protocol", format!("invalid v4 host JSON: {error}")))?;
    let value = match value {
        JsonValue::String(inner) => serde_json::from_str(&inner).map_err(|error| {
            Error::new(
                "host_protocol",
                format!("invalid nested v4 host JSON: {error}"),
            )
        })?,
        value => value,
    };
    let response: Response = serde_json::from_value(value)
        .map_err(|error| Error::new("host_protocol", format!("invalid v4 response: {error}")))?;
    if response.id != Some(expected_id) {
        return Err(Error::new(
            "host_protocol",
            "v4 response id does not match request",
        ));
    }
    if response.ok {
        if response.error.is_some() {
            return Err(Error::new(
                "host_protocol",
                "successful v4 response contains an error",
            ));
        }
        response
            .result
            .ok_or_else(|| Error::new("host_protocol", "v4 response has no result"))
    } else {
        if response.result.is_some() {
            return Err(Error::new(
                "host_protocol",
                "failed v4 response contains a result",
            ));
        }
        let error = response
            .error
            .ok_or_else(|| Error::new("host_protocol", "v4 error response has no error"))?;
        Err(match error.details {
            Some(details) => Error::with_details(error.code, error.message, details),
            None => Error::new(error.code, error.message),
        })
    }
}

fn decode_ipc_value(raw: &str) -> JsonValue {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        JsonValue::Null
    } else {
        serde_json::from_str(trimmed).unwrap_or_else(|_| JsonValue::String(trimmed.to_owned()))
    }
}

fn remove_runtime_tree(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot inspect v4 runtime", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new("unsafe_state", "v4 runtime path is unsafe"));
    }
    let entries = walkdir::WalkDir::new(path)
        .contents_first(true)
        .into_iter()
        .map(|entry| {
            entry
                .map(walkdir::DirEntry::into_path)
                .map_err(|error| Error::new("io_error", error.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;
    for entry in &entries {
        let metadata = fs::symlink_metadata(entry)
            .map_err(|error| Error::io("cannot inspect v4 runtime entry", &error))?;
        let file_type = metadata.file_type();
        if !file_type.is_symlink()
            && !file_type.is_dir()
            && !file_type.is_file()
            && !file_type.is_socket()
            && !file_type.is_fifo()
        {
            return Err(Error::new(
                "unsafe_state",
                "v4 runtime contains an unsafe entry",
            ));
        }
        if file_type.is_dir() {
            fs::set_permissions(entry, fs::Permissions::from_mode(0o700))
                .map_err(|error| Error::io("cannot unlock v4 runtime directory", &error))?;
        }
    }
    for entry in entries {
        let is_dir = fs::symlink_metadata(&entry)
            .map_err(|error| Error::io("cannot inspect removable v4 runtime entry", &error))?
            .file_type()
            .is_dir();
        if is_dir {
            fs::remove_dir(&entry)
                .map_err(|error| Error::io("cannot remove v4 runtime directory", &error))?;
        } else {
            fs::remove_file(&entry)
                .map_err(|error| Error::io("cannot remove v4 runtime file", &error))?;
        }
    }
    Ok(())
}
