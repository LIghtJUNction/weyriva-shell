use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::{Archive, EntryType};

use crate::error::{Error, Result};
use crate::model::{MAX_ARCHIVE_BYTES, MAX_FILES};

const MAX_ARCHIVE_MEMBERS: usize = MAX_FILES * 4;

/// Safely extracts one bounded gzip tar archive and returns its sole root.
///
/// # Errors
///
/// Returns an error for traversal, links, special files, multiple roots, size
/// limits, malformed archives, or I/O failures.
pub fn extract(archive_path: &Path, destination: &Path) -> Result<PathBuf> {
    let roots = inspect(archive_path)?;
    let root = roots
        .into_iter()
        .next()
        .ok_or_else(|| Error::new("unsafe_archive", "archive has no top-level directory"))?;
    fs::create_dir(destination)
        .map_err(|error| Error::io("cannot create extraction directory", &error))?;
    let input =
        fs::File::open(archive_path).map_err(|error| Error::io("cannot reopen archive", &error))?;
    let mut archive = Archive::new(GzDecoder::new(input));
    let entries = archive
        .entries()
        .map_err(|error| Error::new("unsafe_archive", format!("cannot read archive: {error}")))?;
    for entry in entries {
        let mut entry = entry
            .map_err(|error| Error::new("unsafe_archive", format!("invalid archive: {error}")))?;
        let relative = entry
            .path()
            .map_err(|error| {
                Error::new("unsafe_archive", format!("invalid archive path: {error}"))
            })?
            .into_owned();
        validate_path(&relative)?;
        let kind = entry.header().entry_type();
        if kind == EntryType::XGlobalHeader {
            continue;
        }
        let target = destination.join(&relative);
        match kind {
            EntryType::Directory => fs::create_dir_all(&target)
                .map_err(|error| Error::io("cannot create archive directory", &error))?,
            EntryType::Regular => {
                let parent = target
                    .parent()
                    .ok_or_else(|| Error::new("unsafe_archive", "archive file has no parent"))?;
                fs::create_dir_all(parent)
                    .map_err(|error| Error::io("cannot create archive parent", &error))?;
                let mut output = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&target)
                    .map_err(|error| Error::io("cannot create extracted file", &error))?;
                std::io::copy(&mut entry, &mut output)
                    .map_err(|error| Error::io("cannot extract archive file", &error))?;
                output
                    .flush()
                    .map_err(|error| Error::io("cannot flush extracted file", &error))?;
            }
            _ => {
                return Err(Error::new(
                    "unsafe_archive",
                    "archive contains links or special files",
                ));
            }
        }
    }
    Ok(destination.join(root))
}

fn inspect(archive_path: &Path) -> Result<BTreeSet<OsString>> {
    let input =
        fs::File::open(archive_path).map_err(|error| Error::io("cannot open archive", &error))?;
    let mut archive = Archive::new(GzDecoder::new(input));
    let mut roots = BTreeSet::new();
    let mut members = 0_usize;
    let mut total = 0_u64;
    let entries = archive
        .entries()
        .map_err(|error| Error::new("unsafe_archive", format!("cannot read archive: {error}")))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| Error::new("unsafe_archive", format!("invalid archive: {error}")))?;
        members = members.saturating_add(1);
        if members > MAX_ARCHIVE_MEMBERS {
            return Err(Error::new(
                "archive_too_large",
                "archive has too many members",
            ));
        }
        let path = entry.path().map_err(|error| {
            Error::new("unsafe_archive", format!("invalid archive path: {error}"))
        })?;
        validate_path(&path)?;
        let kind = entry.header().entry_type();
        if kind != EntryType::Directory
            && kind != EntryType::Regular
            && kind != EntryType::XGlobalHeader
        {
            return Err(Error::new(
                "unsafe_archive",
                "archive contains links or special files",
            ));
        }
        if kind != EntryType::XGlobalHeader
            && let Some(Component::Normal(root)) = path.components().next()
        {
            roots.insert(root.to_os_string());
        }
        if kind == EntryType::Regular || kind == EntryType::XGlobalHeader {
            total = total.saturating_add(entry.size());
            if total > MAX_ARCHIVE_BYTES {
                return Err(Error::new(
                    "archive_too_large",
                    "archive exceeds extraction limits",
                ));
            }
        }
    }
    if roots.len() != 1 {
        return Err(Error::new(
            "unsafe_archive",
            "archive must contain one top-level directory",
        ));
    }
    Ok(roots)
}

fn validate_path(path: &Path) -> Result<()> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(Error::new(
            "unsafe_archive",
            "archive path escapes extraction root",
        ));
    }
    Ok(())
}
