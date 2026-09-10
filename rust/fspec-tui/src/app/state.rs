//! `App` root (RPC-012, RPC-013 and successors).
//!
//! The struct + constructors live in [`state_types`]; the accessor +
//! run-loop surface in [`state_accessors`]; the TUI-106/109
//! navigator view-status seams in [`state_views`]. All store mutations
//! happen synchronously inside [`crate::app::dispatch`] on the App task
//! (RPC-009 single-task).

pub use state_types::App;

pub mod state_accessors;
pub mod state_types;
pub mod state_views;
