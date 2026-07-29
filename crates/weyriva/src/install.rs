#![expect(
    clippy::missing_errors_doc,
    reason = "installation helpers share the crate's typed control-plane error contract"
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::manifest::{parse_plugin, valid_plugin_id};
use crate::model::PluginRecord;
use crate::paths::Paths;
use crate::sources::{load_state, resolve};
use crate::storage::atomic_json;
use crate::tree::{copy_tree, validate_and_hash};

pub fn install(paths: &Paths, plugin_id: &str) -> Result<PluginRecord> {
    if !valid_plugin_id(plugin_id) {
        return Err(Error::new(
            "invalid_plugin_id",
            "plugin id must be canonical author/plugin",
        ));
    }
    let mut state = load_state(paths)?;
    if state
        .plugins
        .get(plugin_id)
        .is_some_and(|record| record.enabled)
    {
        return Err(Error::new(
            "plugin_enabled",
            format!("disable {plugin_id} before replacing its immutable installed version"),
        ));
    }
    fs::create_dir_all(&paths.data_dir)
        .map_err(|error| Error::io("cannot create plugin data directory", &error))?;
    let resolve_root = paths
        .data_dir
        .join(format!(".resolve-{}", std::process::id()));
    remove_private_tree_if_exists(&resolve_root)?;
    fs::create_dir(&resolve_root)
        .map_err(|error| Error::io("cannot create plugin resolution directory", &error))?;
    let outcome = (|| {
        let (candidate, provenance) = resolve(paths, plugin_id, &resolve_root)?;
        let digest = validate_and_hash(&candidate.root)?;
        let plugin_parent = paths.data_dir.join("installed").join(plugin_id);
        fs::create_dir_all(&plugin_parent)
            .map_err(|error| Error::io("cannot create immutable plugin parent", &error))?;
        let version_dir = plugin_parent.join(&digest);
        if version_dir.exists() {
            let existing = parse_plugin(&version_dir)?;
            if existing.provider != candidate.provider
                || existing.settings_defaults != candidate.settings_defaults
                || validate_and_hash(&version_dir)? != digest
            {
                return Err(Error::new(
                    "install_validation_failed",
                    "existing immutable plugin version failed validation",
                ));
            }
        } else {
            materialize(&candidate, &plugin_parent, &version_dir, &digest)?;
        }
        let previous = state.plugins.get(plugin_id);
        let record = PluginRecord {
            id: plugin_id.to_owned(),
            installed: true,
            enabled: previous.is_some_and(|record| record.enabled),
            path: version_dir,
            digest,
            version: candidate.provider.version.clone(),
            provider: candidate.provider,
            settings_defaults: candidate.settings_defaults,
            provenance,
            last_known_good: previous.and_then(|record| record.last_known_good.clone()),
        };
        state.plugins.insert(plugin_id.to_owned(), record.clone());
        atomic_json(&paths.state_file(), &state)?;
        Ok(record)
    })();
    let cleanup = remove_private_tree_if_exists(&resolve_root);
    match (outcome, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
    }
}

fn materialize(
    candidate: &crate::model::Candidate,
    parent: &Path,
    destination: &Path,
    digest: &str,
) -> Result<()> {
    let stage = parent.join(format!(".stage-{}", std::process::id()));
    remove_private_tree_if_exists(&stage)?;
    copy_tree(&candidate.root, &stage)?;
    let staged = parse_plugin(&stage)?;
    if staged.provider != candidate.provider || validate_and_hash(&stage)? != digest {
        remove_private_tree_if_exists(&stage)?;
        return Err(Error::new(
            "install_validation_failed",
            "staged plugin differs from source",
        ));
    }
    fs::rename(&stage, destination)
        .map_err(|error| Error::io("cannot publish immutable plugin version", &error))?;
    make_immutable(destination)
}

fn make_immutable(root: &Path) -> Result<()> {
    let paths: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .map(|entry| {
            entry
                .map(walkdir::DirEntry::into_path)
                .map_err(|error| Error::new("install_validation_failed", error.to_string()))
        })
        .collect::<Result<_>>()?;
    for path in paths {
        let mode = if path.is_dir() { 0o555 } else { 0o444 };
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))
            .map_err(|error| Error::io("cannot make plugin immutable", &error))?;
    }
    fs::set_permissions(root, fs::Permissions::from_mode(0o555))
        .map_err(|error| Error::io("cannot make plugin root immutable", &error))
}

pub fn remove_private_tree_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot inspect private tree", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new("unsafe_state", "private tree path is unsafe"));
    }
    let entries: Vec<PathBuf> = walkdir::WalkDir::new(path)
        .contents_first(true)
        .into_iter()
        .map(|entry| {
            entry
                .map(walkdir::DirEntry::into_path)
                .map_err(|error| Error::new("io_error", error.to_string()))
        })
        .collect::<Result<_>>()?;
    for entry in &entries {
        let metadata = fs::symlink_metadata(entry)
            .map_err(|error| Error::io("cannot inspect private tree entry", &error))?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(Error::new(
                "unsafe_state",
                "private tree contains unsafe entry",
            ));
        }
        if metadata.is_dir() {
            fs::set_permissions(entry, fs::Permissions::from_mode(0o700))
                .map_err(|error| Error::io("cannot unlock private directory", &error))?;
        }
    }
    for entry in entries {
        if entry.is_dir() {
            fs::remove_dir(&entry)
                .map_err(|error| Error::io("cannot remove private directory", &error))?;
        } else {
            fs::remove_file(&entry)
                .map_err(|error| Error::io("cannot remove private file", &error))?;
        }
    }
    Ok(())
}
