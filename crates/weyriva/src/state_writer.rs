use std::path::Path;

use crate::model::StateDocument;
pub use crate::state_commit::{StateWriteError, StateWriteResult};
use crate::storage::atomic_json_commit;

/// Persists the complete plugin state document.
pub trait StateWriter: Send + Sync {
    /// Writes `state` atomically to `path`.
    ///
    /// # Errors
    ///
    /// Returns a typed persistence failure without changing runtime state.
    fn write(&self, path: &Path, state: &StateDocument) -> StateWriteResult<()>;
}

#[derive(Debug, Default)]
pub struct DurableStateWriter;

impl StateWriter for DurableStateWriter {
    fn write(&self, path: &Path, state: &StateDocument) -> StateWriteResult<()> {
        atomic_json_commit(path, state)
    }
}
