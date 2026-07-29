use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

use super::MigrationOps;

pub const MAX_MIGRATION_MEMBERS: usize = 4_096;
pub const MAX_MIGRATION_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Budget {
    members: usize,
    bytes: u64,
}

impl Budget {
    fn add(&mut self, metadata: &fs::Metadata) -> Result<()> {
        self.members = self.members.saturating_add(1);
        if metadata.is_file() {
            self.bytes = self.bytes.saturating_add(metadata.len());
        }
        if self.members > MAX_MIGRATION_MEMBERS || self.bytes > MAX_MIGRATION_BYTES {
            return Err(Error::new(
                "legacy_migration_too_large",
                "legacy plugin state exceeds migration member or byte limits",
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_tree(root: &Path, budget: &mut Budget) -> Result<()> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| Error::io("cannot inspect legacy root", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "legacy_migration_unsafe",
            format!("legacy root is not a regular directory: {}", root.display()),
        ));
    }
    for entry in walkdir::WalkDir::new(root).follow_links(false).min_depth(1) {
        let entry =
            entry.map_err(|error| Error::new("legacy_migration_unsafe", error.to_string()))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| Error::io("cannot inspect legacy member", &error))?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(Error::new(
                "legacy_migration_unsafe",
                format!(
                    "legacy tree contains an unsafe member: {}",
                    entry.path().display()
                ),
            ));
        }
        if metadata.is_file() && metadata.nlink() != 1 {
            return Err(Error::new(
                "legacy_migration_unsafe",
                format!(
                    "legacy tree contains a hardlink: {}",
                    entry.path().display()
                ),
            ));
        }
        budget.add(&metadata)?;
    }
    Ok(())
}

pub(super) fn copy_root(
    source: &Path,
    destination: &Path,
    kind: usize,
    operations: &dyn MigrationOps,
    boundary: &mut usize,
) -> Result<()> {
    create_private_root(destination)?;
    preparation_checkpoint(operations, boundary)?;
    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .min_depth(1)
    {
        let entry =
            entry.map_err(|error| Error::new("legacy_migration_unsafe", error.to_string()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|_| Error::new("legacy_migration_unsafe", "legacy member escaped root"))?;
        let target = destination.join(relative);
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| Error::io("cannot inspect legacy copy source", &error))?;
        if metadata.is_dir() {
            fs::create_dir(&target)
                .map_err(|error| Error::io("cannot copy legacy directory", &error))?;
            set_mode(&target, 0o700)?;
        } else {
            copy_regular_file(entry.path(), &target, &metadata)?;
            set_mode(&target, copied_mode(kind, relative, false))?;
        }
        preparation_checkpoint(operations, boundary)?;
    }
    finalize_modes(destination, kind)
}

fn preparation_checkpoint(operations: &dyn MigrationOps, boundary: &mut usize) -> Result<()> {
    let current = *boundary;
    *boundary = boundary.saturating_add(1);
    operations.preparation_checkpoint(current)
}

fn copy_regular_file(source: &Path, destination: &Path, before: &fs::Metadata) -> Result<()> {
    if before.nlink() != 1 {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "legacy file became hardlinked during copy",
        ));
    }
    let mut input =
        fs::File::open(source).map_err(|error| Error::io("cannot open legacy file", &error))?;
    let opened = input
        .metadata()
        .map_err(|error| Error::io("cannot inspect opened legacy file", &error))?;
    if !same_file(before, &opened) || opened.nlink() != 1 {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "legacy file changed while opening",
        ));
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .map_err(|error| Error::io("cannot create migrated file", &error))?;
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| Error::io("cannot read legacy file", &error))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| Error::io("cannot write migrated file", &error))?;
    }
    output
        .sync_all()
        .map_err(|error| Error::io("cannot sync migrated file", &error))?;
    let after = fs::symlink_metadata(source)
        .map_err(|error| Error::io("cannot re-inspect legacy file", &error))?;
    if !same_file(before, &after) || after.nlink() != 1 {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "legacy file changed during copy",
        ));
    }
    Ok(())
}

fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

fn copied_mode(kind: usize, relative: &Path, directory: bool) -> u32 {
    if kind == 2 && immutable_slot(relative) {
        if directory { 0o555 } else { 0o444 }
    } else if directory {
        0o700
    } else {
        0o600
    }
}

fn finalize_modes(root: &Path, kind: usize) -> Result<()> {
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .contents_first(true)
        .min_depth(1)
    {
        let entry =
            entry.map_err(|error| Error::new("legacy_migration_unsafe", error.to_string()))?;
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| Error::new("legacy_migration_unsafe", "staged member escaped root"))?;
        set_mode(
            entry.path(),
            copied_mode(kind, relative, entry.file_type().is_dir()),
        )?;
    }
    Ok(())
}

fn immutable_slot(path: &Path) -> bool {
    let components: Vec<_> = path.components().collect();
    components.len() >= 4
        && matches!(components.first(), Some(Component::Normal(name)) if *name == "installed")
}

pub(super) fn create_private_root(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("legacy_migration_unsafe", "staging has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| Error::io("cannot create migration parent", &error))?;
    set_mode(parent, 0o700)?;
    fs::create_dir(path)
        .map_err(|error| Error::io("cannot create migration staging root", &error))?;
    set_mode(path, 0o700)
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| Error::io("cannot set migrated permissions", &error))
}

pub(super) fn remove_private_tree(path: &Path) -> Result<()> {
    let root = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io("cannot inspect migration staging", &error)),
    };
    if root.file_type().is_symlink() || !root.is_dir() {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "refusing to remove unsafe migration staging root",
        ));
    }
    let mut entries: Vec<PathBuf> = walkdir::WalkDir::new(path)
        .follow_links(false)
        .contents_first(true)
        .into_iter()
        .map(|entry| {
            entry
                .map(walkdir::DirEntry::into_path)
                .map_err(|error| Error::new("legacy_migration_unsafe", error.to_string()))
        })
        .collect::<Result<_>>()?;
    if entries.len() > MAX_MIGRATION_MEMBERS + 1 {
        return Err(Error::new(
            "legacy_migration_unsafe",
            "refusing to remove oversized migration staging",
        ));
    }
    for entry in &entries {
        let metadata = fs::symlink_metadata(entry)
            .map_err(|error| Error::io("cannot inspect migration cleanup member", &error))?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(Error::new(
                "legacy_migration_unsafe",
                "refusing to remove unsafe migration staging",
            ));
        }
        if metadata.is_dir() {
            set_mode(entry, 0o700)?;
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.components().count()));
    for entry in entries {
        if entry.is_dir() {
            fs::remove_dir(&entry)
                .map_err(|error| Error::io("cannot remove migration directory", &error))?;
        } else {
            fs::remove_file(&entry)
                .map_err(|error| Error::io("cannot remove migration file", &error))?;
        }
    }
    Ok(())
}
