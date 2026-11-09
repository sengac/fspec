//! TUI-125: a session the TUI did not create on the user's request (a
//! sub-agent spawned with AgentManager, a session created from another
//! client, one seen again on reconnect) opens a tab without taking focus.
//!
//! Feature: spec/features/sessions-the-user-did-not-create-never-take-focus.feature
//!
//! `Action::SessionObserved` (the daemon's session-created broadcast and the
//! reconnect replay) opens with [`SessionFocus::Keep`]; the TUI's own
//! creation answer (`SessionCreated` / `WorkUnitSessionCreated`) and the
//! startup replay open with [`SessionFocus::Take`]. When the announcement
//! of a session the user created arrives before the creation answer, the
//! answer focuses the tab the announcement opened, once.

use codelet_rpc_types::SessionId;

use super::state::App;

/// Whether a newly opened session takes focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionFocus {
    /// The session becomes the current one (the user created it).
    Take,
    /// The current session, view, pane and draft stay as they are.
    Keep,
}

impl App {
    /// Open `session` as a tab while the current session stays current: the
    /// daemon's active-session pointer, the mux focused pane and a pending
    /// mux new-agent prompt are left alone.
    pub(crate) fn open_session_unfocused(&mut self, session: SessionId) {
        self.agent_view_store
            .append_session_unfocused(crate::store::SessionContext::new(session));
        self.rebuild_focus_occupancy();
        if self.navigator.mux.config().enabled {
            let ids: Vec<SessionId> = self
                .agent_view_store
                .open_sessions()
                .iter()
                .map(|c| c.id.clone())
                .collect();
            self.navigator.mux.sync_window_keeping_focus(&ids);
        }
    }

    /// The creation answer for a session whose tab the announcement already
    /// opened unfocused: focus it now (once; a repeated answer is a no-op).
    pub(crate) fn focus_announced_creation(&mut self, session: &SessionId) {
        if !self.agent_view_store.claim_unfocused_tab(session) {
            return;
        }
        let Some(index) = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|c| &c.id == session)
        else {
            return;
        };
        self.switch_to_session_index(index);
        if self.navigator.mux.config().enabled {
            self.navigator.mux.cancel_new_agent_request();
            self.navigator.mux.focus_agent_pane_for_session(session);
        }
        let _ = self.active_session_tx.send(Some(session.clone()));
    }
}
