//! RPC-022 — per-session role overlay accessors for `AgentViewStore`.
//!
//! Feature file: spec/features/rpc022-role-banner.feature
//!
//! The role overlay is per-session text rendered by the inline
//! `RoleBanner` widget above the scrollback. Storage stays as a
//! `HashMap<SessionId, String>` field on `AgentViewStore` (declared in
//! the parent module) — this sub-module hosts the public accessors so
//! `agent_view.rs` honours the 300-LoC ceiling pinned by RPC-024's
//! `session_context_module_exists_under_300_loc` source-shape test.
//!
//! Cards: RPC-022 (parent RPC-002).

use codelet_rpc_types::SessionId;

use super::AgentViewStore;

impl AgentViewStore {
    /// Borrow the role overlay text for `session`, if any.
    ///
    /// Returns `None` when no role is bound to the session — the
    /// inline RoleBanner uses this to collapse its row entirely.
    pub fn role_for(&self, session: &SessionId) -> Option<&str> {
        self.role_by_session.get(session).map(String::as_str)
    }

    /// Set or clear the role overlay for `session`. Passing `None`
    /// removes the entry from the map so subsequent `role_for(...)`
    /// returns `None`.
    pub fn set_role(&mut self, session: SessionId, role: Option<String>) {
        match role {
            Some(text) => {
                self.role_by_session.insert(session, text);
            }
            None => {
                self.role_by_session.remove(&session);
            }
        }
    }
}
