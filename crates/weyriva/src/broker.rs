#![expect(
    clippy::missing_errors_doc,
    reason = "broker methods share the crate's typed control-plane error contract"
)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value as JsonValue, json};

use crate::error::{Error, Result};
use crate::host_session::ProcessControl;
use crate::install::{self, remove_private_tree_if_exists};
use crate::model::{PluginRecord, STATE_SCHEMA, StateDocument, StatusRecord, StatusResponse};
use crate::paths::Paths;
use crate::runtime::{RuntimeRegistry, StartMode, execute_result_actions};
use crate::sources;
use crate::state_writer::{DurableStateWriter, StateWriteError, StateWriter};
use crate::transaction::{committed_not_durable, persistence_failure, rollback_failure};
use crate::validation::{parse_reference, validate_query, validate_uninstall_path};
use std::sync::Arc;

pub struct Broker {
    paths: Paths,
    runtime: RuntimeRegistry,
    state_writer: Box<dyn StateWriter>,
}

impl Broker {
    #[must_use]
    pub fn new(paths: Paths) -> Self {
        let host_executable = env::var_os("WEYRIVA_LUAU_HOST")
            .map_or_else(|| PathBuf::from("weyriva-luau-host"), PathBuf::from);
        Self::with_host(paths, host_executable)
    }

    #[must_use]
    pub fn with_host(paths: Paths, host_executable: PathBuf) -> Self {
        Self::with_host_and_state_writer(
            paths,
            host_executable,
            Box::<DurableStateWriter>::default(),
        )
    }

    #[must_use]
    pub fn with_host_and_state_writer(
        paths: Paths,
        host_executable: PathBuf,
        state_writer: Box<dyn StateWriter>,
    ) -> Self {
        Self {
            paths,
            runtime: RuntimeRegistry::new(host_executable),
            state_writer,
        }
    }

    #[must_use]
    pub fn with_host_state_writer_and_process_control(
        paths: Paths,
        host_executable: PathBuf,
        state_writer: Box<dyn StateWriter>,
        process_control: Arc<dyn ProcessControl>,
    ) -> Self {
        Self {
            paths,
            runtime: RuntimeRegistry::with_process_control(host_executable, process_control),
            state_writer,
        }
    }

    pub fn source_list(&self) -> Result<JsonValue> {
        sources::list(&self.paths)
    }

    pub fn source_add(&self, name: &str, path: &Path) -> Result<JsonValue> {
        sources::add(&self.paths, name, path)
    }

    pub fn source_remove(&self, name: &str) -> Result<JsonValue> {
        sources::remove(&self.paths, name)
    }

    pub fn install(&mut self, plugin_id: &str) -> Result<JsonValue> {
        install::install(&self.paths, plugin_id)?;
        serde_json::to_value(self.status(Some(plugin_id))?).map_err(Into::into)
    }

    pub fn status(&mut self, plugin_id: Option<&str>) -> Result<StatusResponse> {
        let state = sources::load_state(&self.paths)?;
        let records: Vec<_> = if let Some(plugin_id) = plugin_id {
            vec![state.plugins.get(plugin_id).cloned().ok_or_else(|| {
                Error::new(
                    "plugin_not_installed",
                    format!("plugin is not installed: {plugin_id}"),
                )
            })?]
        } else {
            state.plugins.into_values().collect()
        };
        let mut plugins = Vec::with_capacity(records.len());
        for record in records {
            let reference = record.provider.reference();
            let (lifecycle, failure) = self.runtime.status(&reference, record.enabled);
            plugins.push(StatusRecord {
                record,
                lifecycle,
                failure,
            });
        }
        Ok(StatusResponse {
            schema: STATE_SCHEMA,
            plugins,
        })
    }

