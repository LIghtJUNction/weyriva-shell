use std::fmt;

use mlua::Error as LuaError;
use serde::Serialize;

const MAX_ERROR_MESSAGE_BYTES: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostError {
    code: &'static str,
    message: String,
}

impl HostError {
    /// Creates a stable protocol error.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the stable machine-readable error code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// Returns a bounded protocol representation of this error.
    #[must_use]
    pub fn body(&self) -> ErrorBody {
        ErrorBody {
            code: self.code.to_owned(),
            message: bounded_message(&self.message),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HostError {}

impl From<LuaError> for HostError {
    fn from(error: LuaError) -> Self {
        from_lua(&error, "runtime_init")
    }
}

pub fn from_lua(error: &LuaError, fallback_code: &'static str) -> HostError {
    if let Some(host_error) = error.downcast_ref::<HostError>() {
        return host_error.clone();
    }

    let code = match error {
        LuaError::MemoryError(_) => "memory_limit",
        LuaError::SyntaxError { .. } => "plugin_syntax",
        _ => fallback_code,
    };
    HostError::new(code, error.to_string())
}

pub type HostResult<T> = Result<T, HostError>;

fn bounded_message(message: &str) -> String {
    if message.len() <= MAX_ERROR_MESSAGE_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_ERROR_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &message[..end])
}

#[cfg(test)]
mod tests {
    use super::{HostError, MAX_ERROR_MESSAGE_BYTES};

    #[test]
    fn error_body_truncates_long_unicode_message_on_a_character_boundary() {
        let error = HostError::new("fixture", "界".repeat(MAX_ERROR_MESSAGE_BYTES));

        let body = error.body();

        assert!(body.message.len() <= MAX_ERROR_MESSAGE_BYTES + '…'.len_utf8());
    }
}
