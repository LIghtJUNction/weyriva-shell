use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as JsonValue, json};

use crate::actions;
use crate::error::{Error, ErrorBody, Result};
use crate::host_session::{HostSession, ProcessControl, SystemProcessControl};
use crate::model::PluginRecord;

const AUTOMATIC_RESTART_BUDGET: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StartMode {
    Explicit,
    Automatic,
    Rollback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StartOutcome {
    AlreadyRunning,
    Started,
}

#[derive(Debug)]
pub(crate) struct StopReport {
    pub evidence: JsonValue,
}

pub(crate) struct RuntimeRegistry {
    executable: PathBuf,
    process_control: Arc<dyn ProcessControl>,
    slots: BTreeMap<String, HostSlot>,
}

struct HostSlot {
    session: Option<HostSession>,
    phase: Phase,
    failure: Option<ErrorBody>,
    automatic_restarts_remaining: u8,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Disabled,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl RuntimeRegistry {
    pub fn new(executable: PathBuf) -> Self {
        Self::with_process_control(executable, Arc::new(SystemProcessControl))
    }

    pub fn with_process_control(
        executable: PathBuf,
        process_control: Arc<dyn ProcessControl>,
    ) -> Self {
        Self {
            executable,
            process_control,
            slots: BTreeMap::new(),
        }
    }

    pub fn reset_automatic_restart(&mut self, reference: &str) {
        self.slot(reference).automatic_restarts_remaining = AUTOMATIC_RESTART_BUDGET;
    }

    pub fn start(&mut self, record: &PluginRecord, mode: StartMode) -> Result<StartOutcome> {
        let reference = record.provider.reference();
        self.reconcile(&reference)?;
        let executable = &self.executable;
        let process_control = Arc::clone(&self.process_control);
        let slot = self.slots.entry(reference.clone()).or_default();
        if slot.session.is_some() {
            return Ok(StartOutcome::AlreadyRunning);
        }
        if mode == StartMode::Automatic {
            if slot.automatic_restarts_remaining == 0 {
                let error = Error::new(
                    "restart_limit",
                    "automatic plugin host restart limit reached; reload it explicitly",
                );
                slot.fail(error.body());
                return Err(error);
            }
            slot.automatic_restarts_remaining = slot.automatic_restarts_remaining.saturating_sub(1);
        }
        slot.phase = Phase::Starting;
        let settings = serde_json::to_value(&record.settings_defaults)?;
        match HostSession::start_with_process_control(
            executable,
            &record.path,
            &record.provider,
            &settings,
            process_control,
        ) {
            Ok(session) => {
                slot.session = Some(session);
                slot.phase = Phase::Running;
                slot.failure = None;
                Ok(StartOutcome::Started)
            }
            Err(error) => {
                slot.fail(error.body());
                Err(error)
            }
        }
    }

    pub fn request(
        &mut self,
        record: &PluginRecord,
        method: &str,
        params: JsonValue,
    ) -> Result<JsonValue> {
        let reference = record.provider.reference();
        self.start(record, StartMode::Automatic)?;
        let slot = self.slot(&reference);
        let session = slot
            .session
            .as_mut()
            .ok_or_else(|| Error::new("host_unavailable", "plugin host is not running"))?;
        match session.request(method, params) {
            Ok(result) => Ok(result),
            Err(error) => {
                if host_failure(&error) {
                    if let Some(mut failed) = slot.session.take()
                        && let Err(termination) = failed.terminate()
                    {
                        slot.session = Some(failed);
                        slot.fail(termination.body());
                        return Err(termination);
                    }
                    slot.fail(error.body());
                }
                Err(error)
            }
        }
    }

    pub fn stop(&mut self, record: &PluginRecord) -> Result<StopReport> {
        self.stop_reference(&record.provider.reference())
    }

    pub fn status(&mut self, reference: &str, enabled: bool) -> (String, Option<ErrorBody>) {
        let _ = self.reconcile(reference);
        let Some(slot) = self.slots.get(reference) else {
            return (
                if enabled { "enabled" } else { "disabled" }.to_owned(),
                None,
            );
        };
        (slot.phase.as_str().to_owned(), slot.failure.clone())
    }

    pub fn shutdown(&mut self) -> Result<JsonValue> {
        let references: Vec<String> = self.slots.keys().cloned().collect();
        let mut providers = serde_json::Map::new();
        let mut failures = serde_json::Map::new();
        for reference in references {
            match self.stop_reference(&reference) {
                Ok(report) => {
                    providers.insert(reference, report.evidence);
                }
                Err(error) => {
                    failures.insert(reference, json!(error.body()));
                }
            }
        }
        if failures.is_empty() {
            Ok(json!({"providers": providers}))
        } else {
            Err(Error::with_details(
                "host_shutdown_failed",
                "one or more plugin hosts could not be confirmed stopped",
                json!({"providers": providers, "failures": failures}),
            ))
        }
    }

    fn reconcile(&mut self, reference: &str) -> Result<()> {
        let Some(slot) = self.slots.get_mut(reference) else {
            return Ok(());
        };
        let outcome = slot
            .session
            .as_mut()
            .map(HostSession::has_exited)
            .transpose();
        match outcome {
            Ok(Some(false) | None) => Ok(()),
            Ok(Some(true)) => {
                slot.session.take();
                slot.fail(Error::new("host_exited", "plugin host exited unexpectedly").body());
                Ok(())
            }
            Err(error) => {
                slot.fail(error.body());
                Err(error)
            }
        }
    }

    fn stop_reference(&mut self, reference: &str) -> Result<StopReport> {
        self.reconcile(reference)?;
        let slot = self.slot(reference);
        let Some(mut session) = slot.session.take() else {
            let prior_failure = slot.failure.take();
            slot.phase = Phase::Disabled;
            return Ok(StopReport {
                evidence: json!({
                    "onExit": false,
                    "actions": [],
                    "action_results": [],
                    "prior_failure": prior_failure,
                }),
            });
        };
        slot.phase = Phase::Stopping;
        let evidence = match session.shutdown() {
            Ok(mut evidence) => match execute_result_actions(&evidence) {
                Ok(outcomes) => {
                    if let Some(object) = evidence.as_object_mut() {
                        object.insert("action_results".to_owned(), outcomes);
                    }
                    evidence
                }
                Err(error) => json!({
                    "onExit": true,
                    "actions": evidence.get("actions").cloned().unwrap_or_else(|| json!([])),
                    "action_results": [],
                    "action_error": error.body(),
                }),
            },
            Err(shutdown_error) => match session.terminate() {
                Ok(()) => json!({
                    "onExit": false,
                    "actions": [],
                    "action_results": [],
                    "shutdown_error": shutdown_error.body(),
                    "termination": "confirmed",
                }),
                Err(termination_error) => {
                    let error = Error::with_details(
                        "host_termination_failed",
                        "plugin host termination could not be confirmed",
                        json!({
                            "shutdown": shutdown_error.body(),
                            "termination": termination_error.body(),
                        }),
                    );
                    slot.session = Some(session);
                    slot.fail(error.body());
                    return Err(error);
                }
            },
        };
        slot.phase = Phase::Disabled;
        slot.failure = None;
        Ok(StopReport { evidence })
    }

    fn slot(&mut self, reference: &str) -> &mut HostSlot {
        self.slots.entry(reference.to_owned()).or_default()
    }
}

impl Default for HostSlot {
    fn default() -> Self {
        Self {
            session: None,
            phase: Phase::Disabled,
            failure: None,
            automatic_restarts_remaining: AUTOMATIC_RESTART_BUDGET,
        }
    }
}

impl HostSlot {
    fn fail(&mut self, failure: ErrorBody) {
        self.phase = Phase::Failed;
        self.failure = Some(failure);
    }
}

impl Phase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }
}

pub(crate) fn execute_result_actions(result: &JsonValue) -> Result<JsonValue> {
    let empty = JsonValue::Array(Vec::new());
    let values = result.get("actions").unwrap_or(&empty);
    serde_json::to_value(actions::execute(values)?).map_err(Into::into)
}

fn host_failure(error: &Error) -> bool {
    matches!(
        error.code(),
        "host_unavailable" | "host_timeout" | "host_exited" | "host_protocol" | "io_error"
    )
}
