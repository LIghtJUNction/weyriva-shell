use serde_json::{Value as JsonValue, json};

use crate::error::Error;

pub(crate) fn persistence_failure(
    operation: &str,
    persistence: &Error,
    rollback: &JsonValue,
) -> Error {
    Error::with_details(
        "state_persistence_failed",
        format!("{operation} state persistence failed; prior state was restored"),
        json!({
            "operation": operation,
            "commit": "not_committed",
            "persistence": persistence.body(),
            "rollback": rollback,
        }),
    )
}

pub(crate) fn committed_not_durable(
    operation: &str,
    persistence: &Error,
    evidence: &JsonValue,
) -> Error {
    Error::with_details(
        "state_committed_not_durable",
        format!("{operation} state was committed but durability was not confirmed"),
        json!({
            "operation": operation,
            "commit": "committed",
            "durable": false,
            "persistence": persistence.body(),
            "evidence": evidence,
        }),
    )
}

pub(crate) fn rollback_failure(
    operation: &str,
    persistence: &Error,
    rollback: &Error,
    evidence: &JsonValue,
) -> Error {
    Error::with_details(
        "rollback_failed",
        format!("{operation} failed and rollback could not restore the prior runtime"),
        json!({
            "operation": operation,
            "commit": "not_committed",
            "persistence": persistence.body(),
            "rollback": rollback.body(),
            "evidence": evidence,
        }),
    )
}
