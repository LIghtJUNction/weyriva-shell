use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::state_commit::{StateWriteError, StateWriteResult};

const MAX_STATE_BYTES: u64 = 4 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Reads a bounded schema document or returns its default.
///
/// # Errors
///
/// Returns an error for symlinks, non-files, oversized data, I/O failures, or
/// invalid JSON.
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => return Err(Error::io("cannot inspect JSON state", &error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::new(
            "unsafe_state",
            format!("state is not a regular file: {}", path.display()),
        ));
    }
    if metadata.len() > MAX_STATE_BYTES {
        return Err(Error::new("invalid_state", "state exceeds the 4 MiB limit"));
    }
    let mut input =
        File::open(path).map_err(|error| Error::io("cannot open JSON state", &error))?;
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    input
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io("cannot read JSON state", &error))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| Error::new("invalid_state", format!("invalid JSON state: {error}")))
}

/// Atomically writes one private JSON document and durably syncs its directory.
///
/// # Errors
///
/// Returns an error when the directory is unsafe or any write, rename,
/// permission, or durability operation fails.
pub fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    atomic_json_commit(path, value).map_err(StateWriteError::into_error)
}

/// Atomically writes one private JSON document with an explicit commit result.
///
/// The rename is the commit point. Every failure before it leaves the old
/// document authoritative; a directory-sync failure after it leaves the new
/// document authoritative but not durably confirmed.
///
/// # Errors
///
/// Returns `PreCommit` before the rename or `PostCommit` when directory
/// durability confirmation fails after the rename.
pub fn atomic_json_commit<T: Serialize>(path: &Path, value: &T) -> StateWriteResult<()> {
    atomic_json_precommit(path, value).and_then(|directory| {
        directory.sync_all().map_err(|error| {
            StateWriteError::PostCommit(Error::io("cannot sync state directory", &error))
        })
    })
}

fn atomic_json_precommit<T: Serialize>(path: &Path, value: &T) -> StateWriteResult<File> {
    let parent = path.parent().ok_or_else(|| {
        StateWriteError::PreCommit(Error::new("unsafe_state", "state path has no parent"))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        StateWriteError::PreCommit(Error::io("cannot create state directory", &error))
    })?;
    let parent_metadata = fs::symlink_metadata(parent).map_err(|error| {
        StateWriteError::PreCommit(Error::io("cannot inspect state directory", &error))
    })?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(StateWriteError::PreCommit(Error::new(
            "unsafe_state",
            format!(
                "state parent is not a regular directory: {}",
                parent.display()
            ),
        )));
    }
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
        StateWriteError::PreCommit(Error::io("cannot secure state directory", &error))
    })?;
    let directory = File::open(parent).map_err(|error| {
        StateWriteError::PreCommit(Error::io("cannot open state directory", &error))
    })?;

    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            StateWriteError::PreCommit(Error::new("unsafe_state", "state filename is invalid"))
        })?;
    let temporary = parent.join(format!(".{name}.{}.{sequence}.tmp", std::process::id()));
    let result = (|| -> Result<()> {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| Error::io("cannot create temporary state", &error))?;
        output
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| Error::io("cannot secure temporary state", &error))?;
        serde_json::to_writer(&mut output, value).map_err(|error| {
            Error::new("invalid_state", format!("cannot encode state: {error}"))
        })?;
        output
            .write_all(b"\n")
            .map_err(|error| Error::io("cannot finish state", &error))?;
        output
            .sync_all()
            .map_err(|error| Error::io("cannot sync state", &error))?;
        fs::rename(&temporary, path)
            .map_err(|error| Error::io("cannot atomically replace state", &error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
        .map(|()| directory)
        .map_err(StateWriteError::PreCommit)
}
