#![expect(
    clippy::missing_errors_doc,
    reason = "source operations share the crate's typed control-plane error contract"
)]

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value as JsonValue, json};

use crate::archive;
use crate::error::{Error, Result};
use crate::manifest::{parse_plugin, valid_identifier};
use crate::model::{
    Candidate, MAX_ARCHIVE_BYTES, PluginProfile, Provenance, SOURCES_SCHEMA, STATE_SCHEMA,
    SourcesDocument, StateDocument, UserSource,
};
use crate::paths::Paths;
use crate::storage::{atomic_json, read_json};
use crate::tree::validate_and_hash;

const OFFICIAL_REVISION: &str = "4b03f0a5e3b701c5a3ade87d35ed62c1699f93c6";
const COMMUNITY_REVISION: &str = "35afaa444de6389164360b1ecadb87c972b32912";
const V4_OFFICIAL_REVISION: &str = "ea21cb63d063075bc0acd72d8b946ce2c5eef00d";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize)]
struct SourceView {
    name: String,
    kind: String,
    builtin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<PluginProfile>,
}

enum SourceSpec {
    Local(UserSource),
    Github {
        name: &'static str,
        repository: &'static str,
        revision: &'static str,
    },
}

pub fn list(paths: &Paths) -> Result<JsonValue> {
    let users = load_sources(paths)?;
    let mut sources = builtin_views();
    sources.extend(users.sources.into_iter().map(|source| SourceView {
        name: source.name,
        kind: source.kind,
        builtin: source.builtin,
        path: Some(source.path.display().to_string()),
        repository: None,
        revision: None,
        profile: None,
    }));
    Ok(json!({"schema": SOURCES_SCHEMA, "sources": sources}))
}

pub fn add(paths: &Paths, name: &str, path: &Path) -> Result<JsonValue> {
    if !valid_identifier(name) || matches!(name, "official" | "community" | "official-v4") {
        return Err(Error::new(
            "invalid_source",
            "source name is invalid or reserved",
        ));
    }
    let requested = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot inspect source path", &error))?;
    if requested.file_type().is_symlink() || !requested.is_dir() {
        return Err(Error::new(
            "unsafe_source",
            "source path must be a regular directory",
        ));
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| Error::io("cannot resolve source path", &error))?;
    let mut document = load_sources(paths)?;
    let replaced = document.sources.iter().any(|source| source.name == name);
    document.sources.retain(|source| source.name != name);
    let source = UserSource {
        name: name.to_owned(),
        kind: "path".to_owned(),
        path: canonical,
        builtin: false,
    };
    document.sources.push(source.clone());
    atomic_json(&paths.sources_file(), &document)?;
    Ok(json!({"source": source, "replaced": replaced}))
}

pub fn remove(paths: &Paths, name: &str) -> Result<JsonValue> {
    if matches!(name, "official" | "community" | "official-v4") {
        return Err(Error::new(
            "builtin_source",
            "built-in source cannot be removed",
        ));
    }
    let mut document = load_sources(paths)?;
    let before = document.sources.len();
    document.sources.retain(|source| source.name != name);
    if before == document.sources.len() {
        return Err(Error::new(
            "source_not_found",
            format!("unknown source: {name}"),
        ));
    }
    atomic_json(&paths.sources_file(), &document)?;
    Ok(json!({"removed": name}))
}

pub fn load_state(paths: &Paths) -> Result<StateDocument> {
    let document: StateDocument = read_json(&paths.state_file())?;
    if document.schema != STATE_SCHEMA {
        return Err(Error::new(
            "unsupported_state",
            "plugin state schema must be 1",
        ));
    }
    Ok(document)
}

fn load_sources(paths: &Paths) -> Result<SourcesDocument> {
    let document: SourcesDocument = read_json(&paths.sources_file())?;
    if document.schema != SOURCES_SCHEMA {
        return Err(Error::new(
            "unsupported_state",
            "plugin sources schema must be 1",
        ));
    }
    Ok(document)
}

pub(crate) fn resolve(
    paths: &Paths,
    plugin_id: &str,
    temporary: &Path,
    profile: PluginProfile,
) -> Result<(crate::model::Candidate, Provenance)> {
    let users = load_sources(paths)?;
    let mut specifications = match profile {
        PluginProfile::V5Luau => vec![
            SourceSpec::Github {
                name: "official",
                repository: "noctalia-dev/official-plugins",
                revision: OFFICIAL_REVISION,
            },
            SourceSpec::Github {
                name: "community",
                repository: "noctalia-dev/community-plugins",
                revision: COMMUNITY_REVISION,
            },
        ],
        PluginProfile::V4Qml => vec![SourceSpec::Github {
            name: "official-v4",
            repository: "noctalia-dev/legacy-v4-plugins",
            revision: V4_OFFICIAL_REVISION,
        }],
    };
    specifications.extend(users.sources.into_iter().map(SourceSpec::Local));
    let mut download_errors = Vec::new();
    for (index, source) in specifications.into_iter().enumerate().rev() {
        let resolution = resolve_one(&source, plugin_id, temporary, index, profile);
        match resolution {
            Ok(value) => return Ok(value),
            Err(error)
                if matches!(
                    error.code(),
                    "plugin_not_found" | "source_download_failed" | "source_unavailable"
                ) =>
            {
                if error.code() != "plugin_not_found" {
                    download_errors.push(error.to_string());
                }
            }
            Err(error) => return Err(error),
        }
    }
    let suffix = if download_errors.is_empty() {
        String::new()
    } else {
        format!(" ({})", download_errors.join("; "))
    };
    Err(Error::new(
        "plugin_not_found",
        format!("plugin {plugin_id} was not found{suffix}"),
    ))
}

