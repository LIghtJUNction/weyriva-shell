use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub const STATE_SCHEMA: u32 = 1;
pub const SOURCES_SCHEMA: u32 = 1;
pub const PLUGIN_API: u32 = 3;
pub const CONTROL_PROTOCOL: u32 = 1;
pub const HOST_PROTOCOL: &str = "weyriva-luau-host/1";
pub const COMPATIBILITY_PROFILE: &str = "noctalia-v5-luau/1";
pub const MAX_LINE_BYTES: usize = 64 * 1024;
pub const MAX_FILES: usize = 512;
pub const MAX_TREE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub label: String,
    #[serde(default)]
    pub glyph: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub plugin_id: String,
    pub entry_id: String,
    pub entry: String,
    pub prefix: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub glyph: String,
    pub include_in_global_search: bool,
    pub debounce_ms: u64,
    #[serde(default)]
    pub categories: Vec<Category>,
}

impl Provider {
    #[must_use]
    pub fn reference(&self) -> String {
        format!("{}:{}", self.plugin_id, self.entry_id)
    }
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub root: PathBuf,
    pub provider: Provider,
    pub settings_defaults: BTreeMap<String, JsonValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserSource {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub builtin: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourcesDocument {
    pub schema: u32,
    pub sources: Vec<UserSource>,
}

impl Default for SourcesDocument {
    fn default() -> Self {
        Self {
            schema: SOURCES_SCHEMA,
            sources: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub kind: String,
    pub path: String,
    pub revision: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginRecord {
    pub id: String,
    pub installed: bool,
    pub enabled: bool,
    pub path: PathBuf,
    pub digest: String,
    pub version: String,
    pub provider: Provider,
    pub settings_defaults: BTreeMap<String, JsonValue>,
    pub provenance: Provenance,
    pub last_known_good: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateDocument {
    pub schema: u32,
    pub plugins: BTreeMap<String, PluginRecord>,
}

impl Default for StateDocument {
    fn default() -> Self {
        Self {
            schema: STATE_SCHEMA,
            plugins: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusRecord {
    #[serde(flatten)]
    pub record: PluginRecord,
    pub lifecycle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<crate::error::ErrorBody>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusResponse {
    pub schema: u32,
    pub plugins: Vec<StatusRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlRequest {
    pub protocol: u32,
    pub id: JsonValue,
    pub method: String,
    #[serde(default)]
    pub params: JsonValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostRequest {
    pub protocol: String,
    pub id: u64,
    pub method: String,
    pub params: JsonValue,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HostResponse {
    pub protocol: String,
    pub id: u64,
    pub result: Option<JsonValue>,
    pub error: Option<crate::error::ErrorBody>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HostEvent {
    pub protocol: String,
    pub event: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionOutcome {
    Clipboard { ok: bool },
    Notify { ok: bool },
    NotifyError { ok: bool },
    SetQuery { query: String, ok: bool },
}
