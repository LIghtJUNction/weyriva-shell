use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::manifest::valid_plugin_id;
use crate::model::{SOURCES_SCHEMA, STATE_SCHEMA, SourcesDocument, StateDocument};
use crate::storage::{atomic_json, read_json};

use super::MigrationOps;
use super::copy::{Budget, copy_root, create_private_root, validate_tree};
use super::installed::{canonical_digest, validate_closure, validate_slot};
use super::layout::{MigrationRoots, path_exists};

pub(super) struct Preparation {
    state: StateDocument,
    state_exists: bool,
}

pub(super) fn analyze(roots: &MigrationRoots, present: [bool; 3]) -> Result<Preparation> {
    ensure_destinations_absent(roots)?;
    let mut budget = Budget::default();
    for (index, path) in roots.old.iter().enumerate() {
        if present[index] {
            validate_tree(path, &mut budget)?;
        }
    }
    load_legacy(roots, present, &roots.old[0], &roots.old[1], &roots.old[2])
}

fn load_legacy(
    roots: &MigrationRoots,
    present: [bool; 3],
    config_root: &Path,
    state_root: &Path,
    data_root: &Path,
) -> Result<Preparation> {
    validate_sources(config_root, present[0])?;
    let state_path = state_root.join("state.json");
    let state_exists = present[1] && regular_file_exists(&state_path)?;
    let mut state = if state_exists {
        read_json::<StateDocument>(&state_path)?
    } else {
        StateDocument::default()
    };
    if state.schema != STATE_SCHEMA {
        return Err(Error::new(
            "unsupported_state",
            "legacy plugin state schema must be 1",
        ));
    }
    if !state_exists && present[2] && tree_has_members(data_root)? {
        return Err(Error::new(
            "legacy_migration_inconsistent",
            "legacy plugin data exists without a state document",
        ));
    }
    hydrate_state(&mut state, &roots.old[2], &roots.new[2], data_root)?;
    Ok(Preparation {
        state,
        state_exists,
    })
}

pub(super) fn stage(
    roots: &MigrationRoots,
    present: [bool; 3],
    preparation: &Preparation,
    operations: &dyn MigrationOps,
) -> Result<()> {
    let mut boundary = 1;
    for (index, is_present) in present.into_iter().enumerate() {
        if is_present {
            copy_root(
                &roots.old[index],
                &roots.staged[index],
                index,
                operations,
                &mut boundary,
            )?;
        } else {
            create_private_root(&roots.staged[index])?;
            operations.preparation_checkpoint(boundary)?;
            boundary = boundary.saturating_add(1);
        }
    }
    if preparation.state_exists {
        atomic_json(&roots.staged[1].join("state.json"), &preparation.state)?;
        operations.preparation_checkpoint(boundary)?;
    }
    validate_state(
        &roots.staged[1].join("state.json"),
        &roots.new[2],
        &roots.staged[2],
        preparation.state_exists,
    )
}

pub(super) fn validate_published(roots: &MigrationRoots) -> Result<()> {
    validate_destination(roots, &roots.new[1], &roots.new[2])
}

pub(super) fn validate_preserved(
    roots: &MigrationRoots,
    present: [bool; 3],
    config_root: &Path,
    state_root: &Path,
    data_root: &Path,
) -> Result<()> {
    load_legacy(roots, present, config_root, state_root, data_root).map(drop)
}

pub(super) fn validate_destination(
    roots: &MigrationRoots,
    state_root: &Path,
    data_root: &Path,
) -> Result<()> {
    let state_path = state_root.join("state.json");
    validate_state(
        &state_path,
        &roots.new[2],
        data_root,
        regular_file_exists(&state_path)?,
    )
}

fn validate_sources(root: &Path, present: bool) -> Result<()> {
    if !present {
        return Ok(());
    }
    let path = root.join("sources.json");
    if !regular_file_exists(&path)? {
        return Ok(());
    }
    let document: SourcesDocument = read_json(&path)?;
    if document.schema != SOURCES_SCHEMA {
        return Err(Error::new(
            "unsupported_state",
            "legacy plugin sources schema must be 1",
        ));
    }
    Ok(())
}

