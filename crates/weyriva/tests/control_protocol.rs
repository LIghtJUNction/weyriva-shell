mod common;
#[path = "support/process.rs"]
mod process;

use std::ffi::{OsStr, OsString};
use std::sync::Arc;

use process::RecordingProcess;
use serde_json::{Value as JsonValue, json};
use tempfile::{TempDir, tempdir};
use weyriva::Broker;
use weyriva::ipc::{BUILTIN_METHODS, Dispatcher, process_line};
use weyriva::niri::NiriClient;
use weyriva::shell::ShellController;

struct Harness {
    _temporary: TempDir,
    broker: Broker,
    shell: ShellController,
    niri: NiriClient,
    process: Arc<RecordingProcess>,
}

impl Harness {
    fn new() -> Self {
        let temporary = tempdir().expect("temporary directory should be created");
        let process = Arc::new(RecordingProcess::default());
        let shell = ShellController::new(std::iter::empty(), process.clone(), process.clone());
        let niri = NiriClient::new(process.clone());
        let broker = Broker::with_host(
            common::paths(&temporary),
            temporary.path().join("unused-host"),
        );
        Self {
            _temporary: temporary,
            broker,
            shell,
            niri,
            process,
        }
    }

    fn line(&mut self, bytes: &[u8]) -> JsonValue {
        let mut dispatcher = Dispatcher::new(&mut self.broker, &self.shell, &self.niri);
        serde_json::from_slice(&process_line(bytes, &mut dispatcher))
            .expect("response should be JSON")
    }

    fn request(&mut self, method: &str, params: &JsonValue) -> JsonValue {
        let line = serde_json::to_vec(&json!({
            "protocol": 1,
            "id": "test",
            "method": method,
            "params": params,
        }))
        .expect("request should encode");
        let mut framed = line;
        framed.push(b'\n');
        self.line(&framed)
    }
}

#[test]
fn ping_envelope_is_compact_newline_utf8() {
    let mut harness = Harness::new();
    let response = harness.request("weyriva.ping", &json!({}));

    assert_eq!(
        response,
        json!({"id": "test", "result": {"pong": true, "protocol": 1}})
    );
}

#[test]
fn malformed_and_framing_errors_are_structured() {
    let mut harness = Harness::new();
    let malformed = harness.line(b"{bad\n");
    let non_line = harness.line(b"{}");
    let oversized = harness.line(&vec![b'x'; 65_538]);

    assert_eq!(malformed["error"]["code"], "parse_error");
    assert_eq!(non_line["error"]["code"], "request_too_large");
    assert_eq!(oversized["error"]["code"], "request_too_large");
}

#[test]
fn request_shape_rejects_non_object_composite_id_and_empty_method() {
    let mut harness = Harness::new();
    let composite = harness.line(
        br#"{"protocol":1,"id":[],"method":"weyriva.ping"}
"#,
    );
    assert_eq!(composite["id"], json!([]));
    assert_eq!(composite["error"]["code"], "invalid_request");

    let cases = [
        (b"[]\n".as_slice(), "invalid_request"),
        (
            br#"{"protocol":1,"id":1,"method":""}
"#,
            "invalid_request",
        ),
        (
            br#"{"protocol":2,"id":1,"method":"weyriva.ping"}
"#,
            "unsupported_protocol",
        ),
    ];

    for (line, expected) in cases {
        assert_eq!(harness.line(line)["error"]["code"], expected);
    }
}

#[test]
fn scalar_ids_and_missing_params_are_accepted() {
    for id in [json!(null), json!(true), json!(7), json!("request")] {
        let mut harness = Harness::new();
        let mut line = serde_json::to_vec(&json!({
            "protocol": 1,
            "id": id,
            "method": "weyriva.ping",
        }))
        .expect("request should encode");
        line.push(b'\n');
        let response = harness.line(&line);
        assert!(response.get("result").is_some(), "{response}");
    }
}

#[test]
fn methods_are_ordered_and_legacy_lane_is_retired() {
    let mut harness = Harness::new();
    let response = harness.request("weyriva.methods", &JsonValue::Null);

    assert_eq!(response["result"]["builtin"], json!(BUILTIN_METHODS));
    assert_eq!(response["result"]["plugin"], json!([]));
    assert!(!BUILTIN_METHODS.contains(&"weyriva.plugin.list"));
    assert!(!BUILTIN_METHODS.contains(&"weyriva.plugin.reload"));
}

#[test]
fn no_param_builtins_reject_every_nonempty_shape() {
    for method in [
        "weyriva.ping",
        "weyriva.info",
        "weyriva.methods",
        "weyriva.niri.outputs",
        "weyriva.niri.windows",
        "weyriva.launcher.open",
        "weyriva.notifications.dismiss_all",
        "weyriva.panel.toggle",
        "weyriva.panel.reload",
    ] {
        let mut harness = Harness::new();
        let response = harness.request(method, &json!({"unexpected": true}));
        assert_eq!(response["error"]["code"], "invalid_params", "{method}");
    }
}

#[test]
fn dnd_accepts_only_empty_or_exact_boolean() {
    for valid in [
        JsonValue::Null,
        json!({}),
        json!({"enabled": true}),
        json!({"enabled": false}),
    ] {
        let mut harness = Harness::new();
        harness.process.push(0, "ok\n", "");
        assert!(
            harness
                .request("weyriva.notifications.dnd", &valid)
                .get("result")
                .is_some()
        );
    }
    for invalid in [
        json!({"enabled": "yes"}),
        json!({"enabled": true, "extra": false}),
        json!([]),
    ] {
        let mut harness = Harness::new();
        assert_eq!(
            harness.request("weyriva.notifications.dnd", &invalid)["error"]["code"],
            "invalid_params"
        );
    }
}

#[test]
fn niri_and_shell_builtins_use_fixed_argv() {
    let mut harness = Harness::new();
    harness.process.push(0, "{}\n", "");
    harness.process.push(0, "opened\n", "");

    let outputs = harness.request("weyriva.niri.outputs", &json!({}));
    let launcher = harness.request("weyriva.launcher.open", &json!({}));
    let commands = harness.process.commands();

    assert_eq!(outputs["result"], json!({}));
    assert_eq!(launcher["result"]["command"], "route");
    assert_eq!(commands[0].program, OsStr::new("niri"));
    assert_eq!(
        commands[0].arguments,
        ["msg", "-j", "outputs"].map(OsString::from).to_vec()
    );
    assert_eq!(commands[1].program, OsStr::new("quickshell"));
    assert_eq!(
        commands[1].arguments[2..],
        ["ipc", "call", "weyriva", "route", "launcher"].map(OsString::from)
    );
}

#[test]
fn unknown_method_is_structured() {
    let mut harness = Harness::new();
    let response = harness.request("weyriva.missing", &json!({}));

    assert_eq!(response["error"]["code"], "method_not_found");
}
