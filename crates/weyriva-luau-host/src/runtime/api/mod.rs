mod state;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mlua::{Error as LuaError, Lua, LuaSerdeExt, Table, Value};
use serde_json::Value as JsonValue;

use super::value::json_to_lua;
use super::{Invocation, PendingLauncherModel};
use crate::config::{HostConfig, MAX_PLUGIN_DATA_BYTES, read_plugin_file};
use crate::error::{HostError, HostResult, from_lua};
use crate::model::{Action, MAX_QUERY_BYTES, check_string, push_action};
use state::create_state_api;

const MAX_STATE_KEY_BYTES: usize = 256;
const KNOWN_CALLBACK_GLOBALS: &[&str] = &["onQuery", "onActivate", "onIpc", "onExit"];

pub(super) fn install(
    lua: &Lua,
    config: &HostConfig,
    actions: Rc<RefCell<Vec<Action>>>,
    launcher_results: Rc<RefCell<Option<PendingLauncherModel>>>,
    invocation: Rc<Cell<Invocation>>,
) -> HostResult<()> {
    let noctalia = create_noctalia_api(lua, config, Rc::clone(&actions))?;
    let launcher = create_launcher_api(lua, actions, launcher_results, invocation)?;
    let globals = lua.globals();
    globals
        .set("noctalia", noctalia)
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    globals
        .set("launcher", launcher)
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_unsupported_index(lua, &globals, "global", KNOWN_CALLBACK_GLOBALS)
}

fn create_noctalia_api(
    lua: &Lua,
    config: &HostConfig,
    actions: Rc<RefCell<Vec<Action>>>,
) -> HostResult<Table> {
    let noctalia = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_core_api(lua, config, &noctalia)?;
    noctalia
        .set("string", create_string_api(lua)?)
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    noctalia
        .set("json", create_json_api(lua)?)
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    noctalia
        .set("state", create_state_api(lua)?)
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_clipboard_api(lua, &noctalia, Rc::clone(&actions))?;
    install_notification_api(
        lua,
        &noctalia,
        "notify",
        Rc::clone(&actions),
        Action::notify,
    )?;
    install_notification_api(lua, &noctalia, "notifyError", actions, Action::notify_error)?;
    install_unsupported_index(lua, &noctalia, "noctalia", &[])?;
    Ok(noctalia)
}

