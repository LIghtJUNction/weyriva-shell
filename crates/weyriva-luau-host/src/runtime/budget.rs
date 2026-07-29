use std::cell::Cell;
use std::time::{Duration, Instant};

use mlua::{Error as LuaError, VmState};

use crate::error::HostError;

pub(super) const MAX_MEMORY_BYTES: usize = 64 * 1_024 * 1_024;
const MAX_INTERRUPT_TICKS: u64 = 20_000;
const MAX_EXECUTION_TIME: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(super) struct ExecutionBudget {
    active: Cell<bool>,
    deadline: Cell<Option<Instant>>,
    ticks: Cell<u64>,
}

impl ExecutionBudget {
    pub(super) fn new() -> Self {
        Self {
            active: Cell::new(false),
            deadline: Cell::new(None),
            ticks: Cell::new(0),
        }
    }

    pub(super) fn begin(&self) {
        self.ticks.set(0);
        self.deadline.set(Some(Instant::now() + MAX_EXECUTION_TIME));
        self.active.set(true);
    }

    pub(super) fn end(&self) {
        self.active.set(false);
        self.deadline.set(None);
    }

    pub(super) fn interrupt(&self) -> mlua::Result<VmState> {
        if !self.active.get() {
            return Ok(VmState::Continue);
        }
        let ticks = self.ticks.get().saturating_add(1);
        self.ticks.set(ticks);
        let expired = self
            .deadline
            .get()
            .is_some_and(|deadline| Instant::now() >= deadline);
        if ticks > MAX_INTERRUPT_TICKS || expired {
            return Err(LuaError::external(HostError::new(
                "execution_limit",
                "Luau callback exceeded its execution budget",
            )));
        }
        Ok(VmState::Continue)
    }
}
