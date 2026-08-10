use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::error::{Error, Result};
use crate::model::ActionOutcome;

const ACTION_TIMEOUT: Duration = Duration::from_secs(3);

/// Decodes one validated cliphist entry and writes it to the Wayland
/// clipboard without invoking a shell.
///
/// # Errors
///
/// Returns an error for an invalid entry ID, unavailable commands, timeouts,
/// or unsuccessful decode/copy processes.
pub fn copy_clipboard_entry(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::new(
            "invalid_params",
            "clipboard entry id is invalid",
        ));
    }

    let mut decoder = Command::new("cliphist")
        .args(["decode", id])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| Error::io("cannot start cliphist", &error))?;
    let decoded_stdout = decoder
        .stdout
        .take()
        .ok_or_else(|| Error::new("action_failed", "cliphist stdout was not piped"))?;
    let copier = Command::new("wl-copy")
        .stdin(decoded_stdout)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let copier = match copier {
        Ok(copier) => copier,
        Err(error) => {
            let _ = decoder.kill();
            let _ = decoder.wait();
            return Err(Error::io("cannot start wl-copy", &error));
        }
    };
    if let Err(error) = wait_success(decoder, "clipboard decode") {
        let mut copier = copier;
        let _ = copier.kill();
        let _ = copier.wait();
        return Err(error);
    }
    wait_success(copier, "clipboard copy")
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Clipboard { text: String, mime: String },
    Notify { title: String, message: String },
    NotifyError { title: String, message: String },
    LauncherSetQuery { query: String },
    SetQuery { query: String },
}

/// Validates and executes bounded host actions without a shell.
///
/// # Errors
///
/// Returns an error for malformed actions, unavailable commands, invalid
/// values, timeouts, or unsuccessful subprocesses.
pub fn execute(value: &JsonValue) -> Result<Vec<ActionOutcome>> {
    let values = value
        .as_array()
        .ok_or_else(|| Error::new("invalid_action", "actions must be an array"))?;
    if values.len() > 256 {
        return Err(Error::new(
            "invalid_action",
            "actions exceed the limit of 256",
        ));
    }
    values
        .iter()
        .map(|value| {
            let action: Action = serde_json::from_value(value.clone())
                .map_err(|error| Error::new("invalid_action", error.to_string()))?;
            execute_one(action)
        })
        .collect()
}

fn execute_one(action: Action) -> Result<ActionOutcome> {
    match action {
        Action::Clipboard { text, mime } => {
            if text.len() > 16_384 || !valid_mime(&mime) {
                return Err(Error::new("invalid_action", "clipboard action is invalid"));
            }
            let mut child = Command::new("wl-copy")
                .args(["--type", &mime])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| Error::io("cannot start wl-copy", &error))?;
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| Error::new("action_failed", "wl-copy stdin was not piped"))?;
            stdin
                .write_all(text.as_bytes())
                .map_err(|error| Error::io("cannot write clipboard text", &error))?;
            drop(stdin);
            wait_success(child, "clipboard")?;
            Ok(ActionOutcome::Clipboard { ok: true })
        }
        Action::Notify { title, message } => {
            validate_notification(&title, &message)?;
            let child = Command::new("notify-send")
                .args(["--", &title, &message])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| Error::io("cannot start notify-send", &error))?;
            wait_success(child, "notification")?;
            Ok(ActionOutcome::Notify { ok: true })
        }
        Action::NotifyError { title, message } => {
            validate_notification(&title, &message)?;
            let child = Command::new("notify-send")
                .args(["--urgency=critical", "--", &title, &message])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| Error::io("cannot start notify-send", &error))?;
            wait_success(child, "error notification")?;
            Ok(ActionOutcome::NotifyError { ok: true })
        }
        Action::LauncherSetQuery { query } | Action::SetQuery { query } => {
            if query.len() > 4_096 {
                return Err(Error::new("invalid_action", "set_query exceeds 4096 bytes"));
            }
            Ok(ActionOutcome::SetQuery { query, ok: true })
        }
    }
}

fn wait_success(mut child: std::process::Child, label: &str) -> Result<()> {
    let deadline = Instant::now() + ACTION_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| Error::io("cannot inspect action process", &error))?
        {
            if status.success() {
                return Ok(());
            }
            let output = child
                .wait_with_output()
                .map_err(|error| Error::io("cannot collect action failure", &error))?;
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(Error::new(
                "action_failed",
                format!("{label} failed: {}", detail.trim()),
            ));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new("action_timeout", format!("{label} timed out")));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn validate_notification(title: &str, message: &str) -> Result<()> {
    if title.is_empty() || title.len() > 512 || message.len() > 4_096 {
        return Err(Error::new(
            "invalid_action",
            "notification action is invalid",
        ));
    }
    Ok(())
}

fn valid_mime(value: &str) -> bool {
    if value.is_empty() || value.len() > 256 {
        return false;
    }
    let mut sections = value.split(';');
    let Some(media) = sections.next() else {
        return false;
    };
    let mut media_parts = media.split('/');
    if !matches!(
        (media_parts.next(), media_parts.next(), media_parts.next()),
        (Some(top), Some(sub), None) if mime_token(top) && mime_token(sub)
    ) {
        return false;
    }
    sections.all(|parameter| {
        let mut fields = parameter.split('=');
        matches!(
            (fields.next(), fields.next(), fields.next()),
            (Some(name), Some(value), None) if mime_token(name) && mime_token(value)
        )
    })
}

fn mime_token(value: &str) -> bool {
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
