use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value as JsonValue, json};

use crate::error::{Error, Result};
use crate::model::{CONTROL_PROTOCOL, MAX_LINE_BYTES};

use super::protocol::MAX_CONTROL_RESPONSE_BYTES;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(10);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Sends one bounded request to the Weyriva daemon.
///
/// # Errors
///
/// Returns an error for connection, timeout, framing, size, or JSON failures.
pub fn call(socket: &Path, method: &str, params: &JsonValue) -> Result<JsonValue> {
    let timeout = timeout_for_method(method);
    let mut stream = UnixStream::connect(socket)
        .map_err(|error| Error::io("Weyriva daemon is unavailable", &error))?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| Error::io("cannot set IPC timeout", &error))?;
    let mut encoded = serde_json::to_vec(&json!({
        "protocol": CONTROL_PROTOCOL,
        "id": 1,
        "method": method,
        "params": params,
    }))?;
    encoded.push(b'\n');
    if encoded.len() > MAX_LINE_BYTES {
        return Err(Error::new(
            "request_too_large",
            "control request exceeds 64 KiB",
        ));
    }
    stream
        .write_all(&encoded)
        .and_then(|()| stream.flush())
        .map_err(|error| Error::io("cannot send IPC request", &error))?;
    let mut line = Vec::new();
    BufReader::new(stream)
        .take(u64::try_from(MAX_CONTROL_RESPONSE_BYTES).unwrap_or(u64::MAX) + 1)
        .read_until(b'\n', &mut line)
        .map_err(|error| Error::io("cannot read IPC response", &error))?;
    if line.is_empty() {
        return Err(Error::new(
            "connection_closed",
            "daemon closed the connection without a response",
        ));
    }
    if line.len() > MAX_CONTROL_RESPONSE_BYTES || !line.ends_with(b"\n") {
        return Err(Error::new(
            "response_too_large",
            "daemon response is too large",
        ));
    }
    let response: JsonValue = serde_json::from_slice(&line)?;
    if !response.is_object() {
        return Err(Error::new(
            "invalid_response",
            "daemon response is not an object",
        ));
    }
    Ok(response)
}

#[must_use]
pub fn timeout_for_method(method: &str) -> Duration {
    match method.strip_prefix("weyriva.plugin.v1.") {
        Some("install") => INSTALL_TIMEOUT,
        Some("enable" | "disable" | "reload" | "uninstall") => LIFECYCLE_TIMEOUT,
        _ => DEFAULT_TIMEOUT,
    }
}
