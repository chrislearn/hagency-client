//! Scoped Hagency Agent Operations client. Secrets stay in the worker's memory.
pub mod protocol;
mod contract;
pub mod backend;
pub mod ui;

pub const SOURCE_COMMIT: &str = "4a8a8ac25e43e645345fc25987261e0d7a644fcd";
pub const SCHEMA: &str = "com.hagency.agent_ops.v1";
pub const EVENT_KEY: &str = "com.hagency.agent_ops";

pub fn available() -> bool {
    // The producer has not released this contract. Tests/development explicitly
    // opt into the pinned artifact set; normal builds cannot bootstrap a session.
    cfg!(feature = "agent_ops_dev")
}

#[cfg(test)]
mod interop_test;
