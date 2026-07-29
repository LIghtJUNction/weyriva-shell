use mlua::{Lua, LuaSerdeExt, Value, serde::SerializeOptions};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::error::{HostError, HostResult};

const MAX_STATE_VALUE_BYTES: usize = 16 * 1_024;

pub(super) fn validate_json_size(label: &str, value: &impl Serialize) -> HostResult<()> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        HostError::new("invalid_value", format!("cannot encode {label}: {error}"))
    })?;
    if encoded.len() > MAX_STATE_VALUE_BYTES {
        return Err(HostError::new(
            "value_limit",
            format!("{label} exceeds {MAX_STATE_VALUE_BYTES} encoded bytes"),
        ));
    }
    Ok(())
}

pub(super) fn json_to_lua(lua: &Lua, value: &JsonValue) -> mlua::Result<Value> {
    lua.to_value_with(
        value,
        SerializeOptions::new()
            .serialize_none_to_null(false)
            .serialize_unit_to_null(false),
    )
}
