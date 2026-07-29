use std::fs::Metadata;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::process::{CommandSpec, command_text, os};

use super::model::{DisplayManagerState, StartupContext};

#[derive(Clone, Copy)]
pub(super) enum EnableState {
    Enabled,
    Disabled,
}

pub(super) enum Undo {
    RemoveFile(PathBuf),
    RemoveDirectory(PathBuf),
    RestoreFile {
        path: PathBuf,
        data: Vec<u8>,
        mode: u32,
        uid: u32,
        gid: u32,
    },
    RestoreMetadata {
        path: PathBuf,
        mode: u32,
        uid: u32,
        gid: u32,
        symlink: bool,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    RestoreLink {
        path: PathBuf,
        state: DisplayManagerState,
    },
    RestoreEnable(EnableState),
}

impl Undo {
    pub(super) fn label(&self) -> String {
        match self {
            Self::RemoveFile(path) => format!("remove_file:{}", path.display()),
            Self::RemoveDirectory(path) => format!("remove_dir:{}", path.display()),
            Self::RestoreFile { path, .. } => format!("restore_file:{}", path.display()),
            Self::RestoreMetadata { path, .. } => {
                format!("restore_metadata:{}", path.display())
            }
            Self::Rename { from, to } => {
                format!("rename:{}->{}", from.display(), to.display())
            }
            Self::RestoreLink { path, .. } => format!("restore_link:{}", path.display()),
            Self::RestoreEnable(_) => "restore_enable:greetd.service".to_owned(),
        }
    }
}

pub(super) fn metadata_undo(path: &Path, metadata: &Metadata) -> Undo {
    Undo::RestoreMetadata {
        path: path.to_path_buf(),
        mode: metadata.permissions().mode() & 0o7777,
        uid: metadata.uid(),
        gid: metadata.gid(),
        symlink: metadata.file_type().is_symlink(),
    }
}

pub(super) fn execute(context: &StartupContext, undo: Undo) -> Result<()> {
    match undo {
        Undo::RemoveFile(path) => remove_file(context, &path),
        Undo::RemoveDirectory(path) => remove_directory(context, &path),
        Undo::RestoreFile {
            path,
            data,
            mode,
            uid,
            gid,
        } => {
            context
                .filesystem
                .write_atomic(&path, &data, mode)
                .map_err(|error| Error::io("cannot restore startup file", &error))?;
            sync_parent(context, &path)?;
            context.ownership.set_owner(&path, uid, gid)
        }
        Undo::RestoreMetadata {
            path,
            mode,
            uid,
            gid,
            symlink,
        } => {
            if !symlink {
                context
                    .filesystem
                    .set_mode(&path, mode)
                    .map_err(|error| Error::io("cannot restore startup mode", &error))?;
            }
            context.ownership.set_owner(&path, uid, gid)
        }
        Undo::Rename { from, to } => context
            .filesystem
            .rename(&from, &to)
            .map_err(|error| Error::io("cannot restore legacy user unit", &error)),
        Undo::RestoreLink { path, state } => restore_link(context, &path, state),
        Undo::RestoreEnable(state) => restore_enable(context, state),
    }
}

fn remove_file(context: &StartupContext, path: &Path) -> Result<()> {
    match context.filesystem.remove_file(path) {
        Ok(()) => sync_parent(context, path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(
            "cannot remove startup file during rollback",
            &error,
        )),
    }
}

fn remove_directory(context: &StartupContext, path: &Path) -> Result<()> {
    match context.filesystem.remove_dir(path) {
        Ok(()) => sync_parent(context, path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(
            "cannot remove startup directory during rollback",
            &error,
        )),
    }
}

fn sync_parent(context: &StartupContext, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("unsafe_startup_path", "path has no parent"))?;
    context
        .filesystem
        .sync_directory(parent)
        .map_err(|error| Error::io("cannot sync startup directory during rollback", &error))
}

fn restore_link(context: &StartupContext, path: &Path, state: DisplayManagerState) -> Result<()> {
    match context.filesystem.symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_file(context, path)?,
        Ok(_) => {
            return Err(Error::new(
                "startup_rollback_failed",
                "display-manager path became a non-symlink during rollback",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(Error::io(
                "cannot inspect display-manager link during rollback",
                &error,
            ));
        }
    }
    if let DisplayManagerState::Link(target) = state {
        context
            .filesystem
            .symlink(&target, path)
            .map_err(|error| Error::io("cannot restore display-manager link", &error))?;
    }
    Ok(())
}

fn restore_enable(context: &StartupContext, state: EnableState) -> Result<()> {
    let action = match state {
        EnableState::Enabled => "enable",
        EnableState::Disabled => "disable",
    };
    let output = context.process.run(&CommandSpec::new(
        "systemctl",
        [os(action), os("--force"), os("greetd.service")],
    ))?;
    if output.code == 0 {
        Ok(())
    } else {
        Err(Error::new(
            "startup_rollback_failed",
            format!(
                "restoring greetd enable state failed: {}",
                command_text(&output)
            ),
        ))
    }
}
