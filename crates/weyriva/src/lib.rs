//! Weyriva Plugins control-plane library.

#![forbid(unsafe_code)]

pub mod actions;
pub mod archive;
pub mod broker;
pub mod cli;
pub mod diagnose;
pub mod error;
pub mod host_session;
pub mod identity;
pub mod install;
pub mod ipc;
pub mod legacy_migration;
pub mod lock;
pub mod manifest;
pub mod model;
pub mod niri;
pub mod paths;
pub mod process;
mod runtime;
pub mod session;
pub mod shell;
pub mod sources;
pub mod startup;
mod state_commit;
pub mod state_writer;
pub mod storage;
mod transaction;
pub mod tree;
mod validation;

pub use broker::Broker;
pub use error::{Error, ErrorBody, Result};
pub use paths::Paths;
