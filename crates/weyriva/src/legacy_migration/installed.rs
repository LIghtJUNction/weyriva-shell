use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::manifest::{parse_plugin, valid_plugin_id};
use crate::model::Candidate;
use crate::tree::validate_and_hash;

use super::layout::path_exists;

pub(super) fn validate_closure(data_root: &Path, expected: &BTreeSet<PathBuf>) -> Result<()> {
    let Some(installed) = installed_root(data_root, expected)? else {
        return Ok(());
    };

    let authors = child_directories(&installed)?;
    if authors.is_empty() {
        return Err(inconsistent(
            "installed contains an unreferenced empty tree",
        ));
    }
    let mut found = BTreeSet::new();
    for (author, author_path) in authors {
        let plugins = child_directories(&author_path)?;
        if plugins.is_empty() {
            return Err(inconsistent(
                "installed contains an unreferenced author directory",
            ));
        }
        for (plugin, plugin_path) in plugins {
            let plugin_id = format!("{author}/{plugin}");
            if !valid_plugin_id(&plugin_id) {
                return Err(inconsistent(
                    "installed contains a non-canonical plugin path",
                ));
            }
            let digests = child_directories(&plugin_path)?;
            if digests.is_empty() {
                return Err(inconsistent(
                    "installed contains an unreferenced plugin directory",
                ));
            }
            for (digest, _) in digests {
                if !canonical_digest(&digest) {
                    return Err(inconsistent(
                        "installed contains a non-canonical digest directory",
                    ));
                }
                let relative = Path::new("installed").join(&plugin_id).join(digest);
                if !expected.contains(&relative) {
                    return Err(inconsistent(
                        "installed contains an unreferenced immutable slot",
                    ));
                }
                found.insert(relative);
            }
        }
    }
    if found == *expected {
        Ok(())
    } else {
        Err(inconsistent(
            "plugin state does not reference every installed immutable slot",
        ))
    }
}

fn installed_root(data_root: &Path, expected: &BTreeSet<PathBuf>) -> Result<Option<PathBuf>> {
    if !path_exists(data_root)? {
        return if expected.is_empty() {
            Ok(None)
        } else {
            Err(inconsistent("referenced installed slots are missing"))
        };
    }
    let metadata = fs::symlink_metadata(data_root)
        .map_err(|error| Error::io("cannot inspect plugin data root", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(inconsistent("plugin data root is not a regular directory"));
    }
    let mut installed = None;
    for entry in fs::read_dir(data_root)
        .map_err(|error| Error::io("cannot read plugin data root", &error))?
    {
        let entry =
            entry.map_err(|error| Error::io("cannot inspect plugin data member", &error))?;
        if entry.file_name() != "installed" {
            return Err(inconsistent(
                "plugin data root contains an unreferenced top-level member",
            ));
        }
        installed = Some(entry.path());
    }
    match installed {
        Some(path) => Ok(Some(path)),
        None if expected.is_empty() => Ok(None),
        None => Err(inconsistent("referenced installed slots are missing")),
    }
}

pub(super) fn canonical_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn validate_slot(data_root: &Path, plugin_id: &str, digest: &str) -> Result<Candidate> {
    let actual = data_root.join("installed").join(plugin_id).join(digest);
    let actual_digest = validate_and_hash(&actual).map_err(|error| {
        Error::with_details(
            "legacy_migration_inconsistent",
            format!("plugin {plugin_id} installed tree is missing or unsafe"),
            serde_json::json!({"cause": error.body(), "path": actual}),
        )
    })?;
    if actual_digest != digest {
        return Err(inconsistent(
            "installed tree digest does not match its immutable slot",
        ));
    }
    let candidate = parse_plugin(&actual).map_err(|error| {
        Error::with_details(
            "legacy_migration_inconsistent",
            format!("plugin {plugin_id} manifest is invalid"),
            serde_json::json!({"cause": error.body(), "path": actual}),
        )
    })?;
    if candidate.provider.plugin_id != plugin_id {
        return Err(inconsistent(
            "installed manifest id does not match its state key",
        ));
    }
    Ok(candidate)
}

fn child_directories(parent: &Path) -> Result<Vec<(String, PathBuf)>> {
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|error| Error::io("cannot inspect installed directory", &error))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(inconsistent(
            "installed path component is not a regular directory",
        ));
    }
    let entries =
        fs::read_dir(parent).map_err(|error| Error::io("cannot inspect installed data", &error))?;
    entries
        .map(|entry| {
            let entry =
                entry.map_err(|error| Error::io("cannot inspect installed member", &error))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| Error::io("cannot inspect installed member", &error))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(inconsistent(
                    "installed contains a file or unsafe directory member",
                ));
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| inconsistent("installed contains a non-UTF-8 directory component"))?;
            Ok((name, path))
        })
        .collect()
}

fn inconsistent(message: &str) -> Error {
    Error::new("legacy_migration_inconsistent", message)
}
