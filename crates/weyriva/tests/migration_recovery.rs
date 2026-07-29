#[path = "support/migration.rs"]
mod support;

use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs2::FileExt;
use support::{MigrationFixture, assert_absent};
use weyriva::legacy_migration::{
    MigrationLock, MigrationOps, MigrationOutcome, SystemMigrationOps, migrate, migrate_with,
};
use weyriva::{Error, Result};

struct InterruptAt {
    boundary: usize,
}

impl MigrationOps for InterruptAt {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        SystemMigrationOps.remove_tree(path)
    }

    fn checkpoint(&self, boundary: usize) -> Result<()> {
        if boundary == self.boundary {
            Err(Error::new(
                "migration_interrupted",
                "simulated process interruption",
            ))
        } else {
            Ok(())
        }
    }
}

struct FailRenames {
    calls: AtomicUsize,
    fail: Vec<usize>,
}

struct PrepareProbe {
    interrupt: Option<usize>,
    checkpoints: AtomicUsize,
}

impl PrepareProbe {
    fn counting() -> Self {
        Self {
            interrupt: None,
            checkpoints: AtomicUsize::new(0),
        }
    }

    fn interrupt_at(boundary: usize) -> Self {
        Self {
            interrupt: Some(boundary),
            checkpoints: AtomicUsize::new(0),
        }
    }
}

impl MigrationOps for PrepareProbe {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        SystemMigrationOps.remove_tree(path)
    }

    fn preparation_checkpoint(&self, boundary: usize) -> Result<()> {
        self.checkpoints.fetch_add(1, Ordering::SeqCst);
        if self.interrupt == Some(boundary) {
            Err(Error::new(
                "migration_interrupted",
                "simulated preparation interruption",
            ))
        } else {
            Ok(())
        }
    }
}

struct FailCleanup;

impl MigrationOps for FailCleanup {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        Err(Error::new(
            "injected_cleanup_failure",
            format!("cannot remove {}", path.display()),
        ))
    }
}

impl MigrationOps for FailRenames {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail.contains(&call) {
            return Err(Error::new(
                "injected_rename_failure",
                format!("rename call {call} failed"),
            ));
        }
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        SystemMigrationOps.remove_tree(path)
    }
}

#[test]
fn every_commit_boundary_resumes_from_durable_journal() {
    for boundary in 0..6 {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", boundary % 2 == 0, "General");

        let error = migrate_with(&fixture.paths, &InterruptAt { boundary })
            .expect_err("simulated interruption should surface");

        assert_eq!(error.code(), "migration_interrupted", "{boundary}");
        assert!(fixture.journal.is_file(), "{boundary}");
        assert_eq!(
            migrate(&fixture.paths).expect("journal recovery should succeed"),
            MigrationOutcome::Migrated,
            "{boundary}"
        );
        assert!(fixture.new.iter().all(|path| path.is_dir()), "{boundary}");
        assert!(!fixture.journal.exists(), "{boundary}");
        assert_absent(&fixture.staged);
    }
}

#[test]
fn every_preparation_boundary_restarts_from_durable_journal() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", true, "General");
    let counter = PrepareProbe::counting();
    migrate_with(&fixture.paths, &counter).expect("counted migration should succeed");
    let boundaries = counter.checkpoints.load(Ordering::SeqCst);
    assert!(
        boundaries >= 10,
        "fixture must exercise roots, dirs, and files"
    );

    for boundary in 0..boundaries {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", boundary % 2 == 0, "General");
        let error = migrate_with(&fixture.paths, &PrepareProbe::interrupt_at(boundary))
            .expect_err("preparation interruption should surface");

        assert_eq!(error.code(), "migration_interrupted", "{boundary}");
        assert!(fixture.journal.is_file(), "{boundary}");
        assert!(fixture.old.iter().all(|path| path.is_dir()), "{boundary}");
        assert_eq!(
            migrate(&fixture.paths).expect("prepare recovery should restart and succeed"),
            MigrationOutcome::Migrated,
            "{boundary}"
        );
        assert_absent(&fixture.staged);
        assert!(!fixture.journal.exists(), "{boundary}");
    }
}

#[test]
fn interrupted_preparation_cleanup_failure_preserves_recovery_evidence() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", false, "General");
    let error = migrate_with(&fixture.paths, &PrepareProbe::interrupt_at(2))
        .expect_err("preparation should be interrupted after staging starts");
    assert_eq!(error.code(), "migration_interrupted");

    let error = migrate_with(&fixture.paths, &FailCleanup)
        .expect_err("failed restart cleanup should be explicit");

    assert_eq!(error.code(), "migration_rollback_failed");
    let details = error.body().details.expect("recovery details should exist");
    assert!(details.get("journal").is_some());
    assert!(details.get("staged").is_some());
    assert!(details.get("rollback").is_some());
    assert!(fixture.journal.is_file());
}

