use serde_json::json;
use weyriva_luau_host::protocol::{MAX_REQUEST_LINE_BYTES, PROTOCOL_VERSION};

use crate::support::{HostProcess, fixture_dir};

#[test]
fn ready_event_advertises_an_incomplete_launcher_subset() {
    let host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));

    let ready = host.ready();

    assert_eq!(
        ready,
        json!({
            "protocol": PROTOCOL_VERSION,
            "event": "ready",
            "kind": "launcher_provider",
            "capability": {
                "api": 3,
                "subset": "launcher_provider",
                "complete": false
            }
        })
    );
}

#[test]
fn query_exercises_luau_config_relative_json_and_state_watch() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": " cactus "}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": "term"}));

    assert_eq!(
        response["result"],
        json!({
            "query": "term",
            "results": [{
                "id": "terminal",
                "title": "Fixture Terminal · term",
                "subtitle": "cactus · query #1",
                "glyph": "W",
                "category": "development"
            }],
            "actions": []
        })
    );
}

#[test]
fn activate_returns_side_effects_as_host_actions() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();

    let response = host.request(json!("activate"), "activate", json!({"id": "terminal"}));

    assert_eq!(
        response["result"]["actions"],
        json!([
            {
                "type": "clipboard",
                "text": "fixture:terminal",
                "mime": "text/plain"
            },
            {
                "type": "notify",
                "title": "Weyriva launcher fixture",
                "message": format!(
                    "Activated terminal from {}",
                    fixture_dir().display()
                )
            },
            {
                "type": "launcher_set_query",
                "query": ""
            }
        ])
    );
}

#[test]
fn ipc_returns_plain_state_and_a_launcher_action() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();
    host.request(json!(1), "query", json!({"query": ""}));

    let response = host.request(
        json!(2),
        "ipc",
        json!({"event": "status", "payload": {"query": "next"}}),
    );

    assert_eq!(
        response["result"],
        json!({
            "value": {"queryCount": 1, "accent": "cactus"},
            "actions": [{"type": "launcher_set_query", "query": "next"}]
        })
    );
}

#[test]
fn service_runtime_shares_json_state_and_routes_updates_and_ipc_by_entry() {
    let mut host = HostProcess::start_with_service(
        r#"
local latest = 0
local phase = "unloaded"
noctalia.state.watch("count", function(value)
    latest = value
end)
noctalia.state.watch("phase", function(value)
    phase = value
end)

function onQuery(text)
    launcher.setResults(text, {{ id = "count", title = `{phase}:{latest}` }})
end

function onIpc(event, payload)
    return { event = event, count = noctalia.state.get("count"), payload = payload }
end
"#,
        r#"
local updates = 0
noctalia.state.set("phase", "loaded")

function update()
    updates += 1
    noctalia.state.set("count", updates)
    noctalia.notify("service", `update:{updates}`)
end

function onIpc(event, payload)
    noctalia.state.set("count", payload.count)
    return { event = event, count = noctalia.state.get("count") }
end
"#,
        json!({}),
    );

    let ready = host.ready();
    let query = host.request(json!(1), "query", json!({"query": ""}));
    let update = host.request(json!(2), "update", json!({"entry": "sync"}));
    let service_ipc = host.request(
        json!(3),
        "ipc",
        json!({"entry": "sync", "event": "set", "payload": {"count": 7}}),
    );
    let launcher_ipc = host.request(
        json!(4),
        "ipc",
        json!({"entry": "main", "event": "status", "payload": {"ok": true}}),
    );

    assert_eq!(
        ready["actions"],
        json!([{"type": "notify", "title": "service", "message": "update:1"}])
    );
    assert_eq!(query["result"]["results"][0]["title"], "loaded:1");
    assert_eq!(
        update["result"],
        json!({
            "updated": "sync",
            "actions": [{"type": "notify", "title": "service", "message": "update:2"}]
        })
    );
    assert_eq!(
        service_ipc["result"]["value"],
        json!({"event": "set", "count": 7})
    );
    assert_eq!(
        launcher_ipc["result"]["value"],
        json!({"event": "status", "count": 7, "payload": {"ok": true}})
    );
}

