use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::Value as JsonValue;

use crate::error::{HostError, HostResult};

const MAX_SETTINGS_BYTES: usize = 64 * 1_024;
const MAX_ENTRY_BYTES: usize = 4_096;
pub(crate) const MAX_PLUGIN_DATA_BYTES: usize = 1_024 * 1_024;

#[derive(Clone, Debug)]
pub struct HostConfig {
    pub(crate) plugin_dir: PathBuf,
    pub(crate) entry_path: PathBuf,
    pub(crate) settings: JsonValue,
}

impl HostConfig {
    /// Parses and validates one launcher host process configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when required arguments are absent or duplicated, the
    /// entry kind is unsupported, settings are invalid, or either path escapes
    /// the canonical plugin directory.
    pub fn from_args(args: impl IntoIterator<Item = OsString>) -> HostResult<Self> {
        let mut plugin_dir = None;
        let mut entry = None;
        let mut kind = None;
        let mut settings_json = None;
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            let argument = argument
                .into_string()
                .map_err(|_| HostError::new("invalid_cli", "CLI arguments must be valid UTF-8"))?;
            let value = match argument.as_str() {
                "--plugin-dir" | "--entry" | "--kind" | "--settings-json" => args
                    .next()
                    .ok_or_else(|| {
                        HostError::new("invalid_cli", format!("missing value for `{argument}`"))
                    })?
                    .into_string()
                    .map_err(|_| HostError::new("invalid_cli", "CLI values must be valid UTF-8"))?,
                "--help" | "-h" => {
                    return Err(HostError::new(
                        "help",
                        "usage: weyriva-luau-host --plugin-dir ABSOLUTE --entry RELATIVE --kind launcher_provider --settings-json JSON",
                    ));
                }
                _ => {
                    return Err(HostError::new(
                        "invalid_cli",
                        format!("unknown argument `{argument}`"),
                    ));
                }
            };
            match argument.as_str() {
                "--plugin-dir" => set_once(&mut plugin_dir, value, "--plugin-dir")?,
                "--entry" => set_once(&mut entry, value, "--entry")?,
                "--kind" => set_once(&mut kind, value, "--kind")?,
                "--settings-json" => set_once(&mut settings_json, value, "--settings-json")?,
                _ => unreachable!(),
            }
        }

        let plugin_dir =
            plugin_dir.ok_or_else(|| HostError::new("invalid_cli", "missing `--plugin-dir`"))?;
        let entry = entry.ok_or_else(|| HostError::new("invalid_cli", "missing `--entry`"))?;
        let kind = kind.ok_or_else(|| HostError::new("invalid_cli", "missing `--kind`"))?;
        if kind != "launcher_provider" {
            return Err(HostError::new(
                "unsupported_kind",
                format!("entry kind `{kind}` is not implemented"),
            ));
        }
        let settings_json = settings_json.unwrap_or_else(|| "{}".to_owned());
        if settings_json.len() > MAX_SETTINGS_BYTES {
            return Err(HostError::new(
                "settings_limit",
                format!("settings JSON exceeds {MAX_SETTINGS_BYTES} bytes"),
            ));
        }
        let settings: JsonValue = serde_json::from_str(&settings_json).map_err(|error| {
            HostError::new(
                "invalid_settings",
                format!("invalid settings JSON: {error}"),
            )
        })?;
        if !settings.is_object() {
            return Err(HostError::new(
                "invalid_settings",
                "settings JSON must be an object",
            ));
        }

        let requested_root = PathBuf::from(plugin_dir);
        if !requested_root.is_absolute() {
            return Err(HostError::new(
                "invalid_plugin_dir",
                "`--plugin-dir` must be absolute",
            ));
        }
        let plugin_dir = requested_root.canonicalize().map_err(|error| {
            HostError::new(
                "invalid_plugin_dir",
                format!("cannot canonicalize plugin directory: {error}"),
            )
        })?;
        if !plugin_dir.is_dir() {
            return Err(HostError::new(
                "invalid_plugin_dir",
                "plugin directory is not a directory",
            ));
        }

        let entry = PathBuf::from(entry);
        validate_relative_path(&entry, "entry")?;
        let entry_path = resolve_plugin_file(&plugin_dir, &entry)?;
        if entry_path.extension().and_then(|value| value.to_str()) != Some("luau") {
            return Err(HostError::new(
                "invalid_entry",
                "entry filename must end in `.luau`",
            ));
        }

        Ok(Self {
            plugin_dir,
            entry_path,
            settings,
        })
    }
}

pub(crate) fn read_plugin_file(plugin_dir: &Path, relative_path: &str) -> HostResult<String> {
    if relative_path.len() > MAX_ENTRY_BYTES {
        return Err(HostError::new(
            "path_limit",
            format!("plugin path exceeds {MAX_ENTRY_BYTES} bytes"),
        ));
    }
    let relative_path = Path::new(relative_path);
    validate_relative_path(relative_path, "plugin file")?;
    let path = resolve_plugin_file(plugin_dir, relative_path)?;
    let metadata = fs::metadata(&path).map_err(|error| {
        HostError::new(
            "file_access",
            format!(
                "cannot inspect plugin file `{}`: {error}",
                relative_path.display()
            ),
        )
    })?;
    if !metadata.is_file() {
        return Err(HostError::new(
            "file_access",
            format!(
                "plugin path `{}` is not a regular file",
                relative_path.display()
            ),
        ));
    }
    if metadata.len() > MAX_PLUGIN_DATA_BYTES as u64 {
        return Err(HostError::new(
            "file_limit",
            "plugin file exceeds the 1 MiB read limit",
        ));
    }
    fs::read_to_string(path).map_err(|error| {
        HostError::new(
            "file_access",
            format!(
                "cannot read plugin file `{}`: {error}",
                relative_path.display()
            ),
        )
    })
}

fn set_once(slot: &mut Option<String>, value: String, name: &str) -> HostResult<()> {
    if slot.replace(value).is_some() {
        return Err(HostError::new("invalid_cli", format!("duplicate `{name}`")));
    }
    Ok(())
}

fn validate_relative_path(path: &Path, label: &str) -> HostResult<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(HostError::new(
            "file_access",
            format!("{label} path must be non-empty and relative"),
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(HostError::new(
            "file_access",
            format!("{label} path must remain beneath the plugin directory"),
        ));
    }
    Ok(())
}

fn resolve_plugin_file(plugin_dir: &Path, relative_path: &Path) -> HostResult<PathBuf> {
    let requested = plugin_dir.join(relative_path);
    let canonical = requested.canonicalize().map_err(|error| {
        HostError::new(
            "file_access",
            format!(
                "cannot resolve plugin path `{}`: {error}",
                relative_path.display()
            ),
        )
    })?;
    if !canonical.starts_with(plugin_dir) {
        return Err(HostError::new(
            "file_access",
            format!(
                "plugin path `{}` resolves outside the plugin directory",
                relative_path.display()
            ),
        ));
    }
    Ok(canonical)
}
