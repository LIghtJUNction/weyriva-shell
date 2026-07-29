use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use serde_json::json;

use crate::error::{Error, Result};

use super::model::{DisplayManagerState, StartupContext};
pub(super) use super::rollback::EnableState;
use super::rollback::{self, Undo, metadata_undo};

pub(super) struct Transaction<'a> {
    context: &'a StartupContext,
    undo: Vec<Undo>,
}

impl<'a> Transaction<'a> {
    pub(super) fn new(context: &'a StartupContext) -> Self {
        Self {
            context,
            undo: Vec::new(),
        }
    }

    pub(super) fn rollback(mut self, apply_error: Error) -> Error {
        let mut failures = Vec::new();
        while let Some(undo) = self.undo.pop() {
            let label = undo.label();
            let result = self
                .context
                .mutations
                .before_rollback(&label)
                .and_then(|()| rollback::execute(self.context, undo));
            if let Err(error) = result {
                failures.push(json!({
                    "operation": label,
                    "error": error.body(),
                }));
            }
        }
        if failures.is_empty() {
            apply_error
        } else {
            Error::with_details(
                "rollback_failed",
                "startup apply failed and rollback was incomplete",
                json!({
                    "apply_error": apply_error.body(),
                    "rollback_errors": failures,
                    "recovery": "restore the recorded paths and greetd enable state before retrying",
                }),
            )
        }
    }

    pub(super) fn create_tree(&mut self, path: &Path, mode: u32, owner: (u32, u32)) -> Result<()> {
        match self.context.filesystem.symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
            Ok(_) => Err(Error::new(
                "unsafe_startup_path",
                format!("unsafe directory destination: {}", path.display()),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let parent = path
                    .parent()
                    .ok_or_else(|| Error::new("unsafe_startup_path", "directory has no parent"))?;
                self.create_tree(parent, mode, owner)?;
                self.undo.push(Undo::RemoveDirectory(path.to_path_buf()));
                self.context
                    .filesystem
                    .create_dir(path)
                    .map_err(|error| Error::io("cannot create startup directory", &error))?;
                self.applied(&format!("create_dir:{}", path.display()))?;
                self.context
                    .filesystem
                    .set_mode(path, mode)
                    .map_err(|error| Error::io("cannot set startup directory mode", &error))?;
                self.applied(&format!("set_mode:{}", path.display()))?;
                self.context.ownership.set_owner(path, owner.0, owner.1)?;
                self.applied(&format!("set_owner:{}", path.display()))
            }
            Err(error) => Err(Error::io("cannot inspect startup directory", &error)),
        }
    }

