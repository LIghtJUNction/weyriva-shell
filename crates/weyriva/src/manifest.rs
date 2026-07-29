use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

use crate::error::{Error, Result};
use crate::model::{Candidate, Category, PLUGIN_API, Provider};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct RawManifest {
    id: String,
    name: String,
    version: String,
    plugin_api: u32,
    author: String,
    #[serde(default)]
    launcher_provider: Vec<RawProvider>,
    #[serde(default)]
    setting: Vec<RawSetting>,
    #[serde(default, rename = "license")]
    _license: Option<TomlValue>,
    #[serde(default, rename = "dependencies")]
    _dependencies: Option<TomlValue>,
    #[serde(default, rename = "tags")]
    _tags: Option<TomlValue>,
    #[serde(default, rename = "icon")]
    _icon: Option<TomlValue>,
    #[serde(default, rename = "description")]
    _description: Option<TomlValue>,
    #[serde(flatten)]
    extra: BTreeMap<String, TomlValue>,
}

#[derive(Deserialize)]
struct RawProvider {
    id: String,
    entry: String,
    prefix: String,
    #[serde(default)]
    glyph: String,
    #[serde(default)]
    include_in_global_search: bool,
    #[serde(default = "default_debounce")]
    debounce_ms: u64,
    #[serde(default, rename = "category")]
    categories: Vec<Category>,
    #[serde(flatten)]
    extra: BTreeMap<String, TomlValue>,
}

#[derive(Deserialize)]
struct RawSetting {
    key: String,
    #[serde(rename = "type")]
    kind: String,
    default: TomlValue,
    label_key: String,
    #[serde(default, rename = "description_key")]
    _description_key: Option<String>,
    #[serde(default)]
    options: Option<Vec<TomlValue>>,
    #[serde(default, rename = "min")]
    _min: Option<TomlValue>,
    #[serde(default, rename = "max")]
    _max: Option<TomlValue>,
    #[serde(default, rename = "step")]
    _step: Option<TomlValue>,
    #[serde(default, rename = "placeholder")]
    _placeholder: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, TomlValue>,
}

const fn default_debounce() -> u64 {
    120
}

#[must_use]
pub fn valid_plugin_id(value: &str) -> bool {
    let mut segments = value.split('/');
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(author), Some(plugin), None) if valid_identifier(author) && valid_identifier(plugin)
    )
}

#[must_use]
pub fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
}

/// Parses and validates one API3 single-launcher-provider manifest.
///
/// # Errors
///
/// Returns an error for unsupported surfaces, invalid metadata, unsafe entry
/// paths, invalid setting defaults, or unavailable entry files.
pub fn parse_plugin(root: &Path) -> Result<Candidate> {
    let manifest_path = root.join("plugin.toml");
    let metadata = fs::symlink_metadata(&manifest_path)
        .map_err(|error| Error::io("cannot inspect plugin.toml", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::new(
            "invalid_manifest",
            "plugin.toml is not a regular file",
        ));
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(Error::new("plugin_too_large", "plugin.toml exceeds 1 MiB"));
    }
    let text = fs::read_to_string(&manifest_path)
        .map_err(|error| Error::io("cannot read plugin.toml", &error))?;
    let raw: RawManifest = toml::from_str(&text)?;
    validate_manifest_root(&raw)?;
    let raw_provider = raw
        .launcher_provider
        .into_iter()
        .next()
        .ok_or_else(|| Error::new("unsupported_plugin", "missing launcher provider"))?;
    let entry = validate_relative_luau(&raw_provider.entry)?;
    let entry_path = root.join(&entry);
    let entry_metadata = fs::symlink_metadata(&entry_path)
        .map_err(|error| Error::io("cannot inspect launcher entry", &error))?;
    if entry_metadata.file_type().is_symlink() || !entry_metadata.is_file() {
        return Err(Error::new(
            "invalid_manifest",
            "launcher entry is not a regular file",
        ));
    }
    let settings_defaults = validate_settings(raw.setting)?;
    Ok(Candidate {
        root: root.to_path_buf(),
        provider: Provider {
            plugin_id: raw.id,
            entry_id: raw_provider.id,
            entry: path_to_slashes(&entry),
            prefix: raw_provider.prefix,
            name: raw.name,
            version: raw.version,
            glyph: raw_provider.glyph,
            include_in_global_search: raw_provider.include_in_global_search,
            debounce_ms: raw_provider.debounce_ms,
            categories: raw_provider.categories,
        },
        settings_defaults,
    })
}

