//! BUG-163 — id-keyed session indexing for the mux agent panes.
//!
//! Feature: spec/features/mux-agent-panes-render-distinct-window-sessions.feature
//!
//! Lives in its own sibling module so `agent_view.rs` stays under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature` /
//! `rpc025-source-shape.feature` (same pattern as `navigation.rs` for
//! RPC-096).

use super::{AgentViewStore, SessionContext};
use codelet_rpc_types::SessionId;

/// BUG-377: no agent session is open at the index a caller asked to focus.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("no agent session is open at index {index} ({open} open)")]
pub struct SessionNotOpen {
    pub index: usize,
    pub open: usize,
}

impl AgentViewStore {
    /// BUG-377: focus the session at `index` and return its context, or refuse with [`SessionNotOpen`] and leave
    /// the current focus unchanged.
    pub fn focus_open_session(&mut self, index: usize) -> Result<&SessionContext, SessionNotOpen> {
        let open = self.open_sessions.len();
        if index >= open {
            return Err(SessionNotOpen { index, open });
        }
        self.current_session_index = index;
        self.open_sessions
            .get(index)
            .ok_or(SessionNotOpen { index, open })
    }

    /// BUG-163: the `(current, total)` 1-based index of the session with
    /// the given id. BUG-391: `None` when the session is not open. Mirrors
    /// [`Self::session_index`] but keyed by id — the mux render layer
    /// uses it so each agent pane's header shows ITS session's slot
    /// ("#N") instead of the focused session's.
    pub fn session_index_for(&self, id: &SessionId) -> Option<(usize, usize)> {
        let len = self.open_sessions.len();
        self.open_sessions
            .iter()
            .position(|c| &c.id == id)
            .map(|idx| (idx + 1, len))
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
        assert_eq!(store.session_index_for(&sid("s-1")), Some((1, 2)));
        assert_eq!(store.session_index_for(&sid("s-2")), Some((2, 2)));
    }

    #[test]
    fn session_index_for_is_none_for_an_unknown_session() {
        let mut store = AgentViewStore::default();
        store.append_session(SessionContext::new(sid("s-1")));
        assert_eq!(store.session_index_for(&sid("nope")), None);
    }

    #[test]
    fn session_index_for_is_none_when_no_sessions_are_open() {
        let store = AgentViewStore::default();
        assert_eq!(store.session_index_for(&sid("s-1")), None);
    }
}
