use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use mlua::{Error as LuaError, Function, Lua, LuaSerdeExt, Table, Value};
use serde_json::Value as JsonValue;

use super::{MAX_STATE_KEY_BYTES, install_unsupported_index};
use crate::error::{HostResult, from_lua};
use crate::model::check_string;
use crate::runtime::value::{json_to_lua, validate_json_size};

#[derive(Clone, Default)]
pub(in crate::runtime) struct SharedState {
    values: Rc<RefCell<HashMap<String, JsonValue>>>,
    pending: Rc<RefCell<VecDeque<StateChange>>>,
    next_vm_id: Rc<Cell<u64>>,
}

#[derive(Clone)]
struct StateChange {
    source_vm_id: u64,
    key: String,
    value: JsonValue,
}

pub(in crate::runtime) struct StateWatchers {
    vm_id: u64,
    callbacks: Table,
}

impl SharedState {
    pub(in crate::runtime) fn pop_change(&self) -> Option<(u64, String, JsonValue)> {
        self.pending
            .borrow_mut()
            .pop_front()
            .map(|change| (change.source_vm_id, change.key, change.value))
    }

    pub(in crate::runtime) fn clear_changes(&self) {
        self.pending.borrow_mut().clear();
    }
}

impl StateWatchers {
    pub(in crate::runtime) const fn vm_id(&self) -> u64 {
        self.vm_id
    }

    pub(in crate::runtime) fn notify(
        &self,
        lua: &Lua,
        key: &str,
        value: &JsonValue,
    ) -> mlua::Result<()> {
        if let Some(callbacks) = self.callbacks.get::<Option<Table>>(key)? {
            for callback in callbacks.sequence_values::<Function>() {
                callback?.call::<()>(json_to_lua(lua, value)?)?;
            }
        }
        Ok(())
    }
}

pub(super) fn create_state_api(
    lua: &Lua,
    shared: &SharedState,
) -> HostResult<(Table, StateWatchers)> {
    let vm_id = shared.next_vm_id.get();
    shared.next_vm_id.set(vm_id.saturating_add(1));
    let state_watchers = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    let state_api = lua
        .create_table()
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    let get_values = Rc::clone(&shared.values);
    state_api
        .set(
            "get",
            lua.create_function(move |lua, key: String| {
                validate_state_key(&key).map_err(LuaError::external)?;
                let value = get_values
                    .borrow()
                    .get(&key)
                    .cloned()
                    .unwrap_or(JsonValue::Null);
                json_to_lua(lua, &value)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    let set_values = Rc::clone(&shared.values);
    let pending = Rc::clone(&shared.pending);
    let set_watchers = state_watchers.clone();
    state_api
        .set(
            "set",
            lua.create_function(move |lua, (key, value): (String, Value)| {
                validate_state_key(&key).map_err(LuaError::external)?;
                let value = lua.from_value::<JsonValue>(value)?;
                validate_json_size("state value", &value).map_err(LuaError::external)?;
                set_values.borrow_mut().insert(key.clone(), value.clone());
                if let Some(callbacks) = set_watchers.get::<Option<Table>>(key.clone())? {
                    for callback in callbacks.sequence_values::<Function>() {
                        callback?.call::<()>(json_to_lua(lua, &value)?)?;
                    }
                }
                pending.borrow_mut().push_back(StateChange {
                    source_vm_id: vm_id,
                    key,
                    value,
                });
                Ok(())
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;

    let watch_watchers = state_watchers.clone();
    state_api
        .set(
            "watch",
            lua.create_function(move |lua, (key, callback): (String, Function)| {
                validate_state_key(&key).map_err(LuaError::external)?;
                let callbacks =
                    if let Some(callbacks) = watch_watchers.get::<Option<Table>>(key.clone())? {
                        callbacks
                    } else {
                        let callbacks = lua.create_table()?;
                        watch_watchers.set(key, callbacks.clone())?;
                        callbacks
                    };
                let index = callbacks.raw_len().saturating_add(1);
                callbacks.raw_set(index, callback)
            })?,
        )
        .map_err(|error| from_lua(&error, "runtime_init"))?;
    install_unsupported_index(lua, &state_api, "noctalia.state", &[])?;
    Ok((
        state_api,
        StateWatchers {
            vm_id,
            callbacks: state_watchers,
        },
    ))
}

fn validate_state_key(key: &str) -> HostResult<()> {
    check_string("state key", key, MAX_STATE_KEY_BYTES, false)
}
