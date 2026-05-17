//! UI-side state ownership added in RPC-012.
//!
//! Feature: spec/features/rpc012-board-store.feature
//!
//! Two plain owned Rust structs — `BoardStore` and `AgentViewStore` —
//! are the Rust analogues of TS `useFspecStore` + `useSessionStore`.
//! They live on the [`crate::app::App`] alongside the Compositor and
//! action bus and are mutated ONLY on the App task (RPC-009 single-task
//! tenere pattern). No `Mutex` / `RwLock` / atomic types appear in any
//! field of either store.

pub mod agent_view;
pub mod board;
mod board_viewport;

pub use agent_view::{AgentViewStore, SessionContext, TokenState};
pub use board::{column_index, BoardStore, COLUMN_ORDER};
