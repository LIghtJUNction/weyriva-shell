use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::error::{HostError, HostResult};

pub const MAX_ACTIONS: usize = 256;
pub const MAX_RESULTS: usize = 2_000;
pub const MAX_QUERY_BYTES: usize = 4_096;
const MAX_ID_BYTES: usize = 256;
const MAX_TITLE_BYTES: usize = 512;
const MAX_SUBTITLE_BYTES: usize = 1_024;
const MAX_GLYPH_BYTES: usize = 128;
const MAX_CATEGORY_BYTES: usize = 256;
const MAX_NOTIFICATION_TITLE_BYTES: usize = 512;
const MAX_NOTIFICATION_BYTES: usize = 4_096;
const MAX_CLIPBOARD_BYTES: usize = 16_384;
const MAX_MIME_BYTES: usize = 256;
const DEFAULT_TEXT_MIME: &str = "text/plain";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Clipboard { text: String, mime: String },
    Notify { title: String, message: String },
    NotifyError { title: String, message: String },
    LauncherSetQuery { query: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LauncherResult {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LauncherModel {
    pub query: String,
    pub results: Vec<LauncherResult>,
}

impl LauncherModel {
    pub fn from_json(query: String, value: JsonValue) -> HostResult<Self> {
        check_string("query", &query, MAX_QUERY_BYTES, true)?;
        let JsonValue::Array(values) = value else {
            return Err(HostError::new(
                "invalid_model",
                "launcher.setResults expects an array",
            ));
        };
        if values.len() > MAX_RESULTS {
            return Err(HostError::new(
                "result_limit",
                format!("launcher results exceed the limit of {MAX_RESULTS}"),
            ));
        }

        let results = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| parse_result(index, value))
            .collect::<HostResult<Vec<_>>>()?;
        Ok(Self { query, results })
    }
}

impl Action {
    pub fn clipboard(text: String, mime: Option<String>) -> HostResult<Self> {
        check_string("clipboard text", &text, MAX_CLIPBOARD_BYTES, true)?;
        let mime = mime.unwrap_or_else(|| DEFAULT_TEXT_MIME.to_owned());
        validate_mime(&mime)?;
        Ok(Self::Clipboard { text, mime })
    }

    pub fn notify(title: String, message: String) -> HostResult<Self> {
        check_string(
            "notification title",
            &title,
            MAX_NOTIFICATION_TITLE_BYTES,
            false,
        )?;
        check_string(
            "notification message",
            &message,
            MAX_NOTIFICATION_BYTES,
            true,
        )?;
        Ok(Self::Notify { title, message })
    }

    pub fn notify_error(title: String, message: String) -> HostResult<Self> {
        check_string(
            "error notification title",
            &title,
            MAX_NOTIFICATION_TITLE_BYTES,
            false,
        )?;
        check_string(
            "error notification message",
            &message,
            MAX_NOTIFICATION_BYTES,
            true,
        )?;
        Ok(Self::NotifyError { title, message })
    }

    pub fn launcher_set_query(query: String) -> HostResult<Self> {
        check_string("launcher query", &query, MAX_QUERY_BYTES, true)?;
        Ok(Self::LauncherSetQuery { query })
    }
}

fn validate_mime(mime: &str) -> HostResult<()> {
    if mime.len() > MAX_MIME_BYTES {
        return Err(HostError::new(
            "invalid_mime",
            format!("clipboard MIME exceeds the limit of {MAX_MIME_BYTES} bytes"),
        ));
    }
    let mut sections = mime.split(';');
    let media_type = sections.next().unwrap_or_default();
    let Some((top_level, subtype)) = media_type.split_once('/') else {
        return Err(HostError::new(
            "invalid_mime",
            "clipboard MIME must contain one type/subtype separator",
        ));
    };
    if subtype.contains('/') || !is_mime_token(top_level) || !is_mime_token(subtype) {
        return Err(HostError::new(
            "invalid_mime",
            "clipboard MIME type and subtype must be non-empty ASCII tokens",
        ));
    }
    for parameter in sections {
        let Some((name, value)) = parameter.split_once('=') else {
            return Err(HostError::new(
                "invalid_mime",
                "clipboard MIME parameters must use name=value tokens",
            ));
        };
        if !is_mime_token(name) || !is_mime_token(value) {
            return Err(HostError::new(
                "invalid_mime",
                "clipboard MIME parameters must be non-empty ASCII tokens",
            ));
        }
    }
    Ok(())
}

fn is_mime_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

pub fn push_action(actions: &mut Vec<Action>, action: Action) -> HostResult<()> {
    if actions.len() >= MAX_ACTIONS {
        return Err(HostError::new(
            "action_limit",
            format!("plugin actions exceed the limit of {MAX_ACTIONS}"),
        ));
    }
    actions.push(action);
    Ok(())
}

fn parse_result(index: usize, value: JsonValue) -> HostResult<LauncherResult> {
    let JsonValue::Object(mut fields) = value else {
        return Err(HostError::new(
            "invalid_model",
            format!("launcher result {index} must be an object"),
        ));
    };

    let id = required_string(&mut fields, index, "id", MAX_ID_BYTES)?;
    let title = required_string(&mut fields, index, "title", MAX_TITLE_BYTES)?;
    let subtitle = optional_string(&mut fields, index, "subtitle", MAX_SUBTITLE_BYTES)?;
    let glyph = optional_string(&mut fields, index, "glyph", MAX_GLYPH_BYTES)?;
    let category = optional_string(&mut fields, index, "category", MAX_CATEGORY_BYTES)?;
    if let Some(field) = fields.keys().next() {
        return Err(HostError::new(
            "invalid_model",
            format!("launcher result {index} has unsupported field `{field}`"),
        ));
    }

    Ok(LauncherResult {
        id,
        title,
        subtitle,
        glyph,
        category,
    })
}

fn required_string(
    fields: &mut serde_json::Map<String, JsonValue>,
    index: usize,
    name: &str,
    limit: usize,
) -> HostResult<String> {
    let value = fields.remove(name).ok_or_else(|| {
        HostError::new(
            "invalid_model",
            format!("launcher result {index} is missing `{name}`"),
        )
    })?;
    let JsonValue::String(value) = value else {
        return Err(HostError::new(
            "invalid_model",
            format!("launcher result {index} field `{name}` must be a string"),
        ));
    };
    check_string(name, &value, limit, false)?;
    Ok(value)
}

fn optional_string(
    fields: &mut serde_json::Map<String, JsonValue>,
    index: usize,
    name: &str,
    limit: usize,
) -> HostResult<Option<String>> {
    let Some(value) = fields.remove(name) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let JsonValue::String(value) = value else {
        return Err(HostError::new(
            "invalid_model",
            format!("launcher result {index} field `{name}` must be a string"),
        ));
    };
    check_string(name, &value, limit, true)?;
    Ok(Some(value))
}

pub fn check_string(
    label: &str,
    value: &str,
    max_bytes: usize,
    allow_empty: bool,
) -> HostResult<()> {
    if !allow_empty && value.is_empty() {
        return Err(HostError::new(
            "invalid_model",
            format!("{label} must not be empty"),
        ));
    }
    if value.len() > max_bytes {
        return Err(HostError::new(
            "string_limit",
            format!("{label} exceeds the limit of {max_bytes} bytes"),
        ));
    }
    Ok(())
}
