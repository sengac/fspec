//! # codelet-fspec-core
//!
//! Future home of the **Rust port** of the TypeScript `fspec` CLI commands
//! (tracked by RPC-003).
//!
//! Phase 1 — scoped by **TOOL-019** — ships only:
//!
//! * a canonical list of the 162 fspec command names (see [`canonical`])
//! * one stub command module per ported command, each returning
//!   [`error::FspecCoreError::NotYetPorted`] (Phase 1 only ships the single
//!   `add-rule` example; the remaining 161 are generated in a follow-up step)
//! * a synchronous [`dispatch::dispatch_command`] entry point that the
//!   standalone fspec Rust binary's `agent_loop` invokes for every Fspec tool
//!   call
//!
//! The motivation is to make sure the standalone fspec Rust binary's agent
//! loop **no longer hangs** on Fspec tool dispatch — instead of waiting for a
//! NAPI chunk-callback that does not exist, every dispatch returns a
//! structured per-command error the LLM can adapt to.
//!
//! Future phases replace each stub with the real Rust implementation under
//! its own child work unit of RPC-003.

pub mod canonical;
pub mod commands;
pub mod dispatch;
pub mod error;
pub mod foundation;
pub mod generators;
pub mod help;
mod help_dispatch;
mod help_dispatch_table;
pub mod io;
pub mod js_compat;
pub mod types;
pub mod update;
pub mod utils;
pub mod validators;
pub mod virtual_hooks_exec;

pub use dispatch::{dispatch_command, DispatchRequest, DispatchResult};
pub use error::FspecCoreError;
