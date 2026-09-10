//! RPC-025 per-session history accessors on [`AgentViewStore`].
//!
//! Extracted from `agent_view.rs` to keep that file under the 300-LoC
//! ceiling pinned by `source_shape_stores_rpc012`.

use codelet_rpc_types::SessionId;

use super::history_state::HistoryNavState;
use super::AgentViewStore;

impl AgentViewStore {
    // ── RPC-025 per-session history accessors ───────────────────────────

    /// Borrow the current HistoryNavState for `session`, if any.
    pub fn history_state_for(&self, session: &SessionId) -> Option<&HistoryNavState> {
        self.history_state_by_session.get(session)
    }

    /// Mutable accessor — inserts a default state when missing.
    pub fn history_state_for_mut(&mut self, session: &SessionId) -> &mut HistoryNavState {
        self.history_state_by_session
            .entry(session.clone())
            .or_default()
    }

    pub fn cached_history_snapshot(&self, session: &SessionId) -> Option<&Vec<String>> {
        self.cached_history_snapshot.get(session)
    }

    pub fn set_history_snapshot(&mut self, session: SessionId, snapshot: Vec<String>) {
        self.cached_history_snapshot.insert(session, snapshot);
    }

    pub fn reset_history_state(&mut self, session: &SessionId) {
        self.history_state_by_session
            .insert(session.clone(), HistoryNavState::default());
        self.cached_history_snapshot.remove(session);
    }
}