    pub(super) fn ensure_directory(
        &mut self,
        path: &Path,
        mode: u32,
        owner: (u32, u32),
    ) -> Result<()> {
        match self.context.filesystem.symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                self.undo.push(metadata_undo(path, &metadata));
                self.context
                    .filesystem
                    .set_mode(path, mode)
                    .map_err(|error| Error::io("cannot set startup directory mode", &error))?;
                self.applied(&format!("set_mode:{}", path.display()))?;
                self.context.ownership.set_owner(path, owner.0, owner.1)?;
                self.applied(&format!("set_owner:{}", path.display()))
            }
            Ok(_) => Err(Error::new(
                "unsafe_startup_path",
                format!("unsafe directory destination: {}", path.display()),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.create_tree(path, mode, owner)
            }
            Err(error) => Err(Error::io("cannot inspect startup directory", &error)),
        }
    }

    pub(super) fn replace_file(
        &mut self,
        source: &Path,
        destination: &Path,
        backup: &Path,
        destination_owner: (u32, u32),
        backup_owner: (u32, u32),
    ) -> Result<bool> {
        let data = self
            .context
            .filesystem
            .read(source)
            .map_err(|error| Error::io("cannot read startup template", &error))?;
        let previous = self.file_snapshot(destination)?;
        if previous
            .as_ref()
            .is_some_and(|snapshot| snapshot.data == data)
        {
            return Ok(false);
        }
        if let Some(snapshot) = &previous {
            let parent = backup
                .parent()
                .ok_or_else(|| Error::new("unsafe_startup_path", "backup has no parent"))?;
            self.create_tree(parent, 0o700, backup_owner)?;
            self.write_file(backup, &snapshot.data, snapshot.mode)?;
        }
        let parent = destination
            .parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "destination has no parent"))?;
        self.create_tree(parent, 0o755, destination_owner)?;
        self.write_snapshot(destination, &data, 0o644, previous)?;
        Ok(true)
    }

    pub(super) fn rename(&mut self, source: &Path, destination: &Path) -> Result<()> {
        self.undo.push(Undo::Rename {
            from: destination.to_path_buf(),
            to: source.to_path_buf(),
        });
        self.context
            .filesystem
            .rename(source, destination)
            .map_err(|error| Error::io("cannot back up legacy user unit", &error))?;
        self.applied(&format!(
            "rename:{}->{}",
            source.display(),
            destination.display()
        ))
    }

    pub(super) fn set_owner(&mut self, path: &Path, owner: (u32, u32)) -> Result<()> {
        let metadata = self
            .context
            .filesystem
            .symlink_metadata(path)
            .map_err(|error| Error::io("cannot inspect startup ownership", &error))?;
        self.undo.push(metadata_undo(path, &metadata));
        self.context.ownership.set_owner(path, owner.0, owner.1)?;
        self.applied(&format!("set_owner:{}", path.display()))
    }

    pub(super) fn set_mode(&mut self, path: &Path, mode: u32) -> Result<()> {
        let metadata = self
            .context
            .filesystem
            .symlink_metadata(path)
            .map_err(|error| Error::io("cannot inspect startup mode", &error))?;
        self.undo.push(metadata_undo(path, &metadata));
        self.context
            .filesystem
            .set_mode(path, mode)
            .map_err(|error| Error::io("cannot set startup file mode", &error))?;
        self.applied(&format!("set_mode:{}", path.display()))
    }

    pub(super) fn write_new_file(
        &mut self,
        path: &Path,
        data: &[u8],
        mode: u32,
        owner: (u32, u32),
    ) -> Result<bool> {
        if self.file_snapshot(path)?.is_some() {
            return Ok(false);
        }
        let parent = path
            .parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "file has no parent"))?;
        self.create_tree(parent, 0o700, owner)?;
        self.write_snapshot(path, data, mode, None)?;
        self.set_owner(path, owner)?;
        Ok(true)
    }

    pub(super) fn journal_enable(
        &mut self,
        state: EnableState,
        display_manager: &Path,
        link_state: DisplayManagerState,
    ) {
        self.undo.push(Undo::RestoreLink {
            path: display_manager.to_path_buf(),
            state: link_state,
        });
        self.undo.push(Undo::RestoreEnable(state));
    }

    pub(super) fn applied(&self, operation: &str) -> Result<()> {
        self.context.mutations.applied(operation)
    }

    fn write_file(&mut self, path: &Path, data: &[u8], mode: u32) -> Result<()> {
        let previous = self.file_snapshot(path)?;
        self.write_snapshot(path, data, mode, previous)
    }

    fn write_snapshot(
        &mut self,
        path: &Path,
        data: &[u8],
        mode: u32,
        previous: Option<FileSnapshot>,
    ) -> Result<()> {
        self.undo.push(previous.map_or_else(
            || Undo::RemoveFile(path.to_path_buf()),
            |snapshot| Undo::RestoreFile {
                path: path.to_path_buf(),
                data: snapshot.data,
                mode: snapshot.mode,
                uid: snapshot.uid,
                gid: snapshot.gid,
            },
        ));
        self.context
            .filesystem
            .write_atomic(path, data, mode)
            .map_err(|error| Error::io("cannot write startup file", &error))?;
        self.context
            .filesystem
            .sync_directory(
                path.parent()
                    .ok_or_else(|| Error::new("unsafe_startup_path", "file has no parent"))?,
            )
            .map_err(|error| Error::io("cannot sync startup directory", &error))?;
        self.applied(&format!("write_file:{}", path.display()))
    }

    fn file_snapshot(&self, path: &Path) -> Result<Option<FileSnapshot>> {
        match self.context.filesystem.symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                let data = self
                    .context
                    .filesystem
                    .read(path)
                    .map_err(|error| Error::io("cannot read startup file", &error))?;
                Ok(Some(FileSnapshot {
                    data,
                    mode: metadata.permissions().mode() & 0o7777,
                    uid: metadata.uid(),
                    gid: metadata.gid(),
                }))
            }
            Ok(_) => Err(Error::new(
                "unsafe_startup_path",
                format!("unsafe file destination: {}", path.display()),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Error::io("cannot inspect startup file", &error)),
        }
    }
}

struct FileSnapshot {
    data: Vec<u8>,
    mode: u32,
    uid: u32,
    gid: u32,
}
