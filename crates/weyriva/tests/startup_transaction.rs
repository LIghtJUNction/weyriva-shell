#[path = "startup_support/mod.rs"]
mod startup_support;

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::Path;
use std::sync::Arc;

use startup_support::{Fixture, TestFileSystem};
use weyriva::Result;
use weyriva::startup::{Clock, LocalTimezone, Ownership, apply, preflight};

struct FixedClock(i64);

impl Clock for FixedClock {
    fn unix_seconds(&self) -> Result<i64> {
        Ok(self.0)
    }
}

struct FixedTimezone(i32);

impl LocalTimezone for FixedTimezone {
    fn offset_at(&self, _unix_seconds: i64) -> Result<i32> {
        Ok(self.0)
    }
}

#[test]
fn every_completed_mutation_boundary_rolls_back_to_the_exact_pre_state() {
    let baseline = Fixture::new();
    let baseline_plan = baseline.plan();
    apply(&baseline.context, &baseline_plan).expect("baseline apply should succeed");
    let labels = baseline.observer.labels();
    assert!(
        labels.len() >= 10,
        "fixture should exercise the complete startup mutation surface"
    );

    for index in 1..=labels.len() {
        let fixture = Fixture::new();
        let plan = fixture.plan();
        let before = fixture.snapshot();
        fixture.observer.fail_applied(index);

        let error =
            apply(&fixture.context, &plan).expect_err("injected apply failure should be returned");

        assert_eq!(
            error.code(),
            "injected_apply_failure",
            "failure boundary {index}: {}",
            labels[index - 1]
        );
        assert_eq!(
            fixture.snapshot(),
            before,
            "failure boundary {index}: {}",
            labels[index - 1]
        );
    }
}

#[test]
fn partial_systemctl_failure_restores_link_and_effective_enable_state() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let before = fixture.snapshot();
    fixture.process.fail_next_enable_after_mutation();

    let error = apply(&fixture.context, &plan)
        .expect_err("partially applied systemctl failure should be returned");

    assert_eq!(error.code(), "startup_apply_failed");
    assert_eq!(fixture.snapshot(), before);
    assert!(
        !plan
            .backup_root
            .join("systemd/display-manager.service")
            .exists()
    );
}

#[test]
fn rollback_failure_is_typed_and_contains_recovery_evidence() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    fixture.observer.fail_applied(8);
    fixture.observer.fail_rollback(1);

    let error = apply(&fixture.context, &plan).expect_err("rollback failure should be returned");
    let body = error.body();
    let details = body
        .details
        .expect("rollback failure should include structured evidence");

    assert_eq!(body.code, "rollback_failed");
    assert!(details.get("apply_error").is_some());
    assert!(
        details
            .get("rollback_errors")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|errors| !errors.is_empty())
    );
    assert!(details.get("recovery").is_some());
}

#[test]
fn startup_apply_is_idempotent_and_never_restarts_greetd() {
    let fixture = Fixture::new();
    let first_plan = fixture.plan();
    let first = apply(&fixture.context, &first_plan).expect("first apply should succeed");
    let after_first = fixture.snapshot();

    let second_plan =
        preflight(&fixture.context, "tester").expect("second preflight should succeed");
    let second = apply(&fixture.context, &second_plan).expect("second apply should succeed");

    assert!(first.niri_changed);
    assert!(first.greetd_changed);
    assert!(!second.niri_changed);
    assert!(!second.greetd_changed);
    assert_eq!(fixture.snapshot(), after_first);
    assert!(second.output.contains("greetd was not restarted"));
    assert!(fixture.process.commands().iter().all(|command| {
        !command
            .arguments
            .iter()
            .any(|argument| argument == "restart")
    }));
}

#[test]
fn backup_symlink_does_not_change_its_actual_target_owner() {
    let fixture = Fixture::new();
    let before = fs::symlink_metadata(&fixture.old_target).expect("old target should exist");
    let plan = fixture.plan();

    apply(&fixture.context, &plan).expect("apply should succeed");

    let after = fs::symlink_metadata(&fixture.old_target).expect("old target should still exist");
    assert_eq!((after.uid(), after.gid()), (before.uid(), before.gid()));
}

#[test]
fn previous_relative_display_manager_target_is_persisted_as_private_metadata() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let record = plan
        .display_manager_record
        .clone()
        .expect("linked display manager should have a recovery record");

    apply(&fixture.context, &plan).expect("apply should succeed");

    let metadata = fs::symlink_metadata(&record).expect("recovery record should exist");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&record).expect("record should be readable"))
            .expect("record should be valid JSON");
    assert!(metadata.is_file());
    assert!(!metadata.file_type().is_symlink());
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
    assert_eq!(fixture.ownership.owner(&record, &metadata), (1001, 1002));
    assert_eq!(value["format"], "weyriva-display-manager-target-v1");
    assert_eq!(value["target"], "old.service");
}

