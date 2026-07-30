use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use mlua::{Function, Lua, LuaOptions, LuaSerdeExt, StdLib, Value};
use serde_json::Value as JsonValue;

use crate::config::{HostConfig, read_plugin_file};
use crate::error::{HostError, HostResult, from_lua};
use crate::model::{Action, LauncherModel, MAX_QUERY_BYTES, check_string, push_action};

mod api;
mod budget;
mod value;

use api::{SharedState, StateWatchers};
use budget::{ExecutionBudget, MAX_MEMORY_BYTES};
use value::json_to_lua;

const MAX_IPC_EVENT_BYTES: usize = 256;
const MAX_STATE_NOTIFICATIONS: usize = 256;

#[derive(Debug)]
pub struct InvocationResult<T> {
    pub value: T,
    pub actions: Vec<Action>,
}

pub struct Host {
    launcher_id: String,
    launcher: EntryVm,
    service: Option<ServiceVm>,
    shared_state: SharedState,
    startup_actions: RefCell<Vec<Action>>,
}

struct ServiceVm {
    id: String,
    vm: EntryVm,
}

struct EntryVm {
    lua: Lua,
    budget: Rc<ExecutionBudget>,
    actions: Rc<RefCell<Vec<Action>>>,
    launcher_results: Rc<RefCell<Option<PendingLauncherModel>>>,
    invocation: Rc<Cell<Invocation>>,
    state_watchers: StateWatchers,
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
    /// Creates isolated launcher and optional service VMs for one plugin process.
    ///
    /// Both VMs share only copied JSON state. The optional service is loaded
    /// after the launcher and its required `update` callback is invoked exactly
    /// once before the host becomes ready.
    ///
    /// # Errors
    ///
    /// Returns an error when either VM cannot be initialized or loaded, or when
    /// the service's initial update fails.
    pub fn new(config: &HostConfig) -> HostResult<Self> {
        let shared_state = SharedState::default();
        let launcher = EntryVm::new(config, &config.entry_path, &shared_state)?;
        let service = config
            .service
            .as_ref()
            .map(|service| -> HostResult<ServiceVm> {
                Ok(ServiceVm {
                    id: service.id.clone(),
                    vm: EntryVm::new(config, &service.entry_path, &shared_state)?,
                })
            })
            .transpose()?;
        let host = Self {
            launcher_id: config.entry_id.clone(),
            launcher,
            service,
            shared_state,
            startup_actions: RefCell::new(Vec::new()),
        };
        host.flush_state_changes()?;
        let mut startup_actions = host.finish_invocation(())?.actions;
        if let Some(service) = &host.service {
            let startup = host.update(&service.id)?;
            for action in startup.actions {
                push_action(&mut startup_actions, action)?;
            }
        }
        *host.startup_actions.borrow_mut() = startup_actions;
        Ok(host)
    }

    #[must_use]
    pub fn launcher_id(&self) -> &str {
        &self.launcher_id
    }

    #[must_use]
    pub fn service_id(&self) -> Option<&str> {
        self.service.as_ref().map(|service| service.id.as_str())
    }

    pub fn take_startup_actions(&self) -> Vec<Action> {
        self.startup_actions.borrow_mut().drain(..).collect()
    }

    /// Invokes `onQuery` and validates the declarative launcher model.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input, callback or propagation failures, or
    /// an invalid launcher result model.
    pub fn query(&self, query: String) -> HostResult<InvocationResult<LauncherModel>> {
        check_string("query", &query, MAX_QUERY_BYTES, true)?;
        self.begin_invocation(Invocation::Query);
        let result = self
            .launcher
            .required_callback("onQuery")
            .and_then(|function| self.launcher.execute(|| function.call::<()>(query.clone())))
            .and_then(|()| self.flush_state_changes());
        self.launcher.invocation.set(Invocation::Idle);
        if let Err(error) = result {
            return self.fail_invocation(error);
        }
        let pending = self
            .launcher
            .launcher_results
            .borrow_mut()
            .take()
            .unwrap_or_else(|| PendingLauncherModel {
                query,
                results: JsonValue::Array(Vec::new()),
            });
        let model = LauncherModel::from_json(pending.query, pending.results)?;
        self.finish_invocation(model)
    }

    /// Invokes `onActivate` and returns bounded side effects.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input, a missing or failing callback, state
    /// propagation failure, or action exhaustion.
    pub fn activate(&self, id: String) -> HostResult<InvocationResult<()>> {
        check_string("launcher result id", &id, 256, false)?;
        self.begin_invocation(Invocation::Idle);
        let result = self
            .launcher
            .required_callback("onActivate")
            .and_then(|function| self.launcher.execute(|| function.call::<()>(id)))
            .and_then(|()| self.flush_state_changes());
        match result {
            Ok(()) => self.finish_invocation(()),
            Err(error) => self.fail_invocation(error),
        }
    }

    /// Invokes the optional service's required `update` callback.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry does not identify the configured service,
    /// the callback is missing or fails, or state/action bounds are exceeded.
    pub fn update(&self, entry: &str) -> HostResult<InvocationResult<()>> {
        let service = self.service_for(entry)?;
        self.begin_invocation(Invocation::Idle);
        let result = service
            .vm
            .required_callback("update")
            .and_then(|function| service.vm.execute(|| function.call::<()>(())))
            .and_then(|()| self.flush_state_changes());
        match result {
            Ok(()) => self.finish_invocation(()),
            Err(error) => self.fail_invocation(error),
        }
    }

