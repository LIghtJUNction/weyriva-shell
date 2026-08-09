use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::error::{Error, Result};
use crate::manifest::valid_identifier;
use crate::model::{Candidate, PluginProfile, Provider, V4Runtime};

const PINNED_PLUGIN_ID: &str = "kaomoji-provider";
const PINNED_HANDLER_TARGET: &str = "plugin:kaomoji";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug)]
pub struct V4Candidate {
    pub root: PathBuf,
    pub provider: Provider,
    pub runtime: V4Runtime,
    pub settings_defaults: BTreeMap<String, JsonValue>,
    pub profile: PluginProfile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawManifest {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    #[serde(default)]
    _tags: Vec<String>,
    #[serde(default)]
    _official: bool,
    entry_points: RawEntries,
    #[serde(default, rename = "minNoctaliaVersion")]
    _min_noctalia_version: Option<String>,
    #[serde(default, rename = "license")]
    _license: Option<JsonValue>,
    #[serde(default, rename = "repository")]
    _repository: Option<JsonValue>,
    #[serde(default, rename = "dependencies")]
    _dependencies: Option<JsonValue>,
    #[serde(default)]
    metadata: RawMetadata,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEntries {
    main: String,
    launcher_provider: String,
    #[serde(flatten)]
    extra: BTreeMap<String, JsonValue>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMetadata {
    #[serde(default)]
    command_prefix: Option<String>,
    #[serde(flatten)]
    _other: BTreeMap<String, JsonValue>,
}

/// Parses the pinned legacy-v4 Kaomoji launcher profile.
///
/// # Errors
///
/// Returns an error for any other plugin, entry shape, unsafe QML path,
/// unsupported input surface, or missing canonical IPC target.
pub fn parse_plugin(root: &Path) -> Result<V4Candidate> {
    let manifest_path = root.join("manifest.json");
    let metadata = regular_file(&manifest_path, "manifest.json")?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(Error::new(
            "plugin_too_large",
            "manifest.json exceeds 1 MiB",
        ));
    }
    let text = fs::read_to_string(&manifest_path)
        .map_err(|error| Error::io("cannot read manifest.json", &error))?;
    let raw: RawManifest = serde_json::from_str(&text)
        .map_err(|error| Error::new("invalid_manifest", error.to_string()))?;
    validate_identity(&raw)?;
    if !raw.entry_points.extra.is_empty() {
        return Err(Error::new(
            "unsupported_plugin",
            "v4 profile supports only main and launcherProvider",
        ));
    }
    let main = validate_qml_entry(root, &raw.entry_points.main, "main")?;
    let launcher = validate_qml_entry(
        root,
        &raw.entry_points.launcher_provider,
        "launcherProvider",
    )?;
    let main_source = fs::read_to_string(root.join(&main))
        .map_err(|error| Error::io("cannot read v4 main entry", &error))?;
    reject_surface(&main_source)?;
    let launcher_source = fs::read_to_string(root.join(&launcher))
        .map_err(|error| Error::io("cannot read v4 launcher entry", &error))?;
    reject_surface(&launcher_source)?;
    let handler_target = parse_handler_target(&main_source)?;
    let prefix = raw
        .metadata
        .command_prefix
        .unwrap_or_else(|| "kaomoji".to_owned());
    if !valid_identifier(&prefix) {
        return Err(Error::new(
            "invalid_manifest",
            "v4 command prefix is invalid",
        ));
    }
    Ok(V4Candidate {
        root: root.to_path_buf(),
        provider: Provider {
            plugin_id: raw.id,
            entry_id: "launcher".to_owned(),
            entry: path_text(&launcher),
            prefix,
            name: raw.name,
            version: raw.version,
            glyph: String::new(),
            include_in_global_search: true,
            debounce_ms: 120,
            categories: Vec::new(),
            service: None,
        },
        runtime: V4Runtime {
            main: path_text(&main),
            launcher_provider: path_text(&launcher),
            handler_target,
        },
        settings_defaults: BTreeMap::new(),
        profile: PluginProfile::V4Qml,
    })
}

fn validate_identity(raw: &RawManifest) -> Result<()> {
    if raw.id != PINNED_PLUGIN_ID
        || raw.name.trim().is_empty()
        || raw.version.trim().is_empty()
        || raw.author.trim().is_empty()
        || raw.description.trim().is_empty()
    {
        return Err(Error::new(
            "unsupported_plugin",
            "v4 profile is pinned to kaomoji-provider",
        ));
    }
    Ok(())
}

fn validate_qml_entry(root: &Path, value: &str, label: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.extension().and_then(|extension| extension.to_str()) != Some("qml")
        || path.components().any(|component| {
            !matches!(component, Component::Normal(_) | Component::CurDir)
                || component == Component::CurDir
        })
    {
        return Err(Error::new(
            "invalid_manifest",
            format!("{label} must be a safe relative QML path"),
        ));
    }
    regular_file(&root.join(path), label)?;
    Ok(path.to_path_buf())
}

fn regular_file(path: &Path, label: &str) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io(&format!("cannot inspect {label}"), &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::new(
            "invalid_manifest",
            format!("{label} is not a regular file"),
        ));
    }
    Ok(metadata)
}

fn reject_surface(source: &str) -> Result<()> {
    const FORBIDDEN: &[&str] = &[
        "Window",
        "LayerShell",
        "PanelWindow",
        "MouseArea",
        "TapHandler",
        "HoverHandler",
        "Keys.on",
        "focus:",
    ];
    if FORBIDDEN.iter().any(|token| source.contains(token)) {
        return Err(Error::new(
            "unsupported_plugin",
            "v4 launcher slice forbids windows, layer shell, and input surfaces",
        ));
    }
    Ok(())
}

fn parse_handler_target(source: &str) -> Result<String> {
    let mut targets = Vec::new();
    let mut remaining = source;
    while let Some(index) = remaining.find("target:") {
        remaining = &remaining[index + "target:".len()..];
        let value = remaining.trim_start();
        let Some(quote) = value
            .chars()
            .next()
            .filter(|value| matches!(value, '"' | '\''))
        else {
            continue;
        };
        let quoted = &value[quote.len_utf8()..];
        let Some(end) = quoted.find(quote) else {
            continue;
        };
        let target = &quoted[..end];
        if let Some(alias) = target.strip_prefix("plugin:")
            && valid_identifier(alias)
        {
            targets.push(target.to_owned());
        }
        remaining = &quoted[end + quote.len_utf8()..];
    }
    if targets.len() == 1 && targets[0] == PINNED_HANDLER_TARGET {
        Ok(targets.remove(0))
    } else {
        Err(Error::new(
            "invalid_manifest",
            "v4 main must expose exactly the pinned plugin:kaomoji IPC target",
        ))
    }
}

fn path_text(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl From<V4Candidate> for Candidate {
    fn from(candidate: V4Candidate) -> Self {
        Self {
            root: candidate.root,
            provider: candidate.provider,
            settings_defaults: candidate.settings_defaults,
            profile: candidate.profile,
            v4_runtime: Some(candidate.runtime),
        }
    }
}
