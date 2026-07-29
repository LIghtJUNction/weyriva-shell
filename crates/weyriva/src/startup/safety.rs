use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
use std::path::{Component, Path};

use nix::fcntl::{AT_FDCWD, AtFlags};
use nix::unistd::fchownat;

use crate::error::{Error, Result};
use crate::identity::{gid, uid};

pub trait Ownership: Send + Sync {
    /// Changes ownership without following symlinks.
    ///
    /// # Errors
    ///
    /// Returns an error when ownership cannot be applied.
    fn set_owner(&self, path: &Path, uid: u32, gid: u32) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemOwnership;

impl Ownership for SystemOwnership {
    fn set_owner(&self, path: &Path, owner: u32, group: u32) -> Result<()> {
        fchownat(
            AT_FDCWD,
            path,
            Some(uid(owner)),
            Some(gid(group)),
            AtFlags::AT_SYMLINK_NOFOLLOW,
        )
        .map_err(|error| {
            Error::new(
                "ownership_failed",
                format!("cannot own {}: {error}", path.display()),
            )
        })
    }
}

pub trait FileSystem: Send + Sync {
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn read_link(&self, path: &Path) -> io::Result<std::path::PathBuf>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn create_dir(&self, path: &Path) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn remove_dir(&self, path: &Path) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn set_mode(&self, path: &Path, mode: u32) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn write_atomic(&self, path: &Path, data: &[u8], mode: u32) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()>;
    /// # Errors
    ///
    /// Returns an operating-system filesystem error.
    fn sync_directory(&self, path: &Path) -> io::Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemFileSystem;

impl FileSystem for SystemFileSystem {
    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata> {
        fs::symlink_metadata(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        fs::metadata(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<std::path::PathBuf> {
        fs::read_link(path)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn set_mode(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
    }

    fn write_atomic(&self, path: &Path, data: &[u8], mode: u32) -> io::Result<()> {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("state");
        let temporary = path.with_file_name(format!(".{name}.weyriva-{}.tmp", std::process::id()));
        let result = (|| {
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(mode)
                .open(&temporary)?;
            output.write_all(data)?;
            output.sync_all()?;
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        symlink(target, link)
    }

    fn sync_directory(&self, path: &Path) -> io::Result<()> {
        fs::File::open(path)?.sync_all()
    }
}

pub trait MutationObserver: Send + Sync {
    /// Records a completed apply mutation.
    ///
    /// # Errors
    ///
    /// Returns an injected error when tests need to exercise rollback.
    fn applied(&self, _operation: &str) -> Result<()> {
        Ok(())
    }

    /// Runs immediately before one rollback mutation.
    ///
    /// # Errors
    ///
    /// Returns an injected error when tests need to exercise rollback failure.
    fn before_rollback(&self, _operation: &str) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopMutationObserver;

impl MutationObserver for NoopMutationObserver {}

pub fn validate_safe_chain(path: &Path, require_leaf: bool) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::new(
            "unsafe_startup_path",
            format!("path must be absolute: {}", path.display()),
        ));
    }
    let mut current = Path::new("/").to_path_buf();
    for component in path.components() {
        if matches!(component, Component::RootDir) {
            continue;
        }
        let Component::Normal(name) = component else {
            return Err(Error::new("unsafe_startup_path", "unsafe path component"));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::new(
                    "unsafe_startup_path",
                    format!("refusing path containing a symlink: {}", current.display()),
                ));
            }
            Ok(metadata) if !metadata.is_dir() && current != path => {
                return Err(Error::new(
                    "unsafe_startup_path",
                    format!("path component is not a directory: {}", current.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("cannot inspect startup path", &error)),
        }
    }
    if require_leaf && !path.is_dir() {
        return Err(Error::new(
            "unsafe_startup_path",
            format!("required directory is unavailable: {}", path.display()),
        ));
    }
    Ok(())
}

pub fn validate_destination(path: &Path) -> Result<()> {
    validate_safe_chain(
        path.parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "destination has no parent"))?,
        false,
    )?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(Error::new(
                "unsafe_startup_path",
                format!("unsafe file destination: {}", path.display()),
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io("cannot inspect file destination", &error)),
    }
}

pub fn validate_regular_file(path: &Path, description: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(()),
        _ => Err(Error::new(
            "startup_incomplete",
            format!("{description} is unavailable or unsafe: {}", path.display()),
        )),
    }
}
