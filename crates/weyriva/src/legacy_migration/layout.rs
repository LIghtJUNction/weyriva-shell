use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};
use crate::paths::Paths;

pub(super) struct MigrationRoots {
    pub old: [PathBuf; 3],
    pub new: [PathBuf; 3],
    pub staged: [PathBuf; 3],
    pub recovery: [PathBuf; 3],
    pub journal: PathBuf,
}

pub(super) struct RootState {
    pub old: [bool; 3],
    pub new: [bool; 3],
    staged: [bool; 3],
    recovery: [bool; 3],
}

impl RootState {
    pub fn is_completed_migration(&self) -> bool {
        self.new.iter().all(|present| *present)
            && !self.old.iter().any(|present| *present)
            && !self.staged.iter().any(|present| *present)
            && self.recovery.iter().any(|present| *present)
    }

    pub fn has_transient(&self) -> bool {
        self.staged.iter().any(|present| *present) || self.recovery.iter().any(|present| *present)
    }
}

impl MigrationRoots {
    pub fn from_paths(paths: &Paths) -> Result<Self> {
        let new = [
            paths.config_dir.clone(),
            paths.state_dir.clone(),
            paths.data_dir.clone(),
        ];
        let old = new.clone().map(|path| sibling(&path, "plugins-v5"));
        let staged = new.clone().map(|path| sibling(&path, ".plugins.migrating"));
        let recovery = new
            .clone()
            .map(|path| sibling(&path, "plugins-v5.migrated"));
        let state_parent = paths.state_dir.parent().ok_or_else(|| {
            Error::new(
                "legacy_migration_unsafe",
                "state root has no parent directory",
            )
        })?;
        let roots = Self {
            old,
            new,
            staged,
            recovery,
            journal: state_parent.join(".plugins-migration.json"),
        };
        roots.validate_absolute()?;
        Ok(roots)
    }

    pub fn inspect(&self) -> Result<RootState> {
        Ok(RootState {
            old: presence(&self.old)?,
            new: presence(&self.new)?,
            staged: presence(&self.staged)?,
            recovery: presence(&self.recovery)?,
        })
    }

    pub fn journal_exists(&self) -> Result<bool> {
        path_exists(&self.journal)
    }

    fn validate_absolute(&self) -> Result<()> {
        for path in self
            .old
            .iter()
            .chain(self.new.iter())
            .chain(self.staged.iter())
            .chain(self.recovery.iter())
            .chain(std::iter::once(&self.journal))
        {
            if !path.is_absolute() {
                return Err(Error::new(
                    "legacy_migration_unsafe",
                    format!("migration path must be absolute: {}", path.display()),
                ));
            }
            validate_parent_chain(path)?;
        }
        Ok(())
    }
}

fn sibling(path: &Path, name: &str) -> PathBuf {
    path.parent()
        .map_or_else(|| PathBuf::from(name), |parent| parent.join(name))
}

fn presence(paths: &[PathBuf; 3]) -> Result<[bool; 3]> {
    Ok([
        path_exists(&paths[0])?,
        path_exists(&paths[1])?,
        path_exists(&paths[2])?,
    ])
}

pub(super) fn path_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("cannot inspect migration path", &error)),
    }
}

fn validate_parent_chain(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("legacy_migration_unsafe", "migration path has no parent"))?;
    let mut current = PathBuf::from("/");
    for component in parent.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => current.push(name),
            _ => {
                return Err(Error::new(
                    "legacy_migration_unsafe",
                    "migration path contains an unsafe component",
                ));
            }
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(Error::new(
                    "legacy_migration_unsafe",
                    format!("unsafe migration parent: {}", current.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(Error::io("cannot inspect migration parent", &error)),
        }
    }
    Ok(())
}
