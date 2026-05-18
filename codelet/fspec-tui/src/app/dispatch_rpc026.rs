//! App::dispatch routing for RPC-026 Action variants.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. RPC-026 widgets are FULL-SCREEN mode
//! views (not popups) — the helpers here drive their lifecycle plus
//! the new `persistence_delete_session` round-trip.

use codelet_rpc_types::{HistoryMatch, SessionId, SessionInfo};

use crate::components::Action;
use crate::store::SessionContext;
use crate::views::agent::{ResumeSessionView, SearchHistoryView};

use super::state::App;

impl App {
    /// Open AgentView's resume mode view AND spawn a
    /// `backend.list_sessions()` task that dispatches
    /// `Action::SessionListLoaded` on success.
    pub(crate) fn handle_open_resume_view(&mut self) {
        self.navigator.agent.resume_view = Some(ResumeSessionView::new());
        self.navigator.agent.slash_popup = None;
        self.navigator.agent.input.reset();
        self.spawn_list_sessions();
    }

    /// Drop AgentView's resume mode view. No-op when already closed.
    pub(crate) fn handle_close_resume_view(&mut self) {
        self.navigator.agent.resume_view = None;
    }

    /// Open AgentView's search mode view empty — no backend call yet,
    /// the first SearchHistory fires on typing.
    pub(crate) fn handle_open_search_view(&mut self) {
        self.navigator.agent.search_view = Some(SearchHistoryView::new());
        self.navigator.agent.slash_popup = None;
        self.navigator.agent.input.reset();
    }

    /// Drop AgentView's search mode view. No-op when already closed.
    pub(crate) fn handle_close_search_view(&mut self) {
        self.navigator.agent.search_view = None;
    }

    /// Fold a backend list_sessions result into the open resume_view.
    /// No-op when the view is closed.
    pub(crate) fn handle_session_list_loaded(&mut self, sessions: Vec<SessionInfo>) {
        if let Some(v) = self.navigator.agent.resume_view.as_mut() {
            v.set_sessions(sessions);
        }
    }

    /// Attach to a session — index move if already in open_sessions,
    /// else append a fresh SessionContext. Also publishes to
    /// active_session_tx and runs refresh_session_chrome.
    pub(crate) fn handle_attach_to_session(&mut self, session: SessionId) {
        self.navigator.agent.resume_view = None;
        let existing_idx = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|c| c.id == session);
        match existing_idx {
            Some(idx) => {
                while self.agent_view_store.current_session_index() != idx {
                    let delta = if idx > self.agent_view_store.current_session_index() {
                        1
                    } else {
                        -1
                    };
                    self.agent_view_store.cycle_session(delta);
                }
            }
            None => {
                self.agent_view_store
                    .append_session(SessionContext::new(session.clone()));
            }
        }
        let _ = self.active_session_tx.send(Some(session.clone()));
        self.refresh_session_chrome(session);
    }

    /// Spawn a `backend.persistence_search_history(query)` task and
    /// dispatch `Action::HistorySearchResults(matches)` on success.
    pub(crate) fn handle_search_history(&mut self, query: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Ok(matches) = backend.persistence_search_history(query).await {
                let _ = action_tx.send(Action::HistorySearchResults(matches));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a backend persistence_search_history result into the open
    /// search_view. No-op when the view is closed.
    pub(crate) fn handle_history_search_results(&mut self, matches: Vec<HistoryMatch>) {
        if let Some(v) = self.navigator.agent.search_view.as_mut() {
            v.set_matches(matches);
        }
    }

    /// Replace the MultiLineInput's value with `text` AND drop the
    /// search_view. Does NOT auto-fire InputSubmitted — the user may
    /// edit before pressing Enter (mirrors TS /search behaviour).
    pub(crate) fn handle_insert_into_input(&mut self, text: String) {
        self.navigator.agent.input.set_value(&text);
        self.navigator.agent.search_view = None;
    }

    /// Open the delete-confirm dialog inside resume_view. Does NOT
    /// dispatch anything to the backend yet — the dialog's Primary
    /// outcome dispatches `Action::ConfirmDeleteSession`.
    pub(crate) fn handle_request_delete_session(&mut self, _id: SessionId) {
        // The resume_view already opened its internal delete_confirm
        // dialog as a side-effect of producing the
        // ResumeSessionViewOutcome::RequestDelete. This handler exists
        // so the App can react further (e.g. logging) — currently a
        // deliberate no-op.
    }

    /// Spawn `backend.persistence_delete_session(id)`; on success spawn
    /// a follow-up `backend.list_sessions()` so the resume_view
    /// repaints without the deleted session. The resume_view's
    /// `delete_confirm` is cleared by the widget itself when the
    /// Primary outcome fires.
    pub(crate) fn handle_confirm_delete_session(&mut self, id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        // If the deleted session is currently open, remove it from
        // open_sessions and clamp the index.
        self.agent_view_store.remove_session_if_open(&id);
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if backend.persistence_delete_session(id).await.is_ok() {
                if let Ok(sessions) = backend.list_sessions().await {
                    let _ = action_tx.send(Action::SessionListLoaded(sessions));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    fn spawn_list_sessions(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Ok(sessions) = backend.list_sessions().await {
                let _ = action_tx.send(Action::SessionListLoaded(sessions));
            }
        });
        self.pending_tasks.push(handle);
    }
}