fn validate_manifest_root(raw: &RawManifest) -> Result<()> {
    if !raw.extra.is_empty() {
        return Err(Error::new(
            "unsupported_plugin",
            format!(
                "unsupported or mixed plugin surface: {}",
                raw.extra.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    if !valid_plugin_id(&raw.id) {
        return Err(Error::new(
            "invalid_manifest",
            "id must be canonical author/plugin",
        ));
    }
    if raw.name.trim().is_empty() || raw.version.trim().is_empty() || raw.author.trim().is_empty() {
        return Err(Error::new(
            "invalid_manifest",
            "name, version, and author must be non-empty",
        ));
    }
    if raw.plugin_api != PLUGIN_API {
        return Err(Error::new(
            "unsupported_plugin_api",
            format!("plugin_api must be {PLUGIN_API}"),
        ));
    }
    if raw.launcher_provider.len() != 1 {
        return Err(Error::new(
            "unsupported_plugin",
            "plugin must contain exactly one launcher_provider",
        ));
    }
    let provider = &raw.launcher_provider[0];
    if !provider.extra.is_empty() || !valid_identifier(&provider.id) {
        return Err(Error::new(
            "invalid_manifest",
            "launcher provider contains unsupported fields or invalid id",
        ));
    }
    if provider.prefix.is_empty()
        || provider.prefix.len() > 32
        || provider.prefix.chars().any(char::is_whitespace)
        || provider.debounce_ms > 2_000
    {
        return Err(Error::new(
            "invalid_manifest",
            "launcher prefix or debounce is invalid",
        ));
    }
    if provider
        .categories
        .iter()
        .any(|category| category.label.is_empty())
    {
        return Err(Error::new(
            "invalid_manifest",
            "launcher category label is empty",
        ));
    }
    Ok(())
}

fn validate_settings(settings: Vec<RawSetting>) -> Result<BTreeMap<String, JsonValue>> {
    let mut keys = BTreeSet::new();
    let mut defaults = BTreeMap::new();
    for setting in settings {
        if !setting.extra.is_empty()
            || !valid_identifier(&setting.key)
            || !keys.insert(setting.key.clone())
            || setting.label_key.is_empty()
        {
            return Err(Error::new(
                "invalid_manifest",
                "setting metadata is invalid",
            ));
        }
        validate_setting_default(&setting)?;
        let default = serde_json::to_value(setting.default).map_err(|error| {
            Error::new(
                "invalid_manifest",
                format!("cannot convert setting default: {error}"),
            )
        })?;
        defaults.insert(setting.key, default);
    }
    Ok(defaults)
}

fn validate_setting_default(setting: &RawSetting) -> Result<()> {
    let valid = match setting.kind.as_str() {
        "bool" => setting.default.is_bool(),
        "string" | "select" => setting.default.is_str(),
        "int" => setting.default.is_integer(),
        "double" => setting.default.is_float() || setting.default.is_integer(),
        "string_list" => setting
            .default
            .as_array()
            .is_some_and(|values| values.iter().all(TomlValue::is_str)),
        _ => false,
    };
    if !valid {
        return Err(Error::new(
            "invalid_manifest",
            format!("setting {} has invalid default", setting.key),
        ));
    }
    if setting.kind == "select" {
        let options = setting
            .options
            .as_ref()
            .ok_or_else(|| Error::new("invalid_manifest", "select setting requires options"))?;
        let selected = setting.default.as_str().unwrap_or_default();
        let matches = options.iter().any(|option| {
            option.as_str() == Some(selected)
                || option
                    .as_table()
                    .and_then(|table| table.get("value"))
                    .and_then(TomlValue::as_str)
                    == Some(selected)
        });
        if !matches {
            return Err(Error::new(
                "invalid_manifest",
                "select default is not one of its options",
            ));
        }
    }
    Ok(())
}

fn validate_relative_luau(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.extension().and_then(|value| value.to_str()) != Some("luau")
    {
        return Err(Error::new(
            "invalid_manifest",
            "launcher entry must be a safe relative .luau path",
        ));
    }
    Ok(path.to_path_buf())
}

fn path_to_slashes(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
