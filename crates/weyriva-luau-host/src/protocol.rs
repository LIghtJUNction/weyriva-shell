use std::io::{self, BufRead, Write};

use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::config::MAX_PLUGIN_DATA_BYTES;
use crate::error::{ErrorBody, HostError};
use crate::runtime::Host;

pub const PROTOCOL_VERSION: &str = "weyriva-luau-host/1";
pub const MAX_REQUEST_LINE_BYTES: usize = 64 * 1_024;
pub const MAX_RESPONSE_LINE_BYTES: usize = MAX_PLUGIN_DATA_BYTES;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    id: JsonValue,
    method: String,
    #[serde(default)]
    params: JsonValue,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryParams {
    query: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivateParams {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IpcParams {
    event: String,
    #[serde(default)]
    payload: JsonValue,
}

/// Builds the first event emitted by a successfully loaded host.
#[must_use]
pub fn ready_event() -> JsonValue {
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
}

/// Builds a bounded startup failure event.
#[must_use]
pub fn fatal_event(error: &HostError) -> JsonValue {
    json!({
        "protocol": PROTOCOL_VERSION,
        "event": "fatal",
        "error": error.body()
    })
}

/// Serves newline-delimited protocol requests until EOF or shutdown.
///
/// # Errors
///
/// Returns an I/O error when stdin cannot be read or stdout cannot be written.
pub fn serve(host: &Host, input: &mut impl BufRead, output: &mut impl Write) -> io::Result<()> {
    write_message(output, &ready_event(), None)?;
    loop {
        match read_bounded_line(input)? {
            LineRead::End => return Ok(()),
            LineRead::Oversized => {
                let response = error_response(
                    &JsonValue::Null,
                    &ErrorBody {
                        code: "request_too_large".to_owned(),
                        message: format!("request line exceeds {MAX_REQUEST_LINE_BYTES} bytes"),
                    },
                );
                write_message(output, &response, Some(&JsonValue::Null))?;
            }
            LineRead::Unterminated => {
                let response = error_response(
                    &JsonValue::Null,
                    &ErrorBody {
                        code: "unterminated_request".to_owned(),
                        message: "request must end with a newline".to_owned(),
                    },
                );
                write_message(output, &response, Some(&JsonValue::Null))?;
                return Ok(());
            }
            LineRead::Line(line) => {
                let (response, should_stop) = process_line(host, &line);
                let id = response.get("id");
                write_message(output, &response, id)?;
                if should_stop {
                    return Ok(());
                }
            }
        }
    }
}

fn process_line(host: &Host, line: &[u8]) -> (JsonValue, bool) {
    let request = match serde_json::from_slice::<Request>(line) {
        Ok(request) => request,
        Err(error) => {
            return (
                error_response(
                    &JsonValue::Null,
                    &ErrorBody {
                        code: "parse_error".to_owned(),
                        message: format!("invalid request JSON: {error}"),
                    },
                ),
                false,
            );
        }
    };

    if !is_scalar_id(&request.id) {
        return (
            error_response(
                &JsonValue::Null,
                &ErrorBody {
                    code: "invalid_id".to_owned(),
                    message: "request id must be null, boolean, number, or string".to_owned(),
                },
            ),
            false,
        );
    }
    let id = request.id;
    if request.protocol != PROTOCOL_VERSION {
        return (
            error_response(
                &id,
                &ErrorBody {
                    code: "unsupported_protocol".to_owned(),
                    message: format!(
                        "protocol `{}` is unsupported; expected `{PROTOCOL_VERSION}`",
                        request.protocol
                    ),
                },
            ),
            false,
        );
    }

    let method = request.method;
    let result = match method.as_str() {
        "query" => parse_params::<QueryParams>(request.params).and_then(|params| {
            host.query(params.query).map(|invocation| {
                json!({
                    "query": invocation.value.query,
                    "results": invocation.value.results,
                    "actions": invocation.actions
                })
            })
        }),
        "activate" => parse_params::<ActivateParams>(request.params).and_then(|params| {
            let result_id = params.id.clone();
            host.activate(params.id).map(|invocation| {
                json!({
                    "activated": result_id,
                    "actions": invocation.actions
                })
            })
        }),
        "ipc" => parse_params::<IpcParams>(request.params).and_then(|params| {
            host.ipc(params.event, &params.payload).map(|invocation| {
                json!({
                    "value": invocation.value,
                    "actions": invocation.actions
                })
            })
        }),
        "shutdown" => validate_empty_params(request.params).and_then(|()| {
            host.shutdown().map(|invocation| {
                json!({
                    "exit_callback_called": invocation.value,
                    "actions": invocation.actions
                })
            })
        }),
        _ => Err(HostError::new(
            "unknown_method",
            format!("method `{method}` is not supported"),
        )),
    };
    let should_stop = method == "shutdown";
    match result {
        Ok(result) => (success_response(&id, &result), should_stop),
        Err(error) => (error_response(&id, &error.body()), should_stop),
    }
}

fn parse_params<T: for<'de> Deserialize<'de>>(params: JsonValue) -> Result<T, HostError> {
    serde_json::from_value(params).map_err(|error| {
        HostError::new(
            "invalid_params",
            format!("invalid method parameters: {error}"),
        )
    })
}

fn validate_empty_params(params: JsonValue) -> Result<(), HostError> {
    match params {
        JsonValue::Null => Ok(()),
        JsonValue::Object(fields) if fields.is_empty() => Ok(()),
        _ => Err(HostError::new(
            "invalid_params",
            "shutdown params must be absent or an empty object",
        )),
    }
}

fn success_response(id: &JsonValue, result: &JsonValue) -> JsonValue {
    json!({
        "protocol": PROTOCOL_VERSION,
        "id": id,
        "result": result
    })
}

fn error_response(id: &JsonValue, error: &ErrorBody) -> JsonValue {
    json!({
        "protocol": PROTOCOL_VERSION,
        "id": id,
        "error": error
    })
}

fn is_scalar_id(id: &JsonValue) -> bool {
    matches!(
        id,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_)
    )
}

fn write_message(
    output: &mut impl Write,
    value: &JsonValue,
    request_id: Option<&JsonValue>,
) -> io::Result<()> {
    let mut encoded = serde_json::to_vec(value).map_err(io::Error::other)?;
    if encoded.len().saturating_add(1) > MAX_RESPONSE_LINE_BYTES {
        let id = request_id.cloned().unwrap_or(JsonValue::Null);
        encoded = serde_json::to_vec(&error_response(
            &id,
            &ErrorBody {
                code: "response_too_large".to_owned(),
                message: format!("response line exceeds {MAX_RESPONSE_LINE_BYTES} bytes"),
            },
        ))
        .map_err(io::Error::other)?;
    }
    output.write_all(&encoded)?;
    output.write_all(b"\n")?;
    output.flush()
}

enum LineRead {
    Line(Vec<u8>),
    Oversized,
    Unterminated,
    End,
}

fn read_bounded_line(reader: &mut impl BufRead) -> io::Result<LineRead> {
    let mut line = Vec::new();
    let mut oversized = false;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return if line.is_empty() && !oversized {
                Ok(LineRead::End)
            } else {
                Ok(LineRead::Unterminated)
            };
        }

        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(buffer.len(), |index| index + 1);
        let content = newline.map_or(buffer, |index| &buffer[..index]);
        if !oversized {
            if line.len().saturating_add(content.len()).saturating_add(1) > MAX_REQUEST_LINE_BYTES {
                oversized = true;
                line.clear();
            } else {
                line.extend_from_slice(content);
            }
        }
        reader.consume(consumed);
        if newline.is_some() {
            return if oversized {
                Ok(LineRead::Oversized)
            } else {
                Ok(LineRead::Line(line))
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::{LineRead, MAX_REQUEST_LINE_BYTES, read_bounded_line};

    #[test]
    fn bounded_reader_rejects_oversized_line_without_retaining_it() {
        let input = vec![b'x'; MAX_REQUEST_LINE_BYTES + 1]
            .into_iter()
            .chain(*b"\n")
            .collect::<Vec<_>>();
        let mut reader = BufReader::new(input.as_slice());

        let result = read_bounded_line(&mut reader);

        assert!(matches!(result, Ok(LineRead::Oversized)));
    }

    #[test]
    fn bounded_reader_preserves_line_at_exact_limit() {
        let input = vec![b'x'; MAX_REQUEST_LINE_BYTES - 1]
            .into_iter()
            .chain(*b"\n")
            .collect::<Vec<_>>();
        let mut reader = BufReader::new(input.as_slice());

        let result = read_bounded_line(&mut reader);

        assert!(
            matches!(result, Ok(LineRead::Line(line)) if line.len() == MAX_REQUEST_LINE_BYTES - 1)
        );
    }
}
