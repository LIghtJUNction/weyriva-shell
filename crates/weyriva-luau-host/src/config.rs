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
    pub(crate) entry_id: String,
    pub(crate) entry_path: PathBuf,
    pub(crate) service: Option<ServiceConfig>,
    pub(crate) settings: JsonValue,
}

#[derive(Clone, Debug)]
pub(crate) struct ServiceConfig {
    pub(crate) id: String,
    pub(crate) entry_path: PathBuf,
}

#[derive(Default)]
struct CliArgs {
    plugin_dir: Option<String>,
    entry: Option<String>,
    entry_id: Option<String>,
    kind: Option<String>,
    service_id: Option<String>,
    service_entry: Option<String>,
    settings_json: Option<String>,
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
        let CliArgs {
            plugin_dir,
            entry,
            entry_id,
            kind,
            service_id,
            service_entry,
            settings_json,
        } = parse_cli(args)?;

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
        let entry_id = entry_id.unwrap_or_else(|| "launcher".to_owned());
        validate_entry_id(&entry_id, "launcher")?;
        let service = match (service_id, service_entry) {
            (None, None) => None,
            (Some(id), Some(entry)) => {
                validate_entry_id(&id, "service")?;
                if id == entry_id {
                    return Err(HostError::new(
                        "invalid_cli",
                        "launcher and service entry ids must be unique",
                    ));
                }
                Some((id, entry))
            }
            _ => {
                return Err(HostError::new(
                    "invalid_cli",
                    "`--service-id` and `--service-entry` must be provided together",
                ));
            }
        };
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

        let entry_path = resolve_luau_entry(&plugin_dir, &entry, "entry")?;
        let service = service
            .map(|(id, entry)| -> HostResult<ServiceConfig> {
                Ok(ServiceConfig {
                    id,
                    entry_path: resolve_luau_entry(&plugin_dir, &entry, "service entry")?,
                })
            })
            .transpose()?;

        Ok(Self {
            plugin_dir,
            entry_id,
            entry_path,
            service,
            settings,
        })
    }
}

fn parse_cli(args: impl IntoIterator<Item = OsString>) -> HostResult<CliArgs> {
    let mut parsed = CliArgs::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        let argument = argument
            .into_string()
            .map_err(|_| HostError::new("invalid_cli", "CLI arguments must be valid UTF-8"))?;
        if matches!(argument.as_str(), "--help" | "-h") {
            return Err(HostError::new(
                "help",
                "usage: weyriva-luau-host --plugin-dir ABSOLUTE --entry RELATIVE [--entry-id ID] --kind launcher_provider [--service-id ID --service-entry RELATIVE] --settings-json JSON",
            ));
        }
        let value = args
            .next()
            .ok_or_else(|| {
                HostError::new("invalid_cli", format!("missing value for `{argument}`"))
            })?
            .into_string()
            .map_err(|_| HostError::new("invalid_cli", "CLI values must be valid UTF-8"))?;
        let slot = match argument.as_str() {
            "--plugin-dir" => &mut parsed.plugin_dir,
            "--entry" => &mut parsed.entry,
            "--entry-id" => &mut parsed.entry_id,
            "--kind" => &mut parsed.kind,
            "--service-id" => &mut parsed.service_id,
            "--service-entry" => &mut parsed.service_entry,
            "--settings-json" => &mut parsed.settings_json,
            _ => {
                return Err(HostError::new(
                    "invalid_cli",
                    format!("unknown argument `{argument}`"),
                ));
            }
        };
        set_once(slot, value, &argument)?;
    }
    Ok(parsed)
}

fn validate_entry_id(value: &str, kind: &str) -> HostResult<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric);
    if valid {
        Ok(())
    } else {
        Err(HostError::new(
            "invalid_cli",
            format!("{kind} entry id is invalid"),
        ))
    }
}

fn resolve_luau_entry(plugin_dir: &Path, raw_entry: &str, label: &str) -> HostResult<PathBuf> {
    let entry = PathBuf::from(raw_entry);
    validate_relative_path(&entry, label)?;
    let entry_path = resolve_plugin_file(plugin_dir, &entry)?;
    if entry_path.extension().and_then(|value| value.to_str()) != Some("luau") {
        return Err(HostError::new(
            "invalid_entry",
            format!("{label} filename must end in `.luau`"),
        ));
    }
    Ok(entry_path)
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