#[test]
fn service_without_required_update_fails_before_ready() {
    let host = HostProcess::start_with_service(
        "function onQuery(text) launcher.setResults(text, {}) end\n",
        "function onIpc(event, payload) return payload end\n",
        json!({}),
    );

    let fatal = host.ready();

    assert_eq!(fatal["event"], "fatal");
    assert_eq!(fatal["error"]["code"], "missing_callback");
}

#[test]
fn service_methods_reject_unknown_entries_and_extra_parameters() {
    let mut host = HostProcess::start_with_service(
        "function onQuery(text) launcher.setResults(text, {}) end\n",
        "function update() end\nfunction onIpc(event, payload) return payload end\n",
        json!({}),
    );
    host.ready();

    let launcher_update = host.request(json!(1), "update", json!({"entry": "main"}));
    let unknown_ipc = host.request(
        json!(2),
        "ipc",
        json!({"entry": "missing", "event": "status", "payload": {}}),
    );
    let extra_update = host.request(
        json!(3),
        "update",
        json!({"entry": "sync", "unexpected": true}),
    );

    assert_eq!(launcher_update["error"]["code"], "unknown_entry");
    assert_eq!(unknown_ipc["error"]["code"], "unknown_entry");
    assert_eq!(extra_update["error"]["code"], "invalid_params");
}

#[test]
fn shutdown_calls_optional_exit_callback_and_stops_the_process() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();

    let response = host.request(json!(1), "shutdown", json!({}));
    host.wait_for_success();

    assert_eq!(
        response["result"],
        json!({
            "exit_callback_called": true,
            "actions": [{
                "type": "notify",
                "title": "Weyriva launcher fixture",
                "message": "Fixture exit 0:shutdown"
            }]
        })
    );
}

#[test]
fn launcher_without_optional_exit_callback_shuts_down_cleanly() {
    let mut host = HostProcess::start("large-response.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "shutdown", json!({}));
    host.wait_for_success();

    assert_eq!(
        response["result"],
        json!({
            "exit_callback_called": false,
            "actions": []
        })
    );
}

#[test]
fn unknown_ipc_event_emits_a_titled_error_notification() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();

    let response = host.request(json!(1), "ipc", json!({"event": "unknown", "payload": {}}));

    assert_eq!(
        response["result"]["actions"],
        json!([{
            "type": "notify_error",
            "title": "Weyriva launcher fixture",
            "message": "Unknown fixture event: unknown"
        }])
    );
}

#[test]
fn unknown_protocol_method_returns_a_structured_error_without_stopping_host() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();

    let rejected = host.request(json!(1), "unknown", json!({}));
    let recovered = host.request(json!(2), "query", json!({"query": "notes"}));

    assert_eq!(
        (
            rejected["error"]["code"].as_str(),
            recovered["result"]["results"][0]["id"].as_str()
        ),
        (Some("unknown_method"), Some("notes"))
    );
}

#[test]
fn invalid_method_params_return_a_structured_error() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": 42}));

    assert_eq!(response["error"]["code"], "invalid_params");
}

#[test]
fn infinite_loop_is_interrupted_and_a_fresh_process_recovers() {
    let mut stuck = HostProcess::start("infinite-loop.luau", json!({}));
    stuck.ready();
    let timeout = stuck.request(json!(1), "query", json!({"query": ""}));
    let mut recovered = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    recovered.ready();
    let response = recovered.request(json!(2), "query", json!({"query": "notes"}));

    assert_eq!(
        (
            timeout["error"]["code"].as_str(),
            response["result"]["results"][0]["id"].as_str()
        ),
        (Some("execution_limit"), Some("notes"))
    );
}

#[test]
fn oversized_request_is_drained_before_the_next_request() {
    let mut host = HostProcess::start("launcher.luau", json!({"accent": "cactus"}));
    host.ready();
    let oversized = vec![b'x'; MAX_REQUEST_LINE_BYTES + 1];

    let rejected = host.write_raw_line(&oversized);
    let recovered = host.request(json!(2), "query", json!({"query": "notes"}));

    assert_eq!(
        (
            rejected["error"]["code"].as_str(),
            recovered["result"]["results"][0]["id"].as_str()
        ),
        (Some("request_too_large"), Some("notes"))
    );
}
