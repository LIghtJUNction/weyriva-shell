mod client;
mod dispatch;
mod protocol;
mod server;
mod workers;

pub use client::call;
pub use dispatch::{BUILTIN_METHODS, Dispatcher};
pub use protocol::{MAX_CONTROL_RESPONSE_BYTES, process_line};
pub use server::{serve, serve_until};
pub use workers::MAX_ACTIVE_HANDLERS;
