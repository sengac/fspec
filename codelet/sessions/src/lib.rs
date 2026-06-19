//! `codelet-sessions` - the NAPI-free session-manager crate.
//!
//! Hosts `SessionManager` and `BackgroundSession`, the agent loop, and the
//! tokio broadcast wiring that replaces NAPI's `GLOBAL_CHUNK_CALLBACK`.
//!
//! ## Layout
//!
//! * [`background_session`] - populated by RPC-039.
//! * [`chain_of_command`] - lifted from napi by RPC-040.
//! * [`credentials`] - lifted from napi by RPC-040.
//! * [`navigation`] - lifted from napi by RPC-040.
//! * [`session_manager`] - populated by RPC-040 with `SessionManager`,
//!   the `SessionManagerHooks` trait, and `NoopSessionManagerHooks`.

pub mod background_session;
pub mod chain_of_command;
pub mod conversions;
pub mod credentials;
pub mod handle_impl;
pub mod model_resolution;
pub mod navigation;
pub mod profile_sections;
pub mod session_manager;

// Convenient re-exports so downstream crates can write
// `use codelet_sessions::SessionManagerHooks;` instead of the longer
// `use codelet_sessions::session_manager::SessionManagerHooks;`.
pub use chain_of_command::ChainOfCommand;
pub use session_manager::{NoopSessionManagerHooks, SessionManager, SessionManagerHooks};
