use serde_json::{Map, Value as JsonValue, json};

use crate::error::Error;
use crate::model::{CONTROL_PROTOCOL, MAX_LINE_BYTES};

use super::dispatch::Dispatcher;

pub const MAX_CONTROL_RESPONSE_BYTES: usize = 1024 * 1024;

pub struct ParsedRequest {
    pub id: JsonValue,
    pub method: String,
    pub params: JsonValue,
}

pub fn process_line(line: &[u8], dispatcher: &mut Dispatcher<'_>) -> Vec<u8> {
    if line.len() > MAX_LINE_BYTES || !line.ends_with(b"\n") {
        return encode(&error_envelope(
            &JsonValue::Null,
            &Error::new(
                "request_too_large",
                "request must be one newline-terminated line up to 65536 bytes",
            ),
        ));
    }
    let response = match serde_json::from_slice::<JsonValue>(line) {
        Ok(document) => process_document(&document, dispatcher),
        Err(_) => error_envelope(&JsonValue::Null, &Error::new("parse_error", "invalid JSON")),
    };
    encode_bounded(&response)
}

pub fn process_document(document: &JsonValue, dispatcher: &mut Dispatcher<'_>) -> JsonValue {
    match parse_request(document) {
        Ok(request) => match dispatcher.dispatch(&request.method, &request.params) {
            Ok(result) => json!({"id": request.id, "result": result}),
            Err(error) => error_envelope(&request.id, &error),
        },
        Err((id, error)) => error_envelope(&id, &error),
    }
}

fn parse_request(document: &JsonValue) -> Result<ParsedRequest, (JsonValue, Error)> {
    let Some(object) = document.as_object() else {
        return Err((
            JsonValue::Null,
            Error::new("invalid_request", "request must be an object"),
        ));
    };
    let id = object.get("id").cloned().unwrap_or(JsonValue::Null);
    if matches!(id, JsonValue::Array(_) | JsonValue::Object(_)) {
        return Err((
            id,
            Error::new("invalid_request", "id must be a scalar JSON value"),
        ));
    }
    if object.get("protocol").and_then(JsonValue::as_u64) != Some(u64::from(CONTROL_PROTOCOL)) {
        return Err((
            id,
            Error::new(
                "unsupported_protocol",
                format!("protocol must be {CONTROL_PROTOCOL}"),
            ),
        ));
    }
    let method = object
        .get("method")
        .and_then(JsonValue::as_str)
        .filter(|method| !method.is_empty())
        .ok_or_else(|| {
            (
                id.clone(),
                Error::new("invalid_request", "method must be a non-empty string"),
            )
        })?;
    Ok(ParsedRequest {
        id,
        method: method.to_owned(),
        params: object.get("params").cloned().unwrap_or_else(|| json!({})),
    })
}

fn error_envelope(id: &JsonValue, error: &Error) -> JsonValue {
    json!({"id": id, "error": error.body()})
}

fn encode_bounded(response: &JsonValue) -> Vec<u8> {
    let encoded = encode(response);
    if encoded.len() <= MAX_CONTROL_RESPONSE_BYTES {
        return encoded;
    }
    encode(&json!({
        "id": response.get("id").cloned().unwrap_or(JsonValue::Null),
        "error": {
            "code": "response_too_large",
            "message": "control response exceeds 1 MiB"
        }
    }))
}

fn encode(response: &JsonValue) -> Vec<u8> {
    let mut output = serde_json::to_vec(response).unwrap_or_else(|_| {
        b"{\"error\":{\"code\":\"serialization_error\",\"message\":\"cannot encode response\"},\"id\":null}".to_vec()
    });
    output.push(b'\n');
    output
}

pub fn object(params: &JsonValue) -> crate::error::Result<&Map<String, JsonValue>> {
    params
        .as_object()
        .ok_or_else(|| Error::new("invalid_params", "params must be an object"))
}
