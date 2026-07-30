use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{Value as JsonValue, json};
use weyriva_luau_host::protocol::PROTOCOL_VERSION;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(4);
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct HostProcess {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: Receiver<String>,
    reader: Option<JoinHandle<()>>,
    _plugin_dir: Option<TempDir>,
}

impl HostProcess {
    pub(crate) fn start(entry: &str, settings: impl Into<JsonValue>) -> Self {
        Self::start_in(fixture_dir(), entry, settings)
    }

    pub(crate) fn start_in(
        plugin_dir: PathBuf,
        entry: &str,
        settings: impl Into<JsonValue>,
    ) -> Self {
        Self::spawn(plugin_dir, entry, None, settings, None)
    }

    pub(crate) fn start_with_service(
        launcher_source: &str,
        service_source: &str,
        settings: impl Into<JsonValue>,
    ) -> Self {
        let plugin_dir = TempDir::new("service");
        fs::write(plugin_dir.path.join("launcher.luau"), launcher_source)
            .expect("launcher fixture should be written");
        fs::write(plugin_dir.path.join("service.luau"), service_source)
            .expect("service fixture should be written");
        Self::spawn(
            plugin_dir.path.clone(),
            "launcher.luau",
            Some(("sync", "service.luau")),
            settings,
            Some(plugin_dir),
        )
    }

    fn spawn(
        plugin_dir: PathBuf,
        entry: &str,
        service: Option<(&str, &str)>,
        settings: impl Into<JsonValue>,
        plugin_dir_guard: Option<TempDir>,
    ) -> Self {
        let settings = settings.into();
        let mut command = Command::new(env!("CARGO_BIN_EXE_weyriva-luau-host"));
        command
            .arg("--plugin-dir")
            .arg(plugin_dir)
            .arg("--entry")
            .arg(entry)
            .arg("--entry-id")
            .arg("main")
            .arg("--kind")
            .arg("launcher_provider")
            .arg("--settings-json")
            .arg(settings.to_string());
        if let Some((service_id, service_entry)) = service {
            command
                .arg("--service-id")
                .arg(service_id)
                .arg("--service-entry")
                .arg(service_entry);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("host process should spawn");
        let input = child.stdin.take().expect("host stdin should be piped");
        let output = child.stdout.take().expect("host stdout should be piped");
        let (sender, receiver) = mpsc::channel();
        let reader = thread::spawn(move || {
            let reader = BufReader::new(output);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input: BufWriter::new(input),
            output: receiver,
            reader: Some(reader),
            _plugin_dir: plugin_dir_guard,
        }
    }

    pub(crate) fn read_line(&self) -> String {
        self.output
            .recv_timeout(RESPONSE_TIMEOUT)
            .expect("host should produce a bounded response")
    }

    pub(crate) fn read(&self) -> JsonValue {
        serde_json::from_str(&self.read_line()).expect("host output should be JSON")
    }

    pub(crate) fn ready(&self) -> JsonValue {
        self.read()
    }

    pub(crate) fn request(
        &mut self,
        id: impl Into<JsonValue>,
        method: &str,
        params: impl Into<JsonValue>,
    ) -> JsonValue {
        let line = self.request_line(id, method, params);
        serde_json::from_str(&line).expect("host output should be JSON")
    }

    pub(crate) fn request_line(
        &mut self,
        id: impl Into<JsonValue>,
        method: &str,
        params: impl Into<JsonValue>,
    ) -> String {
        let id = id.into();
        let params = params.into();
        let request = json!({
            "protocol": PROTOCOL_VERSION,
            "id": id,
            "method": method,
            "params": params
        });
        writeln!(self.input, "{request}").expect("request should write");
        self.input.flush().expect("request should flush");
        self.read_line()
    }

    pub(crate) fn write_raw_line(&mut self, bytes: &[u8]) -> JsonValue {
        self.input
            .write_all(bytes)
            .expect("raw request should write");
        self.input
            .write_all(b"\n")
            .expect("raw request newline should write");
        self.input.flush().expect("raw request should flush");
        self.read()
    }

    pub(crate) fn wait_for_success(&mut self) {
        let status = self.child.wait().expect("host process should exit");
        assert!(status.success(), "host exited with {status}");
    }
}

impl Drop for HostProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

pub(crate) struct TempDir {
    pub(crate) path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "weyriva-luau-host-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary fixture directory should be created");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn fixture_dir() -> PathBuf {
    canonical(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/plugins/v5-launcher-api3"),
    )
}

pub(crate) fn canonical(path: &Path) -> PathBuf {
    path.canonicalize()
        .unwrap_or_else(|error| panic!("cannot canonicalize {}: {error}", display(path)))
}

fn display(path: &Path) -> &str {
    path.as_os_str().to_str().unwrap_or("<non-UTF-8>")
}