#[test]
fn ordinary_commit_failure_rolls_back_to_original_roots() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", false, "General");
    let operations = FailRenames {
        calls: AtomicUsize::new(0),
        fail: vec![2],
    };

    let error =
        migrate_with(&fixture.paths, &operations).expect_err("rename failure should surface");

    assert_eq!(error.code(), "injected_rename_failure");
    assert!(fixture.old.iter().all(|path| path.is_dir()));
    assert_absent(&fixture.new);
    assert_absent(&fixture.staged);
    assert_absent(&fixture.recovery);
    assert!(!fixture.journal.exists());
}

#[test]
fn rollback_failure_is_explicit_and_next_run_recovers_forward() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", true, "General");
    let operations = FailRenames {
        calls: AtomicUsize::new(0),
        fail: vec![2, 3],
    };

    let error =
        migrate_with(&fixture.paths, &operations).expect_err("rollback failure should surface");

    assert_eq!(error.code(), "migration_rollback_failed");
    assert!(fixture.journal.is_file());
    assert_eq!(
        migrate(&fixture.paths).expect("incomplete rollback should recover"),
        MigrationOutcome::Migrated
    );
    assert!(fixture.new.iter().all(|path| path.is_dir()));
    assert!(!fixture.journal.exists());
}

#[test]
fn orphan_recovery_is_an_explicit_needs_recovery_state() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(&fixture.recovery[0]).expect("recovery root should be created");

    let error = migrate(&fixture.paths).expect_err("orphan recovery should not be ignored");

    assert_eq!(error.code(), "migration_needs_recovery");
    assert!(fixture.recovery[0].is_dir());
}

#[test]
fn malformed_journal_is_never_treated_as_noop() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(
        fixture
            .journal
            .parent()
            .expect("journal parent should exist"),
    )
    .expect("journal parent should be created");
    fs::write(&fixture.journal, b"{").expect("malformed journal should be written");

    let error = migrate(&fixture.paths).expect_err("malformed journal should fail");

    assert_eq!(error.code(), "migration_needs_recovery");
    assert!(fixture.journal.is_file());
}

#[test]
fn journal_presence_must_match_preserved_roots() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", false, "General");
    let error = migrate_with(&fixture.paths, &InterruptAt { boundary: 0 })
        .expect_err("interruption should preserve a journal");
    assert_eq!(error.code(), "migration_interrupted");

    let mut journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.journal).expect("journal should be readable"))
            .expect("journal should be valid JSON");
    journal["present"][0] = serde_json::Value::Bool(false);
    fs::write(
        &fixture.journal,
        serde_json::to_vec(&journal).expect("journal should encode"),
    )
    .expect("journal should be replaced");

    let error = migrate(&fixture.paths).expect_err("mismatched journal should need recovery");

    assert_eq!(error.code(), "migration_needs_recovery");
    assert!(fixture.recovery[0].is_dir());
    assert!(fixture.journal.is_file());
}

#[test]
fn dedicated_lock_rejects_concurrent_migration() {
    let fixture = MigrationFixture::empty();
    let first = MigrationLock::acquire(&fixture.paths).expect("first lock should succeed");

    let error = MigrationLock::acquire(&fixture.paths)
        .err()
        .expect("second lock should fail");

    assert_eq!(error.code(), "migration_busy");
    drop(first);
    MigrationLock::acquire(&fixture.paths).expect("released lock should be reusable");
}

#[test]
fn daemon_lock_holder_blocks_migration_before_state_access() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(&fixture.paths.runtime_dir).expect("runtime directory should be created");
    let daemon = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(fixture.paths.daemon_lock_file())
        .expect("daemon lock should open");
    daemon
        .try_lock_exclusive()
        .expect("daemon lock should be held");

    let error = MigrationLock::acquire(&fixture.paths)
        .err()
        .expect("active daemon should block migration");

    assert_eq!(error.code(), "migration_daemon_active");
}

#[test]
fn daemon_owner_can_acquire_migration_lock_in_order() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(&fixture.paths.runtime_dir).expect("runtime directory should be created");
    let daemon = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(fixture.paths.daemon_lock_file())
        .expect("daemon lock should open");
    daemon
        .try_lock_exclusive()
        .expect("daemon lock should be held");

    let migration = MigrationLock::acquire_after_daemon(&fixture.paths)
        .expect("daemon owner should acquire migration lock");
    let error = MigrationLock::acquire_after_daemon(&fixture.paths)
        .err()
        .expect("second migration lock should fail");

    assert_eq!(error.code(), "migration_busy");
    drop(migration);
}
