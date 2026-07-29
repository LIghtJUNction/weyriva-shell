mod common;
#[path = "support/installed.rs"]
mod installed;

use std::fs;
use std::path::Path;

use installed::{InstalledFixture as Fixture, installed_fixture};
use tempfile::tempdir;
use weyriva::model::StateDocument;
use weyriva::sources;
use weyriva::state_writer::{DurableStateWriter, StateWriteError, StateWriteResult, StateWriter};
use weyriva::{Broker, Error};

#[derive(Clone, Copy)]
enum InjectedWriteFailure {
    PreCommit,
    PostCommit,
}

struct FailingStateWriter {
    failure: InjectedWriteFailure,
}

impl StateWriter for FailingStateWriter {
    fn write(&self, path: &Path, state: &StateDocument) -> StateWriteResult<()> {
        let error = Error::new("injected_state_write", "injected state persistence failure");
        match self.failure {
            InjectedWriteFailure::PreCommit => Err(StateWriteError::PreCommit(error)),
            InjectedWriteFailure::PostCommit => {
                DurableStateWriter.write(path, state)?;
                Err(StateWriteError::PostCommit(error))
            }
        }
    }
}

#[test]
fn enable_write_failure_rolls_runtime_back_to_disabled() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, false);
    let mut broker = failing_broker(&fixture);

    let error = broker
        .enable("test/demo")
        .expect_err("injected enable persistence should fail");
    let persisted = sources::load_state(&fixture.paths).expect("persisted state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("runtime status should load");

    assert_eq!(error.code(), "state_persistence_failed");
    assert!(!persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "disabled");
    assert_eq!(
        error.body().details.expect("rollback details should exist")["rollback"]["runtime"],
        "disabled"
    );
}

#[test]
fn disable_write_failure_restores_enabled_running_runtime() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker = failing_broker(&fixture);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .disable("test/demo")
        .expect_err("injected disable persistence should fail");
    let persisted = sources::load_state(&fixture.paths).expect("persisted state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("runtime status should load");
    let query = broker
        .query("test/demo:main", "restored")
        .expect("restored host should answer");

    assert_eq!(error.code(), "state_persistence_failed");
    assert!(persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "running");
    assert_eq!(query["results"][0]["title"], "Result restored");
    assert_eq!(
        error.body().details.expect("rollback details should exist")["rollback"]["runtime"],
        "running"
    );
}

#[test]
fn uninstall_write_failure_restores_tree_and_running_runtime() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let installed = sources::load_state(&fixture.paths)
        .expect("persisted state should load")
        .plugins["test/demo"]
        .path
        .clone();
    let mut broker = failing_broker(&fixture);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .uninstall("test/demo")
        .expect_err("injected uninstall persistence should fail");
    let persisted = sources::load_state(&fixture.paths).expect("persisted state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("runtime status should load");
    let query = broker
        .query("test/demo:main", "restored")
        .expect("restored host should answer");

    assert_eq!(error.code(), "state_persistence_failed");
    assert!(persisted.plugins["test/demo"].enabled);
    assert!(installed.exists(), "quarantined tree should be restored");
    assert_eq!(status.plugins[0].lifecycle, "running");
    assert_eq!(query["results"][0]["title"], "Result restored");
    assert_eq!(
        error.body().details.expect("rollback details should exist")["rollback"]["tree"],
        "restored"
    );
}

#[test]
fn enable_postcommit_failure_keeps_visible_state_and_running_runtime() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, false);
    let mut broker = broker_with_write_failure(&fixture, InjectedWriteFailure::PostCommit);

    let error = broker
        .enable("test/demo")
        .expect_err("postcommit durability injection should be reported");
    let persisted = sources::load_state(&fixture.paths).expect("committed state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("runtime status should load");
    let query = broker
        .query("test/demo:main", "committed")
        .expect("committed enabled host should remain usable");

    assert_eq!(error.code(), "state_committed_not_durable");
    assert!(persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "running");
    assert_eq!(query["results"][0]["title"], "Result committed");
}

#[test]
fn disable_postcommit_failure_keeps_visible_state_and_stopped_runtime() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker = broker_with_write_failure(&fixture, InjectedWriteFailure::PostCommit);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .disable("test/demo")
        .expect_err("postcommit durability injection should be reported");
    let persisted = sources::load_state(&fixture.paths).expect("committed state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("runtime status should load");

    assert_eq!(error.code(), "state_committed_not_durable");
    assert!(!persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "disabled");
}

#[test]
fn uninstall_postcommit_failure_finalizes_tree_removal_without_restore() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let installed = sources::load_state(&fixture.paths)
        .expect("persisted state should load")
        .plugins["test/demo"]
        .path
        .clone();
    let mut broker = broker_with_write_failure(&fixture, InjectedWriteFailure::PostCommit);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .uninstall("test/demo")
        .expect_err("postcommit durability injection should be reported");
    let persisted = sources::load_state(&fixture.paths).expect("committed state should load");

    assert_eq!(error.code(), "state_committed_not_durable");
    assert!(!persisted.plugins.contains_key("test/demo"));
    assert!(
        !installed.exists(),
        "committed uninstall must not restore the tree"
    );
    assert!(
        fs::read_dir(installed.parent().expect("install has parent"))
            .expect("install root should remain readable")
            .all(|entry| {
                !entry
                    .expect("install root entry should be readable")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".remove-")
            }),
        "postcommit uninstall should finalize quarantine deletion"
    );
}

#[test]
fn disable_reports_explicit_rollback_failure_when_host_cannot_restart() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker = failing_broker(&fixture);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");
    fs::remove_file(&fixture.host).expect("host executable should be removed");

    let error = broker
        .disable("test/demo")
        .expect_err("runtime restoration should fail without the host executable");
    let persisted = sources::load_state(&fixture.paths).expect("persisted state should load");
    let status = broker
        .status(Some("test/demo"))
        .expect("failed rollback status should load");

    assert_eq!(error.code(), "rollback_failed");
    assert!(persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "failed");
}

#[test]
fn reload_does_not_rewrite_unchanged_state() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker = failing_broker(&fixture);
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let result = broker
        .reload("test/demo")
        .expect("reload should not call the failing state writer");

    assert_eq!(result["status"]["plugins"][0]["lifecycle"], "running");
}

fn failing_broker(fixture: &Fixture) -> Broker {
    broker_with_write_failure(fixture, InjectedWriteFailure::PreCommit)
}

fn broker_with_write_failure(fixture: &Fixture, failure: InjectedWriteFailure) -> Broker {
    Broker::with_host_and_state_writer(
        fixture.paths.clone(),
        fixture.host.clone(),
        Box::new(FailingStateWriter { failure }),
    )
}
