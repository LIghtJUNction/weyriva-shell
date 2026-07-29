use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::safety::FileSystem;

// Durable on-disk format: one UTF-8 JSON object with `format` and exact `target`
// strings, followed by a newline. It is deliberately a regular file, never a link.
const FORMAT: &str = "weyriva-display-manager-target-v1";

#[derive(Deserialize, Serialize)]
struct Record<'a> {
    format: &'a str,
    target: &'a str,
}

pub(super) fn encode(display_manager: &Path, target: &Path) -> Result<Vec<u8>> {
    let target = validate_target(display_manager, target)?;
    let mut encoded = serde_json::to_vec(&Record {
        format: FORMAT,
        target,
    })?;
    encoded.push(b'\n');
    Ok(encoded)
}

pub(super) fn validate_existing(
    filesystem: &dyn FileSystem,
    path: &Path,
    display_manager: &Path,
) -> Result<bool> {
    match filesystem.symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            let data = filesystem
                .read(path)
                .map_err(|error| Error::io("cannot read display-manager backup", &error))?;
            decode(&data, display_manager)?;
            Ok(true)
        }
        Ok(_) => Err(Error::new(
            "unsafe_startup_path",
            format!("unsafe display-manager backup: {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::io("cannot inspect display-manager backup", &error)),
    }
}

pub(super) fn resolves_to_greetd(display_manager: &Path, target: &Path) -> Result<bool> {
    validate_target(display_manager, target)?;
    Ok(resolve(display_manager, target)? == Path::new(super::preflight::EXPECTED_DISPLAY_MANAGER))
}

fn decode<'a>(data: &'a [u8], display_manager: &Path) -> Result<&'a str> {
    let record: Record<'_> = serde_json::from_slice(data).map_err(|_| {
        Error::new(
            "unsafe_startup_path",
            "display-manager backup metadata is malformed",
        )
    })?;
    if record.format != FORMAT {
        return Err(Error::new(
            "unsafe_startup_path",
            "display-manager backup metadata has an unsupported format",
        ));
    }
    validate_target(display_manager, Path::new(record.target))
}

fn validate_target<'a>(display_manager: &Path, target: &'a Path) -> Result<&'a str> {
    resolve(display_manager, target)?;
    target
        .to_str()
        .filter(|value| !value.contains('\0'))
        .ok_or_else(|| unsafe_target(target))
}

fn resolve(display_manager: &Path, target: &Path) -> Result<PathBuf> {
    let mut resolved = if target.is_absolute() {
        PathBuf::from("/")
    } else {
        display_manager
            .parent()
            .ok_or_else(|| unsafe_target(target))?
            .to_path_buf()
    };
    let mut normal = 0usize;
    for component in target.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(value) => {
                resolved.push(value);
                normal += 1;
            }
            Component::ParentDir if resolved.pop() => {}
            Component::ParentDir | Component::Prefix(_) => return Err(unsafe_target(target)),
        }
    }
    if normal == 0 {
        Err(unsafe_target(target))
    } else {
        Ok(resolved)
    }
}

fn unsafe_target(target: &Path) -> Error {
    Error::new(
        "unsafe_startup_path",
        format!(
            "unsafe display-manager link target: {}",
            target.as_os_str().to_string_lossy()
        ),
    )
}