fn hydrate_state(
    state: &mut StateDocument,
    expected_data: &Path,
    destination_data: &Path,
    actual_data: &Path,
) -> Result<()> {
    let mut slots = BTreeSet::new();
    for (plugin_id, record) in &mut state.plugins {
        if !valid_plugin_id(plugin_id) || record.id != *plugin_id || !record.installed {
            return Err(Error::new(
                "legacy_migration_inconsistent",
                "plugin state contains a non-canonical id or uninstalled record",
            ));
        }
        if !canonical_digest(&record.digest) {
            return Err(Error::new(
                "legacy_migration_inconsistent",
                format!("plugin {plugin_id} has an invalid digest"),
            ));
        }
        let relative = Path::new("installed").join(plugin_id).join(&record.digest);
        let expected = expected_data.join(&relative);
        if record.path != expected || !slots.insert(relative.clone()) {
            return Err(Error::new(
                "legacy_migration_inconsistent",
                format!("plugin {plugin_id} is not in its exact immutable slot"),
            ));
        }
        let candidate = validate_slot(actual_data, plugin_id, &record.digest)?;
        if let Some(digest) = record.last_known_good.as_deref() {
            if !canonical_digest(digest) {
                return Err(Error::new(
                    "legacy_migration_inconsistent",
                    format!("plugin {plugin_id} has an invalid last-known-good digest"),
                ));
            }
            if digest != record.digest {
                let relative = Path::new("installed").join(plugin_id).join(digest);
                if !slots.insert(relative) {
                    return Err(Error::new(
                        "legacy_migration_inconsistent",
                        format!("plugin {plugin_id} has a duplicate immutable slot reference"),
                    ));
                }
                validate_slot(actual_data, plugin_id, digest)?;
            }
        }
        record.path = destination_data.join(relative);
        record.version.clone_from(&candidate.provider.version);
        record.provider = candidate.provider;
        record.settings_defaults = candidate.settings_defaults;
    }
    validate_closure(actual_data, &slots)?;
    Ok(())
}

fn validate_state(
    state_path: &Path,
    expected_data: &Path,
    actual_data: &Path,
    exists: bool,
) -> Result<()> {
    if !exists {
        return if tree_has_members(actual_data)? {
            Err(Error::new(
                "legacy_migration_inconsistent",
                "migrated plugin data exists without state",
            ))
        } else {
            Ok(())
        };
    }
    let mut state: StateDocument = read_json(state_path)?;
    if state.schema != STATE_SCHEMA {
        return Err(Error::new(
            "unsupported_state",
            "migrated plugin state schema must be 1",
        ));
    }
    let original = state.clone();
    hydrate_state(&mut state, expected_data, expected_data, actual_data)?;
    if !same_hydrated_state(&original, &state) {
        return Err(Error::new(
            "legacy_migration_inconsistent",
            "migrated state is not hydrated from its installed manifests",
        ));
    }
    Ok(())
}

fn same_hydrated_state(left: &StateDocument, right: &StateDocument) -> bool {
    match (serde_json::to_value(left), serde_json::to_value(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn ensure_destinations_absent(roots: &MigrationRoots) -> Result<()> {
    for path in roots
        .new
        .iter()
        .chain(roots.staged.iter())
        .chain(roots.recovery.iter())
    {
        if path_exists(path)? {
            return Err(Error::new(
                "legacy_migration_conflict",
                format!("migration destination already exists: {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn regular_file_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(Error::new(
                "legacy_migration_unsafe",
                format!("migration state file is unsafe: {}", path.display()),
            ))
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("cannot inspect migration state file", &error)),
    }
}

fn tree_has_members(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(Error::new(
            "legacy_migration_unsafe",
            format!("migration data root is unsafe: {}", path.display()),
        )),
        Ok(_) => fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            .map_err(|error| Error::io("cannot inspect migration data root", &error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("cannot inspect migration data root", &error)),
    }
}

pub(super) fn cleanup_staged(roots: &MigrationRoots, operations: &dyn MigrationOps) -> Result<()> {
    let mut failures = Vec::new();
    for path in &roots.staged {
        if let Err(error) = operations.remove_tree(path) {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::with_details(
            "migration_rollback_failed",
            "cannot clean failed migration staging",
            serde_json::json!({"rollback": failures}),
        ))
    }
}