    /// Invokes `onIpc` on the explicitly addressed entry.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown entry, invalid input, callback failure,
    /// or a result that is not bounded plain JSON data.
    pub fn ipc(
        &self,
        entry: Option<&str>,
        event: String,
        payload: &JsonValue,
    ) -> HostResult<InvocationResult<JsonValue>> {
        check_string("IPC event", &event, MAX_IPC_EVENT_BYTES, false)?;
        let vm = self.entry_for(entry)?;
        self.begin_invocation(Invocation::Idle);
        let payload =
            json_to_lua(&vm.lua, payload).map_err(|error| from_lua(&error, "invalid_params"))?;
        let result = vm
            .required_callback("onIpc")
            .and_then(|function| vm.execute(|| function.call::<Value>((event, payload))))
            .and_then(|value| {
                vm.lua
                    .from_value::<JsonValue>(value)
                    .map_err(|error| from_lua(&error, "invalid_result"))
            })
            .and_then(|value| {
                self.flush_state_changes()?;
                Ok(value)
            });
        match result {
            Ok(value) => self.finish_invocation(value),
            Err(error) => self.fail_invocation(error),
        }
    }

    /// Invokes optional exit callbacks in both VMs.
    ///
    /// # Errors
    ///
    /// Returns an error if either callback fails or a bound is exceeded.
    pub fn shutdown(&self) -> HostResult<InvocationResult<bool>> {
        self.begin_invocation(Invocation::Idle);
        let mut called = false;
        for vm in self.entries() {
            let callback = vm.optional_callback("onExit")?;
            called |= callback.is_some();
            if let Some(function) = callback
                && let Err(error) = vm.execute(|| function.call::<()>((0_i64, "shutdown")))
            {
                return self.fail_invocation(error);
            }
        }
        if let Err(error) = self.flush_state_changes() {
            return self.fail_invocation(error);
        }
        self.finish_invocation(called)
    }

    fn service_for(&self, entry: &str) -> HostResult<&ServiceVm> {
        self.service
            .as_ref()
            .filter(|service| service.id == entry)
            .ok_or_else(|| {
                HostError::new(
                    "unknown_entry",
                    format!("entry `{entry}` is not the configured service"),
                )
            })
    }

    fn entry_for(&self, entry: Option<&str>) -> HostResult<&EntryVm> {
        let entry = entry.unwrap_or(&self.launcher_id);
        if entry == self.launcher_id {
            return Ok(&self.launcher);
        }
        self.service_for(entry).map(|service| &service.vm)
    }

    fn entries(&self) -> impl Iterator<Item = &EntryVm> {
        std::iter::once(&self.launcher).chain(self.service.as_ref().map(|service| &service.vm))
    }

    fn begin_invocation(&self, launcher_invocation: Invocation) {
        self.shared_state.clear_changes();
        for vm in self.entries() {
            vm.clear_invocation();
        }
        self.launcher.invocation.set(launcher_invocation);
    }

    fn flush_state_changes(&self) -> HostResult<()> {
        let mut notifications = 0_usize;
        while let Some((source_vm_id, key, value)) = self.shared_state.pop_change() {
            notifications = notifications.saturating_add(1);
            if notifications > MAX_STATE_NOTIFICATIONS {
                return Err(HostError::new(
                    "state_limit",
                    format!("state notifications exceed the limit of {MAX_STATE_NOTIFICATIONS}"),
                ));
            }
            for vm in self.entries() {
                if vm.state_watchers.vm_id() == source_vm_id {
                    continue;
                }
                vm.execute(|| vm.state_watchers.notify(&vm.lua, &key, &value))?;
            }
        }
        Ok(())
    }

    fn finish_invocation<T>(&self, value: T) -> HostResult<InvocationResult<T>> {
        self.launcher.invocation.set(Invocation::Idle);
        let mut actions = Vec::new();
        for vm in self.entries() {
            for action in vm.actions.borrow_mut().drain(..) {
                push_action(&mut actions, action)?;
            }
        }
        Ok(InvocationResult { value, actions })
    }

    fn fail_invocation<T>(&self, error: HostError) -> HostResult<T> {
        self.shared_state.clear_changes();
        for vm in self.entries() {
            vm.clear_invocation();
        }
        Err(error)
    }
}

impl EntryVm {
    fn new(config: &HostConfig, entry_path: &Path, shared_state: &SharedState) -> HostResult<Self> {
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
        let state_watchers = api::install(
            &lua,
            config,
            Rc::clone(&actions),
            Rc::clone(&launcher_results),
            Rc::clone(&invocation),
            shared_state,
        )?;
        lua.sandbox(true)
            .map_err(|error| from_lua(&error, "runtime_init"))?;

        let source = read_entry_source(config, entry_path)?;
        let vm = Self {
            lua,
            budget,
            actions,
            launcher_results,
            invocation,
            state_watchers,
        };
        vm.execute(|| {
            vm.lua
                .load(&source)
                .set_name(format!("@{}", entry_path.display()))
                .exec()
        })
        .map_err(|error| {
            if error.code() == "callback_error" {
                HostError::new("plugin_load", error.to_string())
            } else {
                error
            }
        })?;
        vm.clear_invocation();
        Ok(vm)
    }

    fn clear_invocation(&self) {
        self.invocation.set(Invocation::Idle);
        self.actions.borrow_mut().clear();
        self.launcher_results.borrow_mut().take();
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

fn read_entry_source(config: &HostConfig, entry_path: &Path) -> HostResult<String> {
    let relative = entry_path
        .strip_prefix(&config.plugin_dir)
        .map_err(|_| HostError::new("invalid_entry", "entry escaped plugin root"))?
        .to_str()
        .ok_or_else(|| HostError::new("invalid_entry", "entry path must be valid UTF-8"))?;
    read_plugin_file(&config.plugin_dir, relative)
}
