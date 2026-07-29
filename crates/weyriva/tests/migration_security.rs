#[path = "support/migration.rs"]
mod support;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use support::{MigrationFixture, assert_absent};
use weyriva::legacy_migration::{
    MAX_MIGRATION_BYTES, MAX_MIGRATION_MEMBERS, MigrationOps, SystemMigrationOps, migrate,
    migrate_with,
};
use weyriva::{Error, Result};

struct InterruptAfterFirstQuarantine;
struct InterruptBeforeStaging;

impl MigrationOps for InterruptBeforeStaging {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        SystemMigrationOps.remove_tree(path)
    }

    fn preparation_checkpoint(&self, boundary: usize) -> Result<()> {
        if boundary == 0 {
            Err(Error::new(
                "migration_interrupted",
                "simulated pre-staging interruption",
            ))
        } else {
            Ok(())
        }
    }
}

impl MigrationOps for InterruptAfterFirstQuarantine {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        SystemMigrationOps.rename(source, destination)
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        SystemMigrationOps.remove_tree(path)
    }

    fn checkpoint(&self, boundary: usize) -> Result<()> {
        if boundary == 0 {
            Err(Error::new(
                "migration_interrupted",
                "simulated commit interruption",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn symlinks_hardlinks_and_special_files_are_rejected() {
    for defect in ["symlink", "hardlink", "fifo"] {
        let fixture = MigrationFixture::legacy();
        let source = fixture.old[0].join("source");
        fs::write(&source, "fixture\n").expect("source should be written");
        match defect {
            "symlink" => {
                symlink(&source, fixture.old[0].join("bad")).expect("symlink should be created");
            }
            "hardlink" => {
                fs::hard_link(&source, fixture.old[0].join("bad"))
                    .expect("hardlink should be created");
            }
            "fifo" => {
                mkfifo(&fixture.old[0].join("bad"), Mode::S_IRUSR | Mode::S_IWUSR)
                    .expect("fifo should be created");
            }
            _ => unreachable!(),
        }

        let error = migrate(&fixture.paths).expect_err("unsafe member should fail");

        assert_eq!(error.code(), "legacy_migration_unsafe", "{defect}");
        assert_absent(&fixture.staged);
        assert_absent(&fixture.recovery);
    }
}

#[test]
fn symlinked_legacy_root_and_parent_are_rejected() {
    let fixture = MigrationFixture::empty();
    let target = fixture.old[0]
        .parent()
        .expect("root parent should exist")
        .join("target");
    fs::create_dir_all(&target).expect("target should be created");
    fs::create_dir_all(
        fixture.old[0]
            .parent()
            .expect("legacy root should have a parent"),
    )
    .expect("legacy parent should be created");
    symlink(&target, &fixture.old[0]).expect("legacy root symlink should be created");

    let error = migrate(&fixture.paths).expect_err("root symlink should fail");

    assert_eq!(error.code(), "legacy_migration_unsafe");
    assert_absent(&fixture.staged);
}

#[test]
fn aggregate_member_limit_split_across_logical_roots_fails_before_copy() {
    let fixture = MigrationFixture::legacy();
    for (root, prefix) in [(&fixture.old[0], "config"), (&fixture.old[1], "state")] {
        for index in 0..MAX_MIGRATION_MEMBERS / 2 {
            fs::write(root.join(format!("{prefix}-{index}")), b"")
                .expect("split member should be written");
        }
    }

    let error = migrate(&fixture.paths).expect_err("member limit should fail");

    assert_eq!(error.code(), "legacy_migration_too_large");
    assert_absent(&fixture.staged);
    assert!(!fixture.journal.exists());
}

#[test]
fn aggregate_byte_limit_split_across_logical_roots_fails_before_copy() {
    let fixture = MigrationFixture::legacy();
    for (root, name) in [
        (&fixture.old[0], "config-large"),
        (&fixture.old[1], "state-large"),
    ] {
        let file = fs::File::create(root.join(name)).expect("split file should be created");
        file.set_len(MAX_MIGRATION_BYTES / 2 + 1)
            .expect("split file should be sized");
    }

    let error = migrate(&fixture.paths).expect_err("byte limit should fail");

    assert_eq!(error.code(), "legacy_migration_too_large");
    assert_absent(&fixture.staged);
    assert!(!fixture.journal.exists());
}

#[test]
fn resumable_member_budget_does_not_double_count_the_copy() {
    let fixture = MigrationFixture::legacy();
    let error = migrate_with(&fixture.paths, &InterruptAfterFirstQuarantine)
        .expect_err("baseline migration should be interrupted");
    assert_eq!(error.code(), "migration_interrupted");
    for (root, prefix) in [
        (&fixture.staged[0], "staged"),
        (&fixture.recovery[0], "recovery"),
    ] {
        for index in 0..MAX_MIGRATION_MEMBERS / 2 {
            fs::write(root.join(format!("{prefix}-{index}")), b"")
                .expect("split member should be written");
        }
    }

    let outcome = migrate(&fixture.paths).expect("each logical side is within the member limit");

    assert_eq!(
        outcome,
        weyriva::legacy_migration::MigrationOutcome::Migrated
    );
    assert!(!fixture.journal.exists());
}

#[test]
fn resumable_byte_budget_does_not_double_count_the_copy() {
    let fixture = MigrationFixture::legacy();
    let error = migrate_with(&fixture.paths, &InterruptAfterFirstQuarantine)
        .expect_err("baseline migration should be interrupted");
    assert_eq!(error.code(), "migration_interrupted");
    for (root, name) in [
        (&fixture.staged[0], "staged-large"),
        (&fixture.recovery[0], "recovery-large"),
    ] {
        let file = fs::File::create(root.join(name)).expect("split file should be created");
        file.set_len(MAX_MIGRATION_BYTES / 2 + 1)
            .expect("split file should be sized");
    }

    let outcome = migrate(&fixture.paths).expect("each logical side is within the byte limit");

    assert_eq!(
        outcome,
        weyriva::legacy_migration::MigrationOutcome::Migrated
    );
    assert!(!fixture.journal.exists());
}

#[test]
fn journaled_budget_failure_rolls_back_and_allows_a_clean_retry() {
    let fixture = MigrationFixture::legacy();
    let error = migrate_with(&fixture.paths, &InterruptAfterFirstQuarantine)
        .expect_err("baseline migration should be interrupted");
    assert_eq!(error.code(), "migration_interrupted");
    for (root, prefix) in [
        (&fixture.staged[0], "config"),
        (&fixture.staged[1], "state"),
    ] {
        for index in 0..MAX_MIGRATION_MEMBERS / 2 {
            fs::write(root.join(format!("{prefix}-{index}")), b"")
                .expect("split staged member should be written");
        }
    }

    let error = migrate(&fixture.paths).expect_err("destination budget overflow should fail");

    assert_eq!(error.code(), "legacy_migration_too_large");
    assert!(fixture.old.iter().all(|path| path.is_dir()));
    assert_absent(&fixture.staged);
    assert_absent(&fixture.recovery);
    assert!(!fixture.journal.exists());
    migrate(&fixture.paths).expect("restored legacy roots should retry cleanly");
}

#[test]
fn recovery_rejects_stray_members_on_either_data_side_and_rolls_back() {
    for side in ["source", "destination"] {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", false, "General");
        let error = migrate_with(&fixture.paths, &InterruptAfterFirstQuarantine)
            .expect_err("baseline migration should be interrupted");
        assert_eq!(error.code(), "migration_interrupted");
        let root = if side == "source" {
            &fixture.old[2]
        } else {
            &fixture.staged[2]
        };
        fs::create_dir(root.join("cache")).expect("stray data directory should be created");

        let error = migrate(&fixture.paths).expect_err("stray recovery data should fail");

        assert_eq!(error.code(), "legacy_migration_inconsistent", "{side}");
        assert!(fixture.old.iter().all(|path| path.is_dir()), "{side}");
        assert_absent(&fixture.staged);
        assert_absent(&fixture.recovery);
        assert!(!fixture.journal.exists(), "{side}");
    }
}

#[test]
fn prepare_resume_validation_error_discards_journal_without_poisoning_state() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", false, "General");
    let error = migrate_with(&fixture.paths, &InterruptBeforeStaging)
        .expect_err("preparation should be interrupted before staging");
    assert_eq!(error.code(), "migration_interrupted");
    fs::create_dir(fixture.old[2].join("cache")).expect("stray source directory should be created");

    let error = migrate(&fixture.paths).expect_err("changed source should fail revalidation");

    assert_eq!(error.code(), "legacy_migration_inconsistent");
    assert!(!fixture.journal.exists());
    assert_absent(&fixture.staged);
    assert!(fixture.old.iter().all(|path| path.is_dir()));
}

#[test]
fn unsafe_transient_paths_are_never_treated_as_noop() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(&fixture.staged[0]).expect("orphan staging should be created");

    let error = migrate(&fixture.paths).expect_err("orphan staging should need recovery");

    assert_eq!(error.code(), "migration_needs_recovery");
    assert!(fixture.staged[0].is_dir());
}
