//! TUI-125: a session the daemon announces (a sub-agent spawned with
//! AgentManager, a session created from another client) opens a tab without
//! taking focus.
//!
//! Feature: spec/features/sessions-the-user-did-not-create-never-take-focus.feature

use codelet_rpc_types::SessionId;

use super::{AgentViewStore, SessionContext};

impl AgentViewStore {
    /// Append `ctx` as a new tab while the current session stays current.
    /// No-op when a tab for the id is already open.
    pub fn append_session_unfocused(&mut self, mut ctx: SessionContext) {
        if self.open_sessions.iter().any(|c| c.id == ctx.id) {
            return;
        }
        ctx.opened_unfocused = true;
        self.restore_work_unit_focus_projection(&ctx.id);
        self.open_sessions.push(ctx);
    }

    /// True exactly once for a tab opened by [`Self::append_session_unfocused`]:
    /// the TUI's own creation answer for that session claims it to focus it.
    pub fn claim_unfocused_tab(&mut self, id: &SessionId) -> bool {
        self.open_sessions
            .iter_mut()
            .find(|c| &c.id == id)
            .is_some_and(|c| std::mem::take(&mut c.opened_unfocused))
    }
}
