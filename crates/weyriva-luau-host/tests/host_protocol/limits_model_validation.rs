use std::fs;

use serde_json::{Value as JsonValue, json};
use weyriva_luau_host::protocol::{MAX_REQUEST_LINE_BYTES, MAX_RESPONSE_LINE_BYTES};

use crate::support::{HostProcess, TempDir, canonical, fixture_dir};

const REAL_KAOMOJI_JSON_BYTES: usize = 113_724;

#[test]
fn official_kaomoji_sized_json_decodes_and_returns_the_explicit_query() {
    let plugin = TempDir::new("kaomoji-json");
    fs::copy(
        fixture_dir().join("json-size.luau"),
        plugin.path.join("json-size.luau"),
    )
    .expect("JSON fixture should copy");
    let database = format!("[{}]", " ".repeat(REAL_KAOMOJI_JSON_BYTES - 2));
    assert_eq!(database.len(), REAL_KAOMOJI_JSON_BYTES);
    fs::write(plugin.path.join("large-catalog.json"), database)
        .expect("sized JSON fixture should write");
    let mut host = HostProcess::start_in(canonical(&plugin.path), "json-size.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": "official"}));

    assert_eq!(
        response["result"],
        json!({
            "query": "decoded:official",
            "results": [{
                "id": "json-size",
                "title": "0",
                "subtitle": REAL_KAOMOJI_JSON_BYTES.to_string()
            }],
            "actions": []
        })
    );
}

#[test]
fn json_decode_rejects_input_above_the_one_mibibyte_limit() {
    let host = HostProcess::start(
        "json-size.luau",
        json!({"generated_bytes": MAX_RESPONSE_LINE_BYTES - 1}),
    );

    let fatal = host.read();

    assert_eq!(fatal["error"]["code"], "string_limit");
    assert!(
        fatal["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains(&MAX_RESPONSE_LINE_BYTES.to_string()))
    );
}

#[test]
fn plugin_read_rejects_a_file_above_the_one_mibibyte_limit() {
    let plugin = TempDir::new("oversized-plugin-file");
    fs::copy(
        fixture_dir().join("json-size.luau"),
        plugin.path.join("json-size.luau"),
    )
    .expect("JSON fixture should copy");
    fs::write(
        plugin.path.join("large-catalog.json"),
        vec![b' '; MAX_RESPONSE_LINE_BYTES + 1],
    )
    .expect("oversized plugin file should write");
    let host = HostProcess::start_in(canonical(&plugin.path), "json-size.luau", json!({}));

    let fatal = host.read();

    assert_eq!(fatal["error"]["code"], "file_limit");
}

#[test]
fn launcher_set_results_rejects_the_obsolete_one_argument_shape() {
    let mut host = HostProcess::start("invalid-set-results.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "callback_error");
}

#[test]
fn invalid_launcher_model_returns_a_structured_error() {
    let mut host = HostProcess::start("invalid-model.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "invalid_model");
}

#[test]
fn result_count_above_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("result-limit.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "result_limit");
}

#[test]
fn launcher_field_above_string_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("string-limit.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "string_limit");
}

#[test]
fn action_count_above_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("action-limit.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "fixture"}));

    assert_eq!(response["error"]["code"], "action_limit");
}

#[test]
fn clipboard_without_mime_uses_the_documented_text_default() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "default-mime"}));

    assert_eq!(
        response["result"]["actions"],
        json!([{
            "type": "clipboard",
            "text": "fixture",
            "mime": "text/plain"
        }])
    );
}

#[test]
fn unsafe_clipboard_mime_returns_a_structured_error() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "mime"}));

    assert_eq!(response["error"]["code"], "invalid_mime");
}

#[test]
fn clipboard_mime_above_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "mime-limit"}));

    assert_eq!(response["error"]["code"], "invalid_mime");
}

#[test]
fn notify_without_message_is_rejected_instead_of_using_a_fallback_signature() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "notify"}));

    assert_eq!(response["error"]["code"], "callback_error");
}

#[test]
fn notification_title_above_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "title-limit"}));

    assert_eq!(response["error"]["code"], "string_limit");
}

#[test]
fn notification_message_above_limit_returns_a_structured_error() {
    let mut host = HostProcess::start("invalid-actions.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "activate", json!({"id": "message-limit"}));

    assert_eq!(response["error"]["code"], "string_limit");
}

#[test]
fn non_plain_ipc_return_value_is_rejected() {
    let mut host = HostProcess::start("invalid-result.luau", json!({}));
    host.ready();

    let response = host.request(
        json!(1),
        "ipc",
        json!({"event": "invalid", "payload": null}),
    );

    assert_eq!(response["error"]["code"], "invalid_result");
}

#[test]
fn vm_memory_limit_rejects_allocation_above_sixty_four_mibibytes() {
    let mut host = HostProcess::start("memory-limit.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "memory_limit");
}

#[test]
fn serialized_response_above_limit_becomes_a_small_error() {
    let mut host = HostProcess::start("oversized-response.luau", json!({}));
    host.ready();

    let response = host.request(json!(1), "query", json!({"query": ""}));

    assert_eq!(response["error"]["code"], "response_too_large");
}

#[test]
fn official_empty_query_result_count_fits_the_response_ceiling() {
    let mut host = HostProcess::start("large-response.luau", json!({}));
    host.ready();

    let line = host.request_line(json!(1), "query", json!({"query": "kaomoji"}));
    let response: JsonValue =
        serde_json::from_str(&line).expect("large response should remain valid JSON");
    let line_bytes = line.len().saturating_add(1);

    assert_eq!(response["result"]["query"], "kaomoji");
    assert_eq!(
        response["result"]["results"].as_array().map(Vec::len),
        Some(1_454)
    );
    assert!(
        line_bytes > MAX_REQUEST_LINE_BYTES,
        "fixture should prove responses may exceed the request ceiling"
    );
    assert!(
        line_bytes <= MAX_RESPONSE_LINE_BYTES,
        "fixture response should fit the response ceiling"
    );
}
