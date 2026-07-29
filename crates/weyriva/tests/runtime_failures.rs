mod common;
#[path = "support/installed.rs"]
mod installed;

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use installed::{InstalledFixture as Fixture, installed_fixture};
use tempfile::tempdir;
use weyriva::host_session::ProcessControl;
use weyriva::model::StateDocument;
use weyriva::sources;
use weyriva::state_writer::{DurableStateWriter, StateWriteError, StateWriteResult, StateWriter};
use weyriva::{Broker, Error};

#[test]
fn explicit_enable_allows_only_one_automatic_restart_after_crash() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, false);
    let mut broker = Broker::with_host(fixture.paths.clone(), fixture.host.clone());
    broker
        .enable("test/demo")
        .expect("explicit enable should start the host");

    let first_crash = broker
        .query("test/demo:main", "crash")
        .expect_err("fixture host should crash");
    let failed_status = broker
        .status(Some("test/demo"))
        .expect("crashed status should reconcile");
    let restarted = broker
        .query("test/demo:main", "restarted")
        .expect("one automatic restart should be available");
    let second_crash = broker
        .query("test/demo:main", "crash")
        .expect_err("restarted fixture host should crash");
    let exhausted = broker
        .query("test/demo:main", "again")
        .expect_err("automatic restart budget should be exhausted");

    assert_eq!(first_crash.code(), "host_exited");
    assert_eq!(failed_status.plugins[0].lifecycle, "failed");
    assert_eq!(restarted["results"][0]["title"], "Result restarted");
    assert_eq!(second_crash.code(), "host_exited");
    assert_eq!(exhausted.code(), "restart_limit");
}

#[test]
fn failed_automatic_spawn_consumes_restart_budget() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker =
        Broker::with_host(fixture.paths.clone(), temporary.path().join("missing-host"));

    broker
        .start_enabled()
        .expect("daemon startup should record host failure");
    let first = broker
        .query("test/demo:main", "first")
        .expect_err("consumed startup attempt should not repeat");
    let second = broker
        .query("test/demo:main", "second")
        .expect_err("repeated queries should remain bounded");

    assert_eq!(first.code(), "restart_limit");
    assert_eq!(second.code(), "restart_limit");
}

#[test]
fn status_reconciles_child_that_exited_after_ready() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, false);
    let host = install_exit_after_ready_host(temporary.path());
    let mut broker = Broker::with_host(fixture.paths, host);
    broker
        .enable("test/demo")
        .expect("ready event should complete explicit enable");
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        let status = broker
            .status(Some("test/demo"))
            .expect("status should poll the child");
        if status.plugins[0].lifecycle == "failed" {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "exited host should become failed"
        );
        thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(status.plugins[0].lifecycle, "failed");
    assert_eq!(
        status.plugins[0]
            .failure
            .as_ref()
            .expect("crash failure should be exposed")
            .code,
        "host_exited"
    );
}

#[test]
fn enable_reports_rollback_failed_when_new_host_cannot_be_proven_stopped() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, false);
    let mut broker = broker_with_termination_failure(&fixture, Box::new(PreCommitFailureWriter));

    let error = broker
        .enable("test/demo")
        .expect_err("unconfirmed rollback stop should fail explicitly");
    let persisted = sources::load_state(&fixture.paths).expect("old state should remain visible");
    let status = broker
        .status(Some("test/demo"))
        .expect("failed runtime status should load");

    assert_eq!(error.code(), "rollback_failed");
    assert!(!persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "failed");
    assert_eq!(
        status.plugins[0]
            .failure
            .as_ref()
            .expect("termination failure should be retained")
            .code,
        "host_termination_failed"
    );
}

#[test]
fn disable_does_not_persist_when_initial_stop_is_unconfirmed() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let mut broker =
        broker_with_termination_failure(&fixture, Box::<DurableStateWriter>::default());
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .disable("test/demo")
        .expect_err("unconfirmed stop must prevent disable persistence");
    let persisted = sources::load_state(&fixture.paths).expect("old state should remain visible");
    let status = broker
        .status(Some("test/demo"))
        .expect("failed runtime status should load");

    assert_eq!(error.code(), "host_termination_failed");
    assert!(persisted.plugins["test/demo"].enabled);
    assert_eq!(status.plugins[0].lifecycle, "failed");
}

