//! RPC-056 / BUG-601 — the daemon's turned-off blocklist rules per session,
//! as last answered, held by `AgentViewStore`.
//!
//! Feature file: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature file: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! The daemon enforces and saves the set (BUG-601); this is only its last
//! answer, so the view shows the state the daemon enforces. Lives in its own
//! sub-module so the parent `agent_view.rs` keeps under the 300-LoC
//! source-shape invariant pinned by `rpc025-source-shape.feature`.

use std::collections::HashSet;

use codelet_rpc_types::SessionId;

use super::AgentViewStore;

impl AgentViewStore {
    /// The daemon's last answer of the rules turned off for `session`, if
    /// it has answered.
    pub fn blocklist_disabled_for(&self, session: &SessionId) -> Option<&HashSet<String>> {
        self.blocklist_disabled_by_session.get(session)
    }

    /// Record the daemon's answer: `rules` are turned off for `session`.
    pub fn set_blocklist_disabled(&mut self, session: SessionId, rules: Vec<String>) {
        self.blocklist_disabled_by_session
            .insert(session, rules.into_iter().collect());
    }
}
