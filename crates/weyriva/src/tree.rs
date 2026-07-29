use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::model::{MAX_FILES, MAX_TREE_BYTES};

/// Validates and hashes one plugin tree without following links.
///
/// # Errors
///
/// Returns an error for symlinks, special files, traversal failures, or size
/// limits.
pub fn validate_and_hash(root: &Path) -> Result<String> {
    let root_metadata = fs::symlink_metadata(root)
        .map_err(|error| Error::io("cannot inspect plugin root", &error))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(Error::new(
            "unsafe_plugin",
            "plugin root is not a regular directory",
        ));
    }
    let mut paths = collect_paths(root)?;
    paths.sort_by_key(|left| relative_bytes(root, left));
    let mut count = 0_usize;
    let mut total = 0_u64;
    let mut hash = Sha256::new();
    for path in paths {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| Error::io("cannot inspect plugin tree entry", &error))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::new(
                "unsafe_plugin",
                format!("symlinks are forbidden: {}", path.display()),
            ));
        }
        if metadata.is_dir() {
            continue;
        }
        if !metadata.is_file() {
            return Err(Error::new(
                "unsafe_plugin",
                format!("special files are forbidden: {}", path.display()),
            ));
        }
        let identity = file_identity(&path, &metadata)?;
        count = count.saturating_add(1);
        total = total.saturating_add(metadata.len());
        if count > MAX_FILES || total > MAX_TREE_BYTES {
            return Err(Error::new(
                "plugin_too_large",
                "plugin exceeds file count or byte limit",
            ));
        }
        hash.update(relative_bytes(root, &path));
        hash.update([0]);
        let mut input =
            fs::File::open(&path).map_err(|error| Error::io("cannot open plugin file", &error))?;
        verify_open_file(&path, &input, identity)?;
        let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
        loop {
            let read = input
                .read(&mut buffer)
                .map_err(|error| Error::io("cannot hash plugin file", &error))?;
            if read == 0 {
                break;
            }
            hash.update(&buffer[..read]);
        }
        verify_path_file(&path, identity)?;
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Copies a previously validated tree without following links.
///
/// # Errors
///
/// Returns an error if any entry changes type or a copy operation fails.
pub fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir(destination)
        .map_err(|error| Error::io("cannot create staged plugin directory", &error))?;
    for path in collect_paths(source)? {
        let relative = path
            .strip_prefix(source)
            .map_err(|_| Error::new("unsafe_plugin", "plugin path escaped source root"))?;
        let target = destination.join(relative);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| Error::io("cannot inspect plugin copy source", &error))?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(Error::new(
                "unsafe_plugin",
                "plugin tree changed during copy",
            ));
        }
        if metadata.is_dir() {
            fs::create_dir(&target)
                .map_err(|error| Error::io("cannot create staged plugin subdirectory", &error))?;
            continue;
        }
        let identity = file_identity(&path, &metadata)?;
        let mut input = fs::File::open(&path)
            .map_err(|error| Error::io("cannot open plugin source", &error))?;
        verify_open_file(&path, &input, identity)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)
            .map_err(|error| Error::io("cannot create staged plugin file", &error))?;
        std::io::copy(&mut input, &mut output)
            .map_err(|error| Error::io("cannot copy plugin file", &error))?;
        output
            .flush()
            .map_err(|error| Error::io("cannot flush staged plugin file", &error))?;
        verify_path_file(&path, identity)?;
        let copied = output
            .metadata()
            .map_err(|error| Error::io("cannot inspect staged plugin file", &error))?;
        if copied.nlink() != 1 || copied.len() != identity.len {
            return Err(Error::new(
                "unsafe_plugin",
                "staged plugin file identity is invalid",
            ));
        }
    }
    Ok(())
}

fn collect_paths(root: &Path) -> Result<Vec<PathBuf>> {
    WalkDir::new(root)
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .map(|entry| {
            entry.map(walkdir::DirEntry::into_path).map_err(|error| {
                Error::new("unsafe_plugin", format!("cannot walk plugin tree: {error}"))
            })
        })
        .collect()
}

fn relative_bytes(root: &Path, path: &Path) -> Vec<u8> {
    path.strip_prefix(root)
        .unwrap_or(path)
        .as_os_str()
        .as_bytes()
        .to_vec()
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    links: u64,
    len: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

fn file_identity(path: &Path, metadata: &fs::Metadata) -> Result<FileIdentity> {
    let identity = FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        len: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    };
    if identity.links != 1 {
        return Err(Error::new(
            "unsafe_plugin",
            format!("hardlinked files are forbidden: {}", path.display()),
        ));
    }
    Ok(identity)
}

fn verify_open_file(path: &Path, file: &fs::File, expected: FileIdentity) -> Result<()> {
    let metadata = file
        .metadata()
        .map_err(|error| Error::io("cannot inspect opened plugin file", &error))?;
    if file_identity(path, &metadata)? != expected {
        return Err(Error::new(
            "unsafe_plugin",
            "plugin file changed while opening",
        ));
    }
    Ok(())
}

fn verify_path_file(path: &Path, expected: FileIdentity) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot re-inspect plugin file", &error))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || file_identity(path, &metadata)? != expected
    {
        return Err(Error::new(
            "unsafe_plugin",
            "plugin file changed during processing",
        ));
    }
    Ok(())
}