#[test]
fn uninstall_does_not_change_state_or_tree_when_initial_stop_is_unconfirmed() {
    let temporary = tempdir().expect("temporary directory should be created");
    let fixture = installed_fixture(&temporary, true);
    let installed = sources::load_state(&fixture.paths)
        .expect("persisted state should load")
        .plugins["test/demo"]
        .path
        .clone();
    let mut broker =
        broker_with_termination_failure(&fixture, Box::<DurableStateWriter>::default());
    broker
        .start_enabled()
        .expect("persisted enabled plugin should start");

    let error = broker
        .uninstall("test/demo")
        .expect_err("unconfirmed stop must prevent uninstall persistence");
    let persisted = sources::load_state(&fixture.paths).expect("old state should remain visible");

    assert_eq!(error.code(), "host_termination_failed");
    assert!(persisted.plugins["test/demo"].enabled);
    assert!(installed.exists(), "install tree must remain authoritative");
}

#[test]
fn termination_failures_report_try_wait_kill_and_wait_stages() {
    for (stage, expected) in [
        (TerminationFailureStage::TryWait, "try_wait"),
        (TerminationFailureStage::Kill, "kill"),
        (TerminationFailureStage::Wait, "wait"),
    ] {
        let temporary = tempdir().expect("temporary directory should be created");
        let fixture = installed_fixture(&temporary, true);
        let mut broker =
            broker_with_termination_stage(&fixture, Box::<DurableStateWriter>::default(), stage);
        broker
            .start_enabled()
            .expect("persisted enabled plugin should start");

        let error = broker
            .disable("test/demo")
            .expect_err("injected termination operation should fail");
        let details = error
            .body()
            .details
            .expect("termination details should exist");
        let persisted =
            sources::load_state(&fixture.paths).expect("old enabled state should remain");

        assert_eq!(error.code(), "host_termination_failed");
        assert_eq!(details["termination"]["details"]["stage"], expected);
        assert!(persisted.plugins["test/demo"].enabled);
    }
}

struct PreCommitFailureWriter;

impl StateWriter for PreCommitFailureWriter {
    fn write(&self, _path: &Path, _state: &StateDocument) -> StateWriteResult<()> {
        Err(StateWriteError::PreCommit(Error::new(
            "injected_state_write",
            "injected state persistence failure",
        )))
    }
}

fn broker_with_termination_failure(
    fixture: &Fixture,
    state_writer: Box<dyn StateWriter>,
) -> Broker {
    broker_with_termination_stage(fixture, state_writer, TerminationFailureStage::Kill)
}

fn broker_with_termination_stage(
    fixture: &Fixture,
    state_writer: Box<dyn StateWriter>,
    stage: TerminationFailureStage,
) -> Broker {
    Broker::with_host_state_writer_and_process_control(
        fixture.paths.clone(),
        fixture.host.clone(),
        state_writer,
        Arc::new(FailingTerminationControl::new(stage)),
    )
}

#[derive(Clone, Copy)]
enum TerminationFailureStage {
    TryWait,
    Kill,
    Wait,
}

struct FailingTerminationControl {
    try_wait_calls: AtomicUsize,
    failure: TerminationFailureStage,
}

impl FailingTerminationControl {
    const fn new(failure: TerminationFailureStage) -> Self {
        Self {
            try_wait_calls: AtomicUsize::new(0),
            failure,
        }
    }
}

impl ProcessControl for FailingTerminationControl {
    fn try_wait(&self, child: &mut Child) -> io::Result<Option<ExitStatus>> {
        let call = self.try_wait_calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call == 2 || (matches!(self.failure, TerminationFailureStage::TryWait) && call == 3) {
            Err(io::Error::other("injected try_wait failure"))
        } else {
            child.try_wait()
        }
    }

    fn kill(&self, child: &mut Child) -> io::Result<()> {
        if matches!(self.failure, TerminationFailureStage::Kill) {
            Err(io::Error::other("injected kill failure"))
        } else {
            child.kill()
        }
    }

    fn wait(&self, child: &mut Child) -> io::Result<ExitStatus> {
        if matches!(self.failure, TerminationFailureStage::Wait) {
            Err(io::Error::other("injected wait failure"))
        } else {
            child.wait()
        }
    }
}

fn install_exit_after_ready_host(root: &Path) -> PathBuf {
    let path = root.join("exit-after-ready-host");
    fs::write(
        &path,
        b"#!/usr/bin/env python3\nimport json\nprint(json.dumps({'protocol':'weyriva-luau-host/1','event':'ready'}), flush=True)\n",
    )
    .expect("exit host fixture should be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .expect("exit host fixture should be executable");
    path
}