    pub fn enable(&mut self, plugin_id: &str) -> Result<JsonValue> {
        let (mut state, mut record) = self.record(plugin_id)?;
        let reference = record.provider.reference();
        self.runtime.reset_automatic_restart(&reference);
        self.runtime.start(&record, StartMode::Explicit)?;
        if record.enabled {
            return serde_json::to_value(self.status(Some(plugin_id))?).map_err(Into::into);
        }
        record.enabled = true;
        record.last_known_good = Some(record.digest.clone());
        state.plugins.insert(plugin_id.to_owned(), record.clone());
        if let Err(failure) = self.state_writer.write(&self.paths.state_file(), &state) {
            return match failure {
                StateWriteError::PreCommit(error) => match self.runtime.stop(&record) {
                    Ok(rollback) => Err(persistence_failure(
                        "enable",
                        &error,
                        &json!({"runtime": "disabled", "shutdown": rollback.evidence}),
                    )),
                    Err(rollback) => Err(rollback_failure(
                        "enable",
                        &error,
                        &rollback,
                        &json!({"runtime": "unknown"}),
                    )),
                },
                StateWriteError::PostCommit(error) => Err(committed_not_durable(
                    "enable",
                    &error,
                    &json!({"runtime": "running"}),
                )),
            };
        }
        serde_json::to_value(self.status(Some(plugin_id))?).map_err(Into::into)
    }

    pub fn disable(&mut self, plugin_id: &str) -> Result<JsonValue> {
        let (mut state, mut record) = self.record(plugin_id)?;
        let shutdown = self.runtime.stop(&record)?;
        if !record.enabled {
            return Ok(json!({
                "status": self.status(Some(plugin_id))?,
                "shutdown": shutdown.evidence,
            }));
        }
        let previous = record.clone();
        record.enabled = false;
        state.plugins.insert(plugin_id.to_owned(), record);
        if let Err(failure) = self.state_writer.write(&self.paths.state_file(), &state) {
            return match failure {
                StateWriteError::PreCommit(error) => {
                    match self.runtime.start(&previous, StartMode::Rollback) {
                        Ok(_) => Err(persistence_failure(
                            "disable",
                            &error,
                            &json!({"runtime": "running", "shutdown": shutdown.evidence}),
                        )),
                        Err(rollback) => Err(rollback_failure(
                            "disable",
                            &error,
                            &rollback,
                            &json!({"shutdown": shutdown.evidence}),
                        )),
                    }
                }
                StateWriteError::PostCommit(error) => Err(committed_not_durable(
                    "disable",
                    &error,
                    &json!({"runtime": "disabled", "shutdown": shutdown.evidence}),
                )),
            };
        }
        Ok(json!({
            "status": self.status(Some(plugin_id))?,
            "shutdown": shutdown.evidence,
        }))
    }

    pub fn reload(&mut self, plugin_id: &str) -> Result<JsonValue> {
        let (_, record) = self.record(plugin_id)?;
        let shutdown = self.runtime.stop(&record)?;
        if record.enabled {
            let reference = record.provider.reference();
            self.runtime.reset_automatic_restart(&reference);
            if let Err(error) = self.runtime.start(&record, StartMode::Explicit) {
                return Err(Error::with_details(
                    error.code(),
                    error.to_string(),
                    json!({"operation": "reload", "shutdown": shutdown.evidence}),
                ));
            }
        }
        Ok(json!({
            "status": self.status(Some(plugin_id))?,
            "shutdown": shutdown.evidence,
        }))
    }

