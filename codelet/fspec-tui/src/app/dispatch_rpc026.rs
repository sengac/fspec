//! App::dispatch routing for RPC-026 Action variants:
//! OpenResumePicker, SessionListLoaded, AttachToSession,
//! OpenSearchPalette, SearchHistory, HistorySearchResults,
//! InsertIntoInput.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Each helper is invoked from
//! `App::dispatch`'s match arms.

use codelet_rpc_types::{HistoryMatch, SessionId, SessionInfo};

use crate::components::Action;
use crate::store::SessionContext;
use crate::views::agent::{ResumePicker, SearchPalette};

use super::state::App;

impl App {
    /// Open AgentView's resume picker AND spawn a backend.list_sessions()
    /// task that dispatches `Action::SessionListLoaded` on success.
    pub(crate) fn handle_open_resume_picker(&mut self) {
        self.navigator.agent.resume_popup = Some(ResumePicker::new());
        // Closing the slash popup is normally already done by
        // handle_slash_command, but we null it defensively here so
        // Action::OpenResumePicker can be dispatched programmatically
        // (e.g. from tests) without leaving the slash overlay live.
        self.navigator.agent.slash_popup = None;
        self.navigator.agent.input.reset();
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

    /// Fold a backend list_sessions result into the open resume_popup.
    /// No-op when the popup is closed.
    pub(crate) fn handle_session_list_loaded(&mut self, sessions: Vec<SessionInfo>) {
        if let Some(p) = self.navigator.agent.resume_popup.as_mut() {
            p.set_sessions(sessions);
        }
    }

    /// Attach to a session — index move if already in open_sessions,
    /// else append a fresh SessionContext. Also publishes to
    /// active_session_tx and runs refresh_session_chrome.
    pub(crate) fn handle_attach_to_session(&mut self, session: SessionId) {
        // Drop the resume popup if it's still up (typical case — Enter
        // on the picker dispatched this action).
        self.navigator.agent.resume_popup = None;
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

    /// Open AgentView's search palette empty — no backend call yet,
    /// the first SearchHistory fires on typing.
    pub(crate) fn handle_open_search_palette(&mut self) {
        self.navigator.agent.search_popup = Some(SearchPalette::new());
        self.navigator.agent.slash_popup = None;
        self.navigator.agent.input.reset();
    }

    /// Spawn a `backend.persistence_search_history(query)` task and
    /// dispatch `Action::HistorySearchResults(matches)` on success.
    /// Uses `Handle::try_current` so synchronous unit tests dispatch
    /// without a Tokio runtime get a graceful no-op rather than a panic.
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
    /// search_popup. No-op when the popup is closed.
    pub(crate) fn handle_history_search_results(&mut self, matches: Vec<HistoryMatch>) {
        if let Some(p) = self.navigator.agent.search_popup.as_mut() {
            p.set_matches(matches);
        }
    }

    /// Replace the MultiLineInput's value with `text` AND drop the
    /// search_popup. Does NOT auto-fire InputSubmitted — the user may
    /// edit before pressing Enter (mirrors TS /search behaviour).
    pub(crate) fn handle_insert_into_input(&mut self, text: String) {
        self.navigator.agent.input.set_value(&text);
        self.navigator.agent.search_popup = None;
    }
}
