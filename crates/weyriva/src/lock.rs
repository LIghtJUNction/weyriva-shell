use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::Duration;

use crate::identity::IdentityProvider;
use crate::process::{CommandSpec, ProcessRunner, os};

pub struct LockReconciler {
    process: Arc<dyn ProcessRunner>,
    identity: Arc<dyn IdentityProvider>,
}

impl LockReconciler {
    #[must_use]
    pub fn new(process: Arc<dyn ProcessRunner>, identity: Arc<dyn IdentityProvider>) -> Self {
        Self { process, identity }
    }

    #[must_use]
    pub fn reconcile(&self, environment: &BTreeMap<OsString, OsString>) -> bool {
        let configured = environment
            .get(OsStr::new("XDG_SESSION_ID"))
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let session_id = if configured.trim().is_empty() {
            let command = CommandSpec::new(
                "loginctl",
                [
                    os("show-user"),
                    OsString::from(self.identity.current_uid().to_string()),
                    os("-p"),
                    os("Display"),
                    os("--value"),
                ],
            )
            .with_timeout(Duration::from_secs(5));
            let Ok(output) = self.process.run(&command) else {
                return false;
            };
            if output.code != 0 {
                return false;
            }
            output.stdout.trim().to_owned()
        } else {
            configured.to_owned()
        };
        if !valid_session_id(&session_id) {
            return false;
        }
        let command = CommandSpec::new(
            "loginctl",
            [
                os("show-session"),
                OsString::from(session_id),
                os("-p"),
                os("LockedHint"),
                os("--value"),
            ],
        )
        .with_timeout(Duration::from_secs(5));
        self.process
            .run(&command)
            .is_ok_and(|output| output.code == 0 && output.stdout.trim().eq_ignore_ascii_case("no"))
    }
}

#[must_use]
pub fn valid_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value == value.trim()
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'_' | b'.' | b'-'))
        })
}
