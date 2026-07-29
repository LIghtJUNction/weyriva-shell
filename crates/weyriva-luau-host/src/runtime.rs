use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mlua::{Function, Lua, LuaOptions, LuaSerdeExt, StdLib, Value};
use serde_json::Value as JsonValue;

use crate::config::{HostConfig, read_plugin_file};
use crate::error::{HostError, HostResult, from_lua};
use crate::model::{Action, LauncherModel, MAX_QUERY_BYTES, check_string};

mod api;
mod budget;
mod value;

use budget::{ExecutionBudget, MAX_MEMORY_BYTES};
use value::json_to_lua;

const MAX_IPC_EVENT_BYTES: usize = 256;

#[derive(Debug)]
pub struct InvocationResult<T> {
    pub value: T,
    pub actions: Vec<Action>,
}

pub struct Host {
    lua: Lua,
    budget: Rc<ExecutionBudget>,
    actions: Rc<RefCell<Vec<Action>>>,
    launcher_results: Rc<RefCell<Option<PendingLauncherModel>>>,
    invocation: Rc<Cell<Invocation>>,
}

#[derive(Debug)]
struct PendingLauncherModel {
    query: String,
    results: JsonValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Invocation {
    Idle,
    Query,
}

impl Host {
    /// Creates one sandboxed Luau VM and executes the configured entry once.
    ///
    /// # Errors
    ///
    /// Returns an error when the VM cannot be initialized, the entry cannot be
    /// read, sandbox limits cannot be installed, or top-level plugin code fails.
    pub fn new(config: &HostConfig) -> HostResult<Self> {
        let libraries = StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8 | StdLib::BIT;
        let lua = Lua::new_with(libraries, LuaOptions::default())
            .map_err(|error| from_lua(&error, "runtime_init"))?;
        lua.set_memory_limit(MAX_MEMORY_BYTES)
            .map_err(|error| from_lua(&error, "runtime_init"))?;

        let budget = Rc::new(ExecutionBudget::new());
        let interrupt_budget = Rc::clone(&budget);
        lua.set_interrupt(move |_| interrupt_budget.interrupt());

        let actions = Rc::new(RefCell::new(Vec::new()));
        let launcher_results = Rc::new(RefCell::new(None));
        let invocation = Rc::new(Cell::new(Invocation::Idle));
        api::install(
            &lua,
            config,
            Rc::clone(&actions),
            Rc::clone(&launcher_results),
            Rc::clone(&invocation),
        )?;
        lua.sandbox(true)
            .map_err(|error| from_lua(&error, "runtime_init"))?;

        let source = read_plugin_file(
            &config.plugin_dir,
            config
                .entry_path
                .strip_prefix(&config.plugin_dir)
                .map_err(|_| HostError::new("invalid_entry", "entry escaped plugin root"))?
                .to_str()
                .ok_or_else(|| HostError::new("invalid_entry", "entry path must be valid UTF-8"))?,
        )?;

        let host = Self {
            lua,
            budget,
            actions,
            launcher_results,
            invocation,
        };
        host.execute(|| {
            host.lua
                .load(&source)
                .set_name(format!("@{}", config.entry_path.display()))
                .exec()
        })
        .map_err(|error| {
            if error.code() == "callback_error" {
                HostError::new("plugin_load", error.to_string())
            } else {
                error
            }
        })?;
        host.actions.borrow_mut().clear();
        Ok(host)
    }

    /// Invokes `onQuery` and validates the declarative launcher model.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid query, missing callback, execution
    /// failure, resource limit, or invalid launcher result model.
    pub fn query(&self, query: String) -> HostResult<InvocationResult<LauncherModel>> {
        check_string("query", &query, MAX_QUERY_BYTES, true)?;
        self.begin_invocation(Invocation::Query);
        let result = self
            .required_callback("onQuery")
            .and_then(|function| self.execute(|| function.call::<()>(query.clone())));
        self.invocation.set(Invocation::Idle);
        if let Err(error) = result {
            return self.fail_invocation(error);
        }
        let pending = self
            .launcher_results
            .borrow_mut()
            .take()
            .unwrap_or_else(|| PendingLauncherModel {
                query,
                results: JsonValue::Array(Vec::new()),
            });
        let model = LauncherModel::from_json(pending.query, pending.results)?;
        Ok(self.finish_invocation(model))
    }