    pub fn uninstall(&mut self, plugin_id: &str) -> Result<JsonValue> {
        let (mut state, record) = self.record(plugin_id)?;
        validate_uninstall_path(&self.paths, plugin_id, &record)?;
        let shutdown = self.runtime.stop(&record)?;
        let trash =
            record
                .path
                .with_file_name(format!(".remove-{}-{}", record.digest, std::process::id()));
        if let Err(error) = fs::rename(&record.path, &trash)
            .map_err(|error| Error::io("cannot quarantine installed plugin", &error))
        {
            if record.enabled {
                return match self.runtime.start(&record, StartMode::Rollback) {
                    Ok(_) => Err(Error::with_details(
                        error.code(),
                        error.to_string(),
                        json!({"operation": "uninstall", "rollback": {"runtime": "running"}}),
                    )),
                    Err(rollback) => Err(rollback_failure(
                        "uninstall",
                        &error,
                        &rollback,
                        &json!({"stage": "quarantine"}),
                    )),
                };
            }
            return Err(error);
        }
        state.plugins.remove(plugin_id);
        if let Err(failure) = self.state_writer.write(&self.paths.state_file(), &state) {
            return match failure {
                StateWriteError::PreCommit(error) => {
                    if let Err(restore) = fs::rename(&trash, &record.path)
                        .map_err(|restore| Error::io("cannot restore quarantined plugin", &restore))
                    {
                        return Err(rollback_failure(
                            "uninstall",
                            &error,
                            &restore,
                            &json!({"stage": "restore_tree", "shutdown": shutdown.evidence}),
                        ));
                    }
                    if record.enabled {
                        match self.runtime.start(&record, StartMode::Rollback) {
                            Ok(_) => Err(persistence_failure(
                                "uninstall",
                                &error,
                                &json!({
                                    "tree": "restored",
                                    "runtime": "running",
                                    "shutdown": shutdown.evidence,
                                }),
                            )),
                            Err(rollback) => Err(rollback_failure(
                                "uninstall",
                                &error,
                                &rollback,
                                &json!({"stage": "restore_runtime", "tree": "restored"}),
                            )),
                        }
                    } else {
                        Err(persistence_failure(
                            "uninstall",
                            &error,
                            &json!({"tree": "restored", "runtime": "disabled"}),
                        ))
                    }
                }
                StateWriteError::PostCommit(error) => {
                    let cleanup = remove_private_tree_if_exists(&trash);
                    let evidence = match cleanup {
                        Ok(()) => json!({
                            "tree": "removed",
                            "runtime": "disabled",
                            "shutdown": shutdown.evidence,
                        }),
                        Err(cleanup) => json!({
                            "tree": "quarantined",
                            "runtime": "disabled",
                            "shutdown": shutdown.evidence,
                            "cleanup": cleanup.body(),
                        }),
                    };
                    Err(committed_not_durable("uninstall", &error, &evidence))
                }
            };
        }
        remove_private_tree_if_exists(&trash)?;
        Ok(json!({"uninstalled": plugin_id, "shutdown": shutdown.evidence}))
    }

    pub fn query(&mut self, reference: &str, query: &str) -> Result<JsonValue> {
        if query.len() > 4_096 {
            return Err(Error::new("invalid_query", "query exceeds 4096 bytes"));
        }
        let result = self.host_request(reference, "query", json!({"query": query}))?;
        validate_query(&result, query)?;
        let action_results = execute_result_actions(&result)?;
        let mut object = result
            .as_object()
            .cloned()
            .ok_or_else(|| Error::new("host_protocol", "query result is not an object"))?;
        object.insert("action_results".to_owned(), action_results);
        Ok(JsonValue::Object(object))
    }

    pub fn activate(&mut self, reference: &str, result_id: &str) -> Result<JsonValue> {
        if result_id.is_empty() || result_id.len() > 256 {
            return Err(Error::new("invalid_result", "result id is invalid"));
        }
        let result = self.host_request(reference, "activate", json!({"id": result_id}))?;
        if result.get("activated").and_then(JsonValue::as_str) != Some(result_id) {
            return Err(Error::new(
                "host_protocol",
                "activate result id does not match",
            ));
        }
        let action_results = execute_result_actions(&result)?;
        let mut object = result
            .as_object()
            .cloned()
            .ok_or_else(|| Error::new("host_protocol", "activate result is not an object"))?;
        object.insert("action_results".to_owned(), action_results);
        Ok(JsonValue::Object(object))
    }

    pub fn start_enabled(&mut self) -> Result<()> {
        let state = sources::load_state(&self.paths)?;
        for record in state.plugins.into_values() {
            if record.enabled {
                let _ = self.runtime.start(&record, StartMode::Automatic);
            }
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<JsonValue> {
        self.runtime.shutdown()
    }

    fn record(&self, plugin_id: &str) -> Result<(StateDocument, PluginRecord)> {
        let state = sources::load_state(&self.paths)?;
        let record = state.plugins.get(plugin_id).cloned().ok_or_else(|| {
            Error::new(
                "plugin_not_installed",
                format!("plugin is not installed: {plugin_id}"),
            )
        })?;
        Ok((state, record))
    }

    fn host_request(
        &mut self,
        reference: &str,
        method: &str,
        params: JsonValue,
    ) -> Result<JsonValue> {
        let (plugin_id, entry_id) = parse_reference(reference)?;
        let (_, record) = self.record(plugin_id)?;
        if record.provider.entry_id != entry_id {
            return Err(Error::new("provider_not_found", "unknown provider"));
        }
        if !record.enabled {
            return Err(Error::new("plugin_disabled", "plugin is disabled"));
        }
        self.runtime.request(&record, method, params)
    }
}
