//! Sequential, observable execution with headless conversational sessions and optional local idle snapshots.

pub mod agent;
pub mod auth;
pub mod cancellation;
pub mod changes;
pub mod context;
pub mod events;
pub mod file_lookup;
pub mod model;
pub mod native;
pub mod observability;
pub mod openai;
pub mod output;
pub mod permissions;
pub mod runtime;
pub mod session_store;
pub mod tools;
pub mod workspace;
