use std::fs::{self, File};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{Error, Result};
use crate::storage::{atomic_json, read_json};

use super::layout::{MigrationRoots, path_exists};
use super::{MigrationOps, MigrationOutcome, needs_recovery};
use super::{active, validate};

const JOURNAL_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    #[default]
    Prepare,
    Quarantine,
    Install,
    Complete,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct Journal {
    schema: u32,
    phase: Phase,
    next: usize,
    present: [bool; 3],
    old: [PathBuf; 3],
    new: [PathBuf; 3],
}

impl Journal {
    pub fn new(roots: &MigrationRoots, present: [bool; 3]) -> Self {
        Self {
            schema: JOURNAL_SCHEMA,
            phase: Phase::Prepare,
            next: 0,
            present,
            old: roots.old.clone(),
            new: roots.new.clone(),
        }
    }

    pub fn read(roots: &MigrationRoots) -> Result<Self> {
        let journal: Self = read_json(&roots.journal).map_err(|error| {
            Error::with_details(
                "migration_needs_recovery",
                "migration journal is unreadable; preserved state requires recovery",
                json!({"journal": roots.journal, "cause": error.body()}),
            )
        })?;
        journal.validate(roots)?;
        Ok(journal)
    }

    pub fn write(&self, roots: &MigrationRoots) -> Result<()> {
        atomic_json(&roots.journal, self)
    }

    pub fn is_preparing(&self) -> bool {
        self.phase == Phase::Prepare
    }

    pub fn present(&self) -> [bool; 3] {
        self.present
    }

    pub fn mark_ready(&mut self, roots: &MigrationRoots) -> Result<()> {
        self.phase = Phase::Quarantine;
        self.next = 0;
        self.write(roots)
    }

    fn validate(&self, roots: &MigrationRoots) -> Result<()> {
        if self.schema != JOURNAL_SCHEMA
            || self.old != roots.old
            || self.new != roots.new
            || self.next > 3
        {
            return Err(needs_recovery(
                "migration journal does not match the requested XDG roots",
                roots,
            ));
        }
        for index in 0..3 {
            let old = path_exists(&roots.old[index])?;
            let recovery = path_exists(&roots.recovery[index])?;
            let preserved = if self.phase == Phase::Prepare {
                self.present[index] == old && !recovery
            } else {
                (self.present[index] && old != recovery)
                    || (!self.present[index] && !old && !recovery)
            };
            if !preserved {
                return Err(needs_recovery(
                    "migration journal presence does not match preserved roots",
                    roots,
                ));
            }
            let staged = path_exists(&roots.staged[index])?;
            let published = path_exists(&roots.new[index])?;
            let valid_copy = if self.phase == Phase::Prepare {
                !published
            } else {
                staged != published
            };
            if !valid_copy {
                return Err(needs_recovery(
                    "migration journal has an invalid staged/published root pair",
                    roots,
                ));
            }
        }
        Ok(())
    }
}

pub fn discard(roots: &MigrationRoots) -> Result<()> {
    remove_journal(&roots.journal)
}

pub fn commit(
    roots: &MigrationRoots,
    journal: Journal,
    operations: &dyn MigrationOps,
) -> Result<MigrationOutcome> {
    run(roots, journal, operations)
}

pub fn resume(
    roots: &MigrationRoots,
    journal: Journal,
    operations: &dyn MigrationOps,
) -> Result<MigrationOutcome> {
    run(roots, journal, operations)
}

fn run(
    roots: &MigrationRoots,
    mut journal: Journal,
    operations: &dyn MigrationOps,
) -> Result<MigrationOutcome> {
    let result: Result<MigrationOutcome> = (|| {
        active::validate_resumable(roots, journal.present)?;
        journal.phase = Phase::Quarantine;
        journal.write(roots)?;
        for index in 0..3 {
            if journal.present[index] {
                advance(
                    &roots.old[index],
                    &roots.recovery[index],
                    index,
                    roots,
                    &mut journal,
                    operations,
                )?;
            }
        }
        journal.phase = Phase::Install;
        journal.next = 0;
        journal.write(roots)?;
        for index in 0..3 {
            advance(
                &roots.staged[index],
                &roots.new[index],
                index + 3,
                roots,
                &mut journal,
                operations,
            )?;
        }
        validate::validate_published(roots)?;
        journal.phase = Phase::Complete;
        journal.next = 3;
        journal.write(roots)?;
        remove_journal(&roots.journal)?;
        Ok(MigrationOutcome::Migrated)
    })();
    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) if error.code() == "migration_interrupted" => Err(error),
        Err(error) => rollback(roots, &journal, operations, error),
    }
}

fn advance(
    source: &Path,
    destination: &Path,
    boundary: usize,
    roots: &MigrationRoots,
    journal: &mut Journal,
    operations: &dyn MigrationOps,
) -> Result<()> {
    match (path_exists(source)?, path_exists(destination)?) {
        (true, false) => {
            operations.rename(source, destination)?;
            operations.checkpoint(boundary)?;
        }
        (false, true) => {}
        (true, true) => {
            return Err(needs_recovery(
                "both sides of a migration boundary exist",
                roots,
            ));
        }
        (false, false) => {
            return Err(needs_recovery(
                "both sides of a migration boundary are missing",
                roots,
            ));
        }
    }
    journal.next = boundary % 3 + 1;
    journal.write(roots)
}

fn rollback(
    roots: &MigrationRoots,
    journal: &Journal,
    operations: &dyn MigrationOps,
    original: Error,
) -> Result<MigrationOutcome> {
    let mut failures = Vec::new();
    for index in (0..3).rev() {
        reverse(
            &roots.new[index],
            &roots.staged[index],
            operations,
            &mut failures,
        );
    }
    for index in (0..3).rev().filter(|index| journal.present[*index]) {
        reverse(
            &roots.recovery[index],
            &roots.old[index],
            operations,
            &mut failures,
        );
    }
    if failures.is_empty() {
        for path in &roots.staged {
            if let Err(error) = operations.remove_tree(path) {
                failures.push(error.to_string());
            }
        }
    }
    if failures.is_empty()
        && let Err(error) = remove_journal(&roots.journal)
    {
        failures.push(error.to_string());
    }
    if failures.is_empty() {
        Err(original)
    } else {
        Err(Error::with_details(
            "migration_rollback_failed",
            "legacy migration failed and automatic rollback is incomplete",
            json!({"original": original.body(), "rollback": failures}),
        ))
    }
}

fn reverse(
    source: &Path,
    destination: &Path,
    operations: &dyn MigrationOps,
    failures: &mut Vec<String>,
) {
    match (path_exists(source), path_exists(destination)) {
        (Ok(true), Ok(false)) => {
            if let Err(error) = operations.rename(source, destination) {
                failures.push(error.to_string());
            }
        }
        (Ok(false), Ok(_)) => {}
        (Ok(true), Ok(true)) => failures.push(format!(
            "rollback conflict: {} and {} both exist",
            source.display(),
            destination.display()
        )),
        (Err(error), _) | (_, Err(error)) => failures.push(error.to_string()),
    }
}

fn remove_journal(path: &Path) -> Result<()> {
    fs::remove_file(path).map_err(|error| Error::io("cannot remove migration journal", &error))?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("legacy_migration_unsafe", "journal has no parent"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::io("cannot sync migration journal directory", &error))
}