fn resolve_one(
    source: &SourceSpec,
    plugin_id: &str,
    temporary: &Path,
    index: usize,
    profile: PluginProfile,
) -> Result<(crate::model::Candidate, Provenance)> {
    match source {
        SourceSpec::Local(source) => {
            let candidate = scan_source(&source.path, plugin_id, profile)?;
            let provenance = Provenance {
                source: source.name.clone(),
                kind: source.kind.clone(),
                path: candidate.root.display().to_string(),
                revision: None,
            };
            Ok((candidate, provenance))
        }
        SourceSpec::Github {
            name,
            repository,
            revision,
        } => {
            let slot = temporary.join(format!("source-{index}"));
            fs::create_dir(&slot)
                .map_err(|error| Error::io("cannot create source download slot", &error))?;
            let url = format!("https://github.com/{repository}/archive/{revision}.tar.gz");
            let archive_path = slot.join("source.tar.gz");
            download(&url, &archive_path)?;
            let extracted = archive::extract(&archive_path, &slot.join("extracted"))?;
            let candidate = scan_source(&extracted, plugin_id, profile)?;
            let provenance = Provenance {
                source: (*name).to_owned(),
                kind: "github".to_owned(),
                path: url,
                revision: Some((*revision).to_owned()),
            };
            Ok((candidate, provenance))
        }
    }
}

fn scan_source(root: &Path, plugin_id: &str, profile: PluginProfile) -> Result<Candidate> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| Error::io("cannot inspect plugin source", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "source_unavailable",
            "source path is not a directory",
        ));
    }
    let mut matches = Vec::new();
    let manifest_name = match profile {
        PluginProfile::V5Luau => "plugin.toml",
        PluginProfile::V4Qml => "manifest.json",
    };
    if root.join(manifest_name).is_file() {
        matches.push(root.to_path_buf());
    }
    for entry in
        fs::read_dir(root).map_err(|error| Error::io("cannot read plugin source", &error))?
    {
        let entry = entry.map_err(|error| Error::io("cannot read plugin source entry", &error))?;
        let metadata = entry
            .file_type()
            .map_err(|error| Error::io("cannot inspect plugin source entry", &error))?;
        if metadata.is_symlink() {
            return Err(Error::new(
                "unsafe_source",
                "source contains a top-level symlink",
            ));
        }
        if metadata.is_dir() && entry.path().join(manifest_name).is_file() {
            matches.push(entry.path());
        }
    }
    let mut selected = Vec::new();
    for directory in matches {
        let text = match fs::read_to_string(directory.join(manifest_name)) {
            Ok(text) if text.len() <= 1024 * 1024 => text,
            _ => continue,
        };
        let id = match profile {
            PluginProfile::V5Luau => toml::from_str::<toml::Value>(&text).ok().and_then(|value| {
                value
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            }),
            PluginProfile::V4Qml => {
                serde_json::from_str::<JsonValue>(&text)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("id")
                            .and_then(JsonValue::as_str)
                            .map(str::to_owned)
                    })
            }
        };
        if id.as_deref() == Some(plugin_id) {
            selected.push(directory);
        }
    }
    if selected.len() != 1 {
        return Err(Error::new(
            if selected.is_empty() {
                "plugin_not_found"
            } else {
                "duplicate_plugin"
            },
            format!("source must contain exactly one plugin with id {plugin_id}"),
        ));
    }
    let candidate = parse_candidate(&selected[0], profile)?;
    validate_and_hash(&candidate.root)?;
    Ok(candidate)
}

pub(crate) fn parse_candidate(root: &Path, profile: PluginProfile) -> Result<Candidate> {
    match profile {
        PluginProfile::V5Luau => parse_plugin(root),
        PluginProfile::V4Qml => crate::v4_manifest::parse_plugin(root).map(Into::into),
    }
}

fn download(url: &str, destination: &Path) -> Result<()> {
    let agent = ureq::AgentBuilder::new().timeout(DOWNLOAD_TIMEOUT).build();
    let response = agent
        .get(url)
        .set("User-Agent", "Weyriva-Plugins/0.1")
        .call()
        .map_err(|error| Error::new("source_download_failed", error.to_string()))?;
    let mut reader = response.into_reader().take(MAX_ARCHIVE_BYTES + 1);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .map_err(|error| Error::io("cannot create source archive", &error))?;
    let copied = std::io::copy(&mut reader, &mut output)
        .map_err(|error| Error::io("cannot download source archive", &error))?;
    if copied > MAX_ARCHIVE_BYTES {
        return Err(Error::new(
            "archive_too_large",
            "source archive exceeds download limit",
        ));
    }
    output
        .flush()
        .map_err(|error| Error::io("cannot flush source archive", &error))
}

fn builtin_views() -> Vec<SourceView> {
    vec![
        SourceView {
            name: "official".to_owned(),
            kind: "github".to_owned(),
            builtin: true,
            path: None,
            repository: Some("noctalia-dev/official-plugins".to_owned()),
            revision: Some(OFFICIAL_REVISION.to_owned()),
            profile: Some(PluginProfile::V5Luau),
        },
        SourceView {
            name: "community".to_owned(),
            kind: "github".to_owned(),
            builtin: true,
            path: None,
            repository: Some("noctalia-dev/community-plugins".to_owned()),
            revision: Some(COMMUNITY_REVISION.to_owned()),
            profile: Some(PluginProfile::V5Luau),
        },
        SourceView {
            name: "official-v4".to_owned(),
            kind: "github".to_owned(),
            builtin: true,
            path: None,
            repository: Some("noctalia-dev/legacy-v4-plugins".to_owned()),
            revision: Some(V4_OFFICIAL_REVISION.to_owned()),
            profile: Some(PluginProfile::V4Qml),
        },
    ]
}