#[test]
fn absolute_and_dangling_display_manager_targets_are_persisted_exactly() {
    for target in [
        "/usr/lib/systemd/system/sddm.service",
        "missing-display-manager.service",
    ] {
        let fixture = Fixture::new();
        fs::remove_file(&fixture.context.layout.display_manager)
            .expect("old display-manager link should be removed");
        symlink(target, &fixture.context.layout.display_manager)
            .expect("test display-manager link should be created");
        let plan = fixture.plan();
        let record = plan
            .display_manager_record
            .clone()
            .expect("linked display manager should have a recovery record");

        apply(&fixture.context, &plan).expect("apply should succeed");

        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(record).expect("record should be readable"))
                .expect("record should be valid JSON");
        assert_eq!(value["target"], target);
    }
}

#[test]
fn parent_traversal_display_manager_target_is_rejected_before_mutation() {
    let fixture = Fixture::new();
    fs::remove_file(&fixture.context.layout.display_manager)
        .expect("old display-manager link should be removed");
    symlink(
        Path::new("../../../../../../../../unsafe.service"),
        &fixture.context.layout.display_manager,
    )
    .expect("unsafe test link should be created");
    let before = fixture.snapshot();

    let error =
        preflight(&fixture.context, "tester").expect_err("unsafe link target should be rejected");

    assert_eq!(error.code(), "unsafe_startup_path");
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn recovery_record_directory_sync_failure_rolls_back_the_published_file() {
    let mut fixture = Fixture::new();
    let plan = fixture.plan();
    let before = fixture.snapshot();
    let record = plan
        .display_manager_record
        .as_ref()
        .expect("transition should have a recovery record");
    fixture.context.filesystem = Arc::new(TestFileSystem::failing_sync(
        record
            .parent()
            .expect("record should have a parent")
            .to_path_buf(),
    ));

    let error = apply(&fixture.context, &plan)
        .expect_err("post-rename directory sync failure should be returned");

    assert_eq!(error.code(), "io_error");
    assert_eq!(fixture.snapshot(), before);
    assert!(!record.exists());
}

#[test]
fn existing_record_mode_and_owner_are_reconciled_without_changing_content() {
    let mut fixture = Fixture::new();
    let first_plan = fixture.plan();
    apply(&fixture.context, &first_plan).expect("first apply should succeed");
    let record = first_plan
        .display_manager_record
        .expect("first transition should have a recovery record");
    let content = fs::read(&record).expect("record should be readable initially");
    fs::set_permissions(&record, fs::Permissions::from_mode(0o000))
        .expect("test record mode should be changed");
    fixture
        .ownership
        .set_owner(&record, 55, 66)
        .expect("test effective owner should be changed");
    fixture.context.filesystem =
        Arc::new(TestFileSystem::readable(record.clone(), content.clone()));

    let second_plan = fixture.plan();
    apply(&fixture.context, &second_plan).expect("record reconciliation should succeed");

    let metadata = fs::symlink_metadata(&record).expect("record should remain present");
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
    assert_eq!(fixture.ownership.owner(&record, &metadata), (1001, 1002));
    assert_eq!(
        fs::read(record).expect("record should be readable"),
        content
    );
}

#[test]
fn already_greetd_with_changing_timestamps_creates_no_recovery_records() {
    let mut fixture = Fixture::new();
    fs::remove_file(&fixture.context.layout.display_manager)
        .expect("old display-manager link should be removed");
    symlink(
        "../../../../usr/lib/systemd/system/greetd.service",
        &fixture.context.layout.display_manager,
    )
    .expect("relative greetd link should be created");
    fixture
        .context
        .environment
        .insert("WEYRIVA_STARTUP_TIMESTAMP".into(), "20260730-020000".into());
    let first_plan = fixture.plan();
    assert!(first_plan.display_manager_record.is_none());
    apply(&fixture.context, &first_plan).expect("first apply should succeed");

    fixture
        .context
        .environment
        .insert("WEYRIVA_STARTUP_TIMESTAMP".into(), "20260730-030000".into());
    let second_plan = fixture.plan();
    assert!(second_plan.display_manager_record.is_none());
    apply(&fixture.context, &second_plan).expect("second apply should succeed");

    for timestamp in ["20260730-020000", "20260730-030000"] {
        assert!(
            !first_plan
                .user_home
                .join(".local/state/weyriva/startup-backups")
                .join(timestamp)
                .join("systemd/display-manager.service")
                .exists()
        );
    }
}

#[test]
fn timestamp_fallback_uses_injected_local_wall_clock() {
    let mut fixture = Fixture::new();
    fixture
        .context
        .environment
        .remove(OsStr::new("WEYRIVA_STARTUP_TIMESTAMP"));
    fixture.context.clock = Arc::new(FixedClock(1_704_067_199));
    fixture.context.timezone = Arc::new(FixedTimezone(8 * 60 * 60));

    let plan = fixture.plan();

    assert!(
        plan.backup_root
            .ends_with("startup-backups/20240101-075959")
    );
}

#[test]
fn timestamp_override_remains_strictly_validated() {
    let mut fixture = Fixture::new();
    fixture
        .context
        .environment
        .insert("WEYRIVA_STARTUP_TIMESTAMP".into(), "2024-01-01".into());

    let error = preflight(&fixture.context, "tester")
        .expect_err("invalid timestamp override should be rejected");

    assert_eq!(error.code(), "invalid_timestamp");
}
