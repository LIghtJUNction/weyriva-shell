use std::path::Path;

use crate::error::Result;

use super::copy::{Budget, validate_tree};
use super::layout::{MigrationRoots, path_exists};
use super::{needs_recovery, validate};

pub(super) fn validate_completed(roots: &MigrationRoots) -> Result<()> {
    let present = [
        path_exists(&roots.recovery[0])?,
        path_exists(&roots.recovery[1])?,
        path_exists(&roots.recovery[2])?,
    ];
    let mut source_budget = Budget::default();
    for (index, path) in roots.recovery.iter().enumerate() {
        if present[index] {
            validate_tree(path, &mut source_budget)?;
        }
    }
    let mut destination_budget = Budget::default();
    for path in &roots.new {
        validate_tree(path, &mut destination_budget)?;
    }
    validate::validate_preserved(
        roots,
        present,
        &roots.recovery[0],
        &roots.recovery[1],
        &roots.recovery[2],
    )?;
    validate::validate_destination(roots, &roots.new[1], &roots.new[2])
}

pub(super) fn validate_resumable(roots: &MigrationRoots, present: [bool; 3]) -> Result<()> {
    let mut source = [
        roots.old[0].as_path(),
        roots.old[1].as_path(),
        roots.old[2].as_path(),
    ];
    let mut source_budget = Budget::default();
    for index in 0..3 {
        if present[index] {
            source[index] = active_path(&roots.old[index], &roots.recovery[index], roots)?;
            validate_tree(source[index], &mut source_budget)?;
        }
    }

    let mut destination = [
        roots.staged[0].as_path(),
        roots.staged[1].as_path(),
        roots.staged[2].as_path(),
    ];
    let mut destination_budget = Budget::default();
    for (index, active) in destination.iter_mut().enumerate() {
        *active = active_path(&roots.staged[index], &roots.new[index], roots)?;
        validate_tree(active, &mut destination_budget)?;
    }

    validate::validate_preserved(roots, present, source[0], source[1], source[2])?;
    validate::validate_destination(roots, destination[1], destination[2])
}

fn active_path<'a>(first: &'a Path, second: &'a Path, roots: &MigrationRoots) -> Result<&'a Path> {
    match (path_exists(first)?, path_exists(second)?) {
        (true, false) => Ok(first),
        (false, true) => Ok(second),
        _ => Err(needs_recovery(
            "migration has an invalid active root pair",
            roots,
        )),
    }
}
