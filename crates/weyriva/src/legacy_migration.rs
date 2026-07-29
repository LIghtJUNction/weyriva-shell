mod active;
mod copy;
mod installed;
mod journal;
mod layout;
mod lifecycle_lock;
mod validate;

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::error::{Error, Result};
use crate::paths::Paths;

use journal::{Journal, commit, resume};
use layout::MigrationRoots;
pub use lifecycle_lock::MigrationLock;
pub const MAX_MIGRATION_MEMBERS: usize = copy::MAX_MIGRATION_MEMBERS;
pub const MAX_MIGRATION_BYTES: u64 = copy::MAX_MIGRATION_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationOutcome {
    NotNeeded,
    Migrated,
}

pub trait MigrationOps {
    /// Renames one migration root at a commit or rollback boundary.
    ///
    /// # Errors
    ///
    /// Returns an error when the filesystem rename fails.
    fn rename(&self, source: &Path, destination: &Path) -> Result<()>;

    /// Removes one validated private staging tree.
    ///
    /// # Errors
    ///
    /// Returns an error when safe removal cannot be completed.
    fn remove_tree(&self, path: &Path) -> Result<()>;

    /// Test seam immediately after a commit-boundary rename.
    ///
    /// # Errors
    ///
    /// A process interruption test returns `migration_interrupted`.
    fn checkpoint(&self, _boundary: usize) -> Result<()> {
        Ok(())
    }

    /// Test seam after a durable prepare journal and each staging mutation.
    ///
    /// # Errors
    ///
    /// A process interruption test returns `migration_interrupted`.
    fn preparation_checkpoint(&self, _boundary: usize) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SystemMigrationOps;

impl MigrationOps for SystemMigrationOps {
    fn rename(&self, source: &Path, destination: &Path) -> Result<()> {
        fs::rename(source, destination)
            .map_err(|error| Error::io("cannot commit legacy state migration", &error))?;
        sync_parent(source)?;
        if source.parent() != destination.parent() {
            sync_parent(destination)?;
        }
        Ok(())
    }

    fn remove_tree(&self, path: &Path) -> Result<()> {
        copy::remove_private_tree(path)
    }
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("legacy_migration_unsafe", "migration path has no parent"))?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::io("cannot sync migration directory", &error))
}

/// Runs the one-time migration under dedicated migration and daemon locks.
///
/// # Errors
///
/// Returns an error when another daemon/migration is active or migration fails.
pub fn prepare_daemon(paths: &Paths) -> Result<MigrationOutcome> {
    let _guard = MigrationLock::acquire(paths)?;
    migrate(paths)
}

/// Migrates the legacy `plugins-v5` XDG roots once.
///
/// This low-level entry point assumes its caller already serialized daemon
/// startup. Production uses [`prepare_daemon`].
///
/// # Errors
///
/// Returns an error for unsafe, conflicting, invalid, or failed migrations.
pub fn migrate(paths: &Paths) -> Result<MigrationOutcome> {
    migrate_with(paths, &SystemMigrationOps)
}

/// Migrates legacy roots through injected filesystem operations.
///
/// # Errors
///
/// Returns an error for unsafe, conflicting, invalid, interrupted, or failed
/// migrations.
pub fn migrate_with(paths: &Paths, operations: &dyn MigrationOps) -> Result<MigrationOutcome> {
    let roots = MigrationRoots::from_paths(paths)?;
    if roots.journal_exists()? {
        let journal = Journal::read(&roots)?;
        if journal.is_preparing() {
            return restart_preparation(&roots, journal, operations);
        }
        return resume(&roots, journal, operations);
    }
    let state = roots.inspect()?;
    if state.is_completed_migration() {
        active::validate_completed(&roots)?;
        return Ok(MigrationOutcome::NotNeeded);
    }
    if state.has_transient() {
        return Err(needs_recovery(
            "migration staging or recovery exists without a journal",
            &roots,
        ));
    }
    if !state.old.iter().any(|present| *present) {
        return Ok(MigrationOutcome::NotNeeded);
    }
    if state.new.iter().any(|present| *present) {
        return Err(Error::new(
            "legacy_migration_conflict",
            "legacy and unversioned plugin roots both exist",
        ));
    }
    let preparation = validate::analyze(&roots, state.old)?;
    let journal = Journal::new(&roots, state.old);
    journal.write(&roots)?;
    operations.preparation_checkpoint(0)?;
    finish_preparation(&roots, journal, &preparation, operations)
}

fn restart_preparation(
    roots: &MigrationRoots,
    journal: Journal,
    operations: &dyn MigrationOps,
) -> Result<MigrationOutcome> {
    if let Err(error) = validate::cleanup_staged(roots, operations) {
        return Err(preparation_rollback_failed(
            roots,
            &Error::new(
                "migration_prepare_resume",
                "cannot restart interrupted migration preparation",
            ),
            &error,
        ));
    }
    let preparation = match validate::analyze(roots, journal.present()) {
        Ok(preparation) => preparation,
        Err(original) => {
            return match journal::discard(roots) {
                Ok(()) => Err(original),
                Err(rollback) => Err(preparation_rollback_failed(roots, &original, &rollback)),
            };
        }
    };
    finish_preparation(roots, journal, &preparation, operations)
}

fn finish_preparation(
    roots: &MigrationRoots,
    mut journal: Journal,
    preparation: &validate::Preparation,
    operations: &dyn MigrationOps,
) -> Result<MigrationOutcome> {
    match validate::stage(roots, journal.present(), preparation, operations) {
        Ok(()) => {
            journal.mark_ready(roots)?;
            commit(roots, journal, operations)
        }
        Err(error) if error.code() == "migration_interrupted" => Err(error),
        Err(original) => match validate::cleanup_staged(roots, operations) {
            Ok(()) => match journal::discard(roots) {
                Ok(()) => Err(original),
                Err(rollback) => Err(preparation_rollback_failed(roots, &original, &rollback)),
            },
            Err(rollback) => Err(preparation_rollback_failed(roots, &original, &rollback)),
        },
    }
}

fn preparation_rollback_failed(
    roots: &MigrationRoots,
    original: &Error,
    rollback: &Error,
) -> Error {
    Error::with_details(
        "migration_rollback_failed",
        "legacy migration preparation cleanup is incomplete",
        json!({
            "journal": roots.journal,
            "staged": roots.staged,
            "original": original.body(),
            "rollback": rollback.body(),
        }),
    )
}

fn needs_recovery(message: &str, roots: &MigrationRoots) -> Error {
    Error::with_details(
        "migration_needs_recovery",
        message,
        json!({
            "journal": roots.journal,
            "old": roots.old,
            "staged": roots.staged,
            "new": roots.new,
            "recovery": roots.recovery,
        }),
    )
}
