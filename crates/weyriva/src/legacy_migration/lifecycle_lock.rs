use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use fs2::FileExt;

use crate::error::{Error, Result};
use crate::paths::Paths;

pub struct MigrationLock {
    _migration: File,
    _daemon_probe: Option<File>,
}

impl MigrationLock {
    /// Acquires the migration lock and proves no daemon owns plugin state.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe lock paths or an active migration/daemon.
    pub fn acquire(paths: &Paths) -> Result<Self> {
        secure_runtime_dir(&paths.runtime_dir)?;
        let migration = open_lock(&paths.runtime_dir.join("migration.lock"))?;
        try_lock(
            &migration,
            "migration_busy",
            "another Weyriva migration is active",
        )?;
        let daemon_probe = open_lock(&paths.daemon_lock_file())?;
        try_lock(
            &daemon_probe,
            "migration_daemon_active",
            "a Weyriva daemon is using plugin state; stop it before migration",
        )?;
        Ok(Self {
            _migration: migration,
            _daemon_probe: Some(daemon_probe),
        })
    }

    /// Acquires only the migration lock after the caller holds `daemon.lock`.
    ///
    /// This preserves daemon-to-migration lock order and avoids probing a lock
    /// already owned by the current process.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe lock path or an active migration.
    pub fn acquire_after_daemon(paths: &Paths) -> Result<Self> {
        secure_runtime_dir(&paths.runtime_dir)?;
        let migration = open_lock(&paths.runtime_dir.join("migration.lock"))?;
        try_lock(
            &migration,
            "migration_busy",
            "another Weyriva migration is active",
        )?;
        Ok(Self {
            _migration: migration,
            _daemon_probe: None,
        })
    }
}

fn secure_runtime_dir(path: &Path) -> Result<()> {
    validate_parent(path)?;
    fs::create_dir_all(path)
        .map_err(|error| Error::io("cannot create migration runtime directory", &error))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot inspect migration runtime directory", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "migration runtime path must be a regular directory",
        ));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| Error::io("cannot secure migration runtime directory", &error))
}

fn validate_parent(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "migration runtime path must be absolute",
        ));
    }
    let mut current = PathBuf::from("/");
    for component in path
        .parent()
        .ok_or_else(|| Error::new("legacy_migration_unsafe", "runtime path has no parent"))?
        .components()
    {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => current.push(name),
            _ => {
                return Err(Error::new(
                    "legacy_migration_unsafe",
                    "migration runtime path contains an unsafe component",
                ));
            }
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(Error::new(
                    "legacy_migration_unsafe",
                    format!("unsafe migration runtime parent: {}", current.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(Error::io("cannot inspect runtime parent", &error)),
        }
    }
    Ok(())
}

fn open_lock(path: &Path) -> Result<File> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(Error::new(
                "legacy_migration_unsafe",
                format!("migration lock is unsafe: {}", path.display()),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io("cannot inspect migration lock", &error)),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|error| Error::io("cannot open migration lock", &error))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| Error::io("cannot secure migration lock", &error))?;
    Ok(file)
}

fn try_lock(file: &File, code: &str, message: &str) -> Result<()> {
    file.try_lock_exclusive().map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Error::new(code, message)
        } else {
            Error::io("cannot acquire migration lock", &error)
        }
    })
}
