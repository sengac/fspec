//! Command history tracking — RPC-025 lift.
//!
//! The implementation now lives in
//! `codelet_core::persistence::history`. This module exists as a thin
//! re-export shim so the historical `codelet/napi/src/persistence::history::HistoryStore`
//! import path keeps working for any internal NAPI consumer.
//!
//! The on-disk JSONL file at
//! `codelet_common::get_data_dir().join("history.jsonl")` is the single
//! source of truth shared by the TS Ink TUI (via the existing
//! `persistence_*_history` NAPI exports) and the Rust ratatui TUI (via
//! the new `FspecService::persistence_*_history` RPC methods).

pub use codelet_core::persistence::history::HistoryStore;
pub use codelet_core::persistence::HistoryEntry;