fn install_core_api(lua: &Lua, config: &HostConfig, noctalia: &Table) -> HostResult<()> {
    let settings = config.settings.clone();
    noctalia
        .set(
            "getConfig",
            lua.create_function(move |lua, key: String| {
                check_string("config key", &key, MAX_STATE_KEY_BYTES, false)
                    .map_err(LuaError::external)?;
                let value = settings.get(&key).cloned().unwrap_or(JsonValue::Null);
                json_to_lua(lua, &value)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    let plugin_dir_text = config.plugin_dir.to_string_lossy().into_owned();
    noctalia
        .set(
            "pluginDir",
            lua.create_function(move |_, ()| Ok(plugin_dir_text.clone()))?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    let read_root = config.plugin_dir.clone();
    noctalia
        .set(
            "readFile",
            lua.create_function(move |_, path: String| {
                read_plugin_file(&read_root, &path).map_err(LuaError::external)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    Ok(())
}

fn create_string_api(lua: &Lua) -> HostResult<Table> {
    let string_api = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    string_api
        .set(
            "trim",
            lua.create_function(|_, value: String| Ok(value.trim().to_owned()))?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_unsupported_index(lua, &string_api, "noctalia.string", &[])?;
    Ok(string_api)
}

fn create_json_api(lua: &Lua) -> HostResult<Table> {
    let json_api = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    json_api
        .set(
            "encode",
            lua.create_function(|lua, value: Value| {
                let value = lua.from_value::<JsonValue>(value)?;
                let encoded = serde_json::to_string(&value).map_err(LuaError::external)?;
                if encoded.len() > MAX_PLUGIN_DATA_BYTES {
                    return Err(LuaError::external(HostError::new(
                        "string_limit",
                        format!("encoded JSON exceeds {MAX_PLUGIN_DATA_BYTES} bytes"),
                    )));
                }
                Ok(encoded)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    json_api
        .set(
            "decode",
            lua.create_function(|lua, encoded: String| {
                if encoded.len() > MAX_PLUGIN_DATA_BYTES {
                    return Err(LuaError::external(HostError::new(
                        "string_limit",
                        format!("JSON input exceeds {MAX_PLUGIN_DATA_BYTES} bytes"),
                    )));
                }
                let value: JsonValue =
                    serde_json::from_str(&encoded).map_err(LuaError::external)?;
                json_to_lua(lua, &value)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_unsupported_index(lua, &json_api, "noctalia.json", &[])?;
    Ok(json_api)
}

fn create_launcher_api(
    lua: &Lua,
    actions: Rc<RefCell<Vec<Action>>>,
    launcher_results: Rc<RefCell<Option<PendingLauncherModel>>>,
    invocation: Rc<Cell<Invocation>>,
) -> HostResult<Table> {
    let launcher = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    let result_slot = launcher_results;
    let result_invocation = invocation;
    launcher
        .set(
            "setResults",
            lua.create_function(move |lua, (query, value): (String, Value)| {
                if result_invocation.get() != Invocation::Query {
                    return Err(LuaError::external(HostError::new(
                        "invalid_lifecycle",
                        "launcher.setResults is only valid during onQuery",
                    )));
                }
                check_string("launcher model query", &query, MAX_QUERY_BYTES, true)
                    .map_err(LuaError::external)?;
                let value = lua.from_value::<JsonValue>(value)?;
                *result_slot.borrow_mut() = Some(PendingLauncherModel {
                    query,
                    results: value,
                });
                Ok(())
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_string_action_api(
        lua,
        &launcher,
        "setQuery",
        actions,
        Action::launcher_set_query,
    )?;
    install_unsupported_index(lua, &launcher, "launcher", &[])?;
    Ok(launcher)
}

fn install_clipboard_api(
    lua: &Lua,
    table: &Table,
    actions: Rc<RefCell<Vec<Action>>>,
) -> HostResult<()> {
    table
        .set(
            "copyToClipboard",
            lua.create_function(move |_, (text, mime): (String, Option<String>)| {
                let action = Action::clipboard(text, mime).map_err(LuaError::external)?;
                push_action(&mut actions.borrow_mut(), action).map_err(LuaError::external)
            })
            .map_err(|error| from_lua(&error, "runtime_init"))?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))
}

fn install_notification_api(
    lua: &Lua,
    table: &Table,
    name: &str,
    actions: Rc<RefCell<Vec<Action>>>,
    create: fn(String, String) -> HostResult<Action>,
) -> HostResult<()> {
    table
        .set(
            name,
            lua.create_function(move |_, (title, message): (String, String)| {
                let action = create(title, message).map_err(LuaError::external)?;
                push_action(&mut actions.borrow_mut(), action).map_err(LuaError::external)
            })
            .map_err(|error| from_lua(&error, "runtime_init"))?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))
}

fn install_string_action_api(
    lua: &Lua,
    table: &Table,
    name: &str,
    actions: Rc<RefCell<Vec<Action>>>,
    create: fn(String) -> HostResult<Action>,
) -> HostResult<()> {
    table
        .set(
            name,
            lua.create_function(move |_, value: String| {
                let action = create(value).map_err(LuaError::external)?;
                push_action(&mut actions.borrow_mut(), action).map_err(LuaError::external)
            })
            .map_err(|error| from_lua(&error, "runtime_init"))?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))
}

fn install_unsupported_index(
    lua: &Lua,
    table: &Table,
    namespace: &str,
    known_absent: &'static [&'static str],
) -> HostResult<()> {
    let metatable = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    let namespace = namespace.to_owned();
    metatable
        .set(
            "__index",
            lua.create_function(move |_, (_table, key): (Table, Value)| {
                let key = match key {
                    Value::String(value) => value.to_string_lossy(),
                    value => value.type_name().to_owned(),
                };
                if known_absent.contains(&key.as_str()) {
                    return Ok(Value::Nil);
                }
                Err::<Value, _>(LuaError::external(HostError::new(
                    "unsupported_api",
                    format!("{namespace}.{key} is not available in the API 3 launcher subset"),
                )))
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    metatable
        .set("__metatable", "locked")
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    table
        .set_metatable(Some(metatable))
        .map_err(|error| from_lua(&error, "runtime_init"))
}
