//! RPC-025: per-session history-recall navigation state.
//!
//! Lives in its own sub-module under `store/agent_view/` so
//! `agent_view.rs` stays under the 300-LoC ceiling pinned by
//! `rpc025-source-shape.feature`. Accessor methods live in
//! `agent_view.rs` alongside the state fields they touch.

/// Per-session Shift+↑/↓ recall state. `recall_index == None` means the
/// user is on the live MultiLineInput draft (no history is shown);
/// `recall_index == Some(k)` means history\[k\] is currently shown (0 =
/// most recent submitted input). `cached_draft` is the saved live
/// draft, restored when the user walks back past index 0 via Shift+↓.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryNavState {
    pub recall_index: Option<usize>,
    pub cached_draft: Option<String>,
}