    /// Invokes `onActivate` and returns queued side effects as actions.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid result identifier, missing callback,
    /// execution failure, or resource/action limit.
    pub fn activate(&self, id: String) -> HostResult<InvocationResult<()>> {
        check_string("launcher result id", &id, 256, false)?;
        self.begin_invocation(Invocation::Idle);
        let result = self
            .required_callback("onActivate")
            .and_then(|function| self.execute(|| function.call::<()>(id)));
        if let Err(error) = result {
            return self.fail_invocation(error);
        }
        Ok(self.finish_invocation(()))
    }

    /// Invokes `onIpc` with a copied plain-data payload.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input, a missing callback, execution
    /// failure, or a callback result that is not bounded plain data.
    pub fn ipc(
        &self,
        event: String,
        payload: &JsonValue,
    ) -> HostResult<InvocationResult<JsonValue>> {
        check_string("IPC event", &event, MAX_IPC_EVENT_BYTES, false)?;
        self.begin_invocation(Invocation::Idle);
        let payload =
            json_to_lua(&self.lua, payload).map_err(|error| from_lua(&error, "invalid_params"))?;
        let result = self
            .required_callback("onIpc")
            .and_then(|function| self.execute(|| function.call::<Value>((event, payload))));
        let result = match result {
            Ok(value) => self
                .lua
                .from_value::<JsonValue>(value)
                .map_err(|error| from_lua(&error, "invalid_result")),
            Err(error) => Err(error),
        };
        match result {
            Ok(value) => Ok(self.finish_invocation(value)),
            Err(error) => self.fail_invocation(error),
        }
    }

    /// Invokes the optional exit callback for a graceful protocol shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the callback exists but fails or exceeds a limit.
    pub fn shutdown(&self) -> HostResult<InvocationResult<bool>> {
        self.begin_invocation(Invocation::Idle);
        let callback = self.optional_callback("onExit")?;
        let called = callback.is_some();
        if let Some(function) = callback
            && let Err(error) = self.execute(|| function.call::<()>((0_i64, "shutdown")))
        {
            return self.fail_invocation(error);
        }
        Ok(self.finish_invocation(called))
    }

    fn begin_invocation(&self, invocation: Invocation) {
        self.actions.borrow_mut().clear();
        self.launcher_results.borrow_mut().take();
        self.invocation.set(invocation);
    }

    fn finish_invocation<T>(&self, value: T) -> InvocationResult<T> {
        self.invocation.set(Invocation::Idle);
        InvocationResult {
            value,
            actions: self.actions.borrow_mut().drain(..).collect(),
        }
    }

    fn fail_invocation<T>(&self, error: HostError) -> HostResult<T> {
        self.invocation.set(Invocation::Idle);
        self.actions.borrow_mut().clear();
        self.launcher_results.borrow_mut().take();
        Err(error)
    }

    fn execute<T>(&self, operation: impl FnOnce() -> mlua::Result<T>) -> HostResult<T> {
        self.budget.begin();
        let result = operation();
        self.budget.end();
        result.map_err(|error| from_lua(&error, "callback_error"))
    }

    fn required_callback(&self, name: &str) -> HostResult<Function> {
        self.optional_callback(name)?.ok_or_else(|| {
            HostError::new(
                "missing_callback",
                format!("plugin does not define required callback `{name}`"),
            )
        })
    }

    fn optional_callback(&self, name: &str) -> HostResult<Option<Function>> {
        match self
            .lua
            .globals()
            .get::<Value>(name)
            .map_err(|error| from_lua(&error, "callback_error"))?
        {
            Value::Nil => Ok(None),
            Value::Function(function) => Ok(Some(function)),
            value => Err(HostError::new(
                "invalid_callback",
                format!(
                    "global `{name}` must be a function, found {}",
                    value.type_name()
                ),
            )),
        }
    }
}
