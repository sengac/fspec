//! BUG-163 — id-keyed session indexing for the mux agent panes.
//!
//! Feature: spec/features/mux-agent-panes-render-distinct-window-sessions.feature
//!
//! Lives in its own sibling module so `agent_view.rs` stays under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature` /
//! `rpc025-source-shape.feature` (same pattern as `navigation.rs` for
//! RPC-096).

use super::AgentViewStore;
use codelet_rpc_types::SessionId;

impl AgentViewStore {
    /// BUG-163: the `(current, total)` 1-based index of the session with
    /// the given id, or `(0, 0)` when the session is not open. Mirrors
    /// [`Self::session_index`] but keyed by id — the mux render layer
    /// uses it so each agent pane's header shows ITS session's slot
    /// ("#N") instead of the focused session's.
    pub fn session_index_for(&self, id: &SessionId) -> (usize, usize) {
        let len = self.open_sessions.len();
        match self.open_sessions.iter().position(|c| &c.id == id) {
            Some(idx) => (idx + 1, len),
            None => (0, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SessionContext;

    fn sid(s: &str) -> SessionId {
        SessionId::new(s)
    }

    #[test]
    fn session_index_for_reports_1_based_slot_of_an_open_session() {
        let mut store = AgentViewStore::default();
        store.append_session(SessionContext::new(sid("s-1")));
        store.append_session(SessionContext::new(sid("s-2")));
        assert_eq!(store.session_index_for(&sid("s-1")), (1, 2));
        assert_eq!(store.session_index_for(&sid("s-2")), (2, 2));
    }

    #[test]
    fn session_index_for_is_zero_pair_for_an_unknown_session() {
        let mut store = AgentViewStore::default();
        store.append_session(SessionContext::new(sid("s-1")));
        assert_eq!(store.session_index_for(&sid("nope")), (0, 0));
    }

    #[test]
    fn session_index_for_is_zero_pair_when_no_sessions_are_open() {
        let store = AgentViewStore::default();
        assert_eq!(store.session_index_for(&sid("s-1")), (0, 0));
    }
}
