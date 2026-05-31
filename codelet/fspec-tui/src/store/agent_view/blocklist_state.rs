//! RPC-056 — per-session blocklist-disabled rule set held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/rpc056-blocklist-view-dispatch.feature
//!
//! Mirrors the TS `disabledBlocklistRules` `Set<string>` state lifted on
//! the AgentView component. Lives in its own sub-module so the parent
//! `agent_view.rs` keeps under the 300-LoC source-shape invariant pinned
//! by `rpc025-source-shape.feature`.

use std::collections::HashSet;

use codelet_rpc_types::SessionId;

use super::AgentViewStore;

impl AgentViewStore {
    /// Borrow the in-memory disabled-rule set for `session`, if any.
    /// Returns `None` when the user hasn't toggled any rule yet for
    /// this session — callers should default to an empty set in that
    /// case.
    pub fn blocklist_disabled_for(&self, session: &SessionId) -> Option<&HashSet<String>> {
        self.blocklist_disabled_by_session.get(session)
    }

    /// Borrow a mutable handle to the disabled-rule set for `session`,
    /// inserting a fresh empty `HashSet` when missing.
    pub fn blocklist_disabled_for_mut(
        &mut self,
        session: &SessionId,
    ) -> &mut HashSet<String> {
        self.blocklist_disabled_by_session
            .entry(session.clone())
            .or_default()
    }

    /// Toggle a rule id in the per-session disabled set. Inserts when
    /// absent; removes when present. Returns the new "disabled" state
    /// for the rule (true == now disabled).
    pub fn toggle_blocklist_rule(
        &mut self,
        session: &SessionId,
        rule_id: impl Into<String>,
    ) -> bool {
        let id = rule_id.into();
        let set = self.blocklist_disabled_for_mut(session);
        if set.contains(&id) {
            set.remove(&id);
            false
        } else {
            set.insert(id);
            true
        }
    }
}
