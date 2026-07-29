use std::fs;

use serde_json::Value as JsonValue;

use crate::error::{Error, Result};
use crate::manifest::{valid_identifier, valid_plugin_id};
use crate::model::PluginRecord;
use crate::paths::Paths;

pub(crate) fn parse_reference(reference: &str) -> Result<(&str, &str)> {
    let (plugin_id, entry_id) = reference
        .rsplit_once(':')
        .ok_or_else(|| Error::new("invalid_provider", "provider must be ID:ENTRY"))?;
    if !valid_plugin_id(plugin_id) || !valid_identifier(entry_id) {
        return Err(Error::new("invalid_provider", "provider must be ID:ENTRY"));
    }
    Ok((plugin_id, entry_id))
}

pub(crate) fn validate_uninstall_path(
    paths: &Paths,
    plugin_id: &str,
    record: &PluginRecord,
) -> Result<()> {
    if record.digest.len() != 64 || !record.digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::new("unsafe_state", "recorded digest is invalid"));
    }
    let installed = paths.data_dir.join("installed");
    let expected_parent = installed.join(plugin_id);
    if record.path.parent() != Some(expected_parent.as_path())
        || record.path.file_name().and_then(|name| name.to_str()) != Some(&record.digest)
    {
        return Err(Error::new(
            "unsafe_state",
            "installed path is not the recorded immutable version",
        ));
    }
    let metadata = fs::symlink_metadata(&record.path)
        .map_err(|error| Error::io("cannot inspect installed plugin", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "unsafe_state",
            "installed plugin path is unsafe",
        ));
    }
    Ok(())
}

pub(crate) fn validate_query(result: &JsonValue, query: &str) -> Result<()> {
    let object = result
        .as_object()
        .ok_or_else(|| Error::new("host_protocol", "query result is not an object"))?;
    if object.get("query").and_then(JsonValue::as_str) != Some(query) {
        return Err(Error::new(
            "host_protocol",
            "query result does not match request",
        ));
    }
    let results = object
        .get("results")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| Error::new("host_protocol", "query results are invalid"))?;
    if results.len() > 2_000 {
        return Err(Error::new("host_protocol", "query result limit exceeded"));
    }
    let mut ids = std::collections::BTreeSet::new();
    for item in results {
        let item = item
            .as_object()
            .ok_or_else(|| Error::new("host_protocol", "launcher result is not an object"))?;
        if item.keys().any(|field| {
            !matches!(
                field.as_str(),
                "id" | "title" | "subtitle" | "glyph" | "category"
            )
        }) {
            return Err(Error::new(
                "host_protocol",
                "launcher result has unknown fields",
            ));
        }
        let id = bounded_string(item.get("id"), 256, false)?;
        bounded_string(item.get("title"), 512, false)?;
        for (field, limit) in [("subtitle", 1_024), ("glyph", 128), ("category", 256)] {
            if let Some(value) = item.get(field) {
                bounded_string(Some(value), limit, true)?;
            }
        }
        if !ids.insert(id) {
            return Err(Error::new(
                "host_protocol",
                "launcher result ids are not unique",
            ));
        }
    }
    Ok(())
}

fn bounded_string(value: Option<&JsonValue>, limit: usize, allow_empty: bool) -> Result<String> {
    let value = value
        .and_then(JsonValue::as_str)
        .ok_or_else(|| Error::new("host_protocol", "launcher result field is not a string"))?;
    if (!allow_empty && value.is_empty()) || value.len() > limit {
        return Err(Error::new(
            "host_protocol",
            "launcher result string is invalid",
        ));
    }
    Ok(value.to_owned())
}
