//! App::dispatch routing for RPC-026 Action variants.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. RPC-026 widgets are FULL-SCREEN mode
//! views (not popups) — the helpers here drive their lifecycle plus
//! the new `persistence_delete_session` round-trip.
//!
//! RPC-064 extends `handle_search_history` with a 150ms keystroke
//! debounce (so rapid typing inside the window collapses to a single
//! backend round-trip) and widens `handle_history_search_results` to
//! discard responses whose originating query no longer matches the
//! live `search_view.query()`.

use std::time::Duration;

use codelet_rpc_types::{HistoryMatch, SessionId, SessionInfo};

use crate::components::Action;
use crate::views::agent::{ResumeSessionView, SearchHistoryView};

use super::state::App;

/// RPC-064: the 150ms keystroke-to-RPC debounce window used by
/// [`App::handle_search_history`]. Sized to match the TS reference
/// implementation in `AgentView.tsx` (search-as-you-type without
/// flooding the backend on every keystroke).
const SEARCH_DEBOUNCE_MS: u64 = 150;

impl App {
    /// Open AgentView's resume mode view AND spawn a
    /// `backend.list_sessions(project_path)` task that dispatches
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
    ///
    /// RPC-064: the live input value is INTENTIONALLY preserved across
    /// `/search` open/close so the user's in-progress draft survives a
    /// quick history lookup (matches TS AgentView.tsx — the live input
    /// state is independent from `setIsSearchMode(true)`). The slash
    /// palette path already clears the typed `/search` text in
    /// `AgentView::handle_popup_key` (PopupOutcome::Selected →
    /// `input.reset()` BEFORE emitting `Action::SlashCommandSelected`),
    /// so no clear is needed here. The `Ctrl+R` chord path now correctly
    /// leaves the draft intact for the Esc-restores-draft scenario.
    pub(crate) fn handle_open_search_view(&mut self) {
        self.navigator.agent.search_view = Some(SearchHistoryView::new());
        self.navigator.agent.slash_popup = None;
    }

    /// Drop AgentView's search mode view. No-op when already closed.
    pub(crate) fn handle_close_search_view(&mut self) {
        self.navigator.agent.search_view = None;
    }

    /// Fold a backend list_sessions result into the open resume_view.
    /// No-op when the view is closed.
    pub(crate) fn handle_session_list_loaded(&mut self, sessions: Vec<SessionInfo>) {
        tracing::info!(
            session_count = sessions.len(),
            session_ids = ?sessions.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            "TUI: handle_session_list_loaded: received session list from backend"
        );
        if let Some(v) = self.navigator.agent.resume_view.as_mut() {
            v.set_sessions(sessions);
            tracing::debug!("TUI: handle_session_list_loaded: folded into resume_view");
        } else {
            tracing::debug!("TUI: handle_session_list_loaded: resume_view not open, discarding");
        }
    }

    /// Spawn a `backend.persistence_search_history(query)` task and
    /// dispatch `Action::HistorySearchResults { query, matches }` on
    /// success.
    ///
    /// RPC-064: keystroke-to-RPC debouncing. Each call aborts the
    /// previous debounce handle so rapid typing inside the
    /// `SEARCH_DEBOUNCE_MS` window collapses to a single backend
    /// round-trip carrying the final query.
    pub(crate) fn handle_search_history(&mut self, query: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        // Last-write-wins: cancel any pending debounced call.
        if let Some(handle) = self.search_history_debounce_handle.take() {
            handle.abort();
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let q = query;
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(SEARCH_DEBOUNCE_MS)).await;
            if let Ok(matches) = backend.persistence_search_history(q.clone()).await {
                let _ = action_tx.send(Action::HistorySearchResults { query: q, matches });
            }
        });
        // RPC-064: park the abort token for the next keystroke's
        // cancellation AND push the `JoinHandle` itself onto
        // `pending_tasks` so the existing RPC-026 test harness keeps
        // working (it awaits via `App::next_pending_task`).
        self.search_history_debounce_handle = Some(handle.abort_handle());
        self.pending_tasks.push(handle);
    }

    /// Fold a backend persistence_search_history result into the open
    /// search_view. No-op when the view is closed.
    ///
    /// RPC-064: stale-discard. The response is folded ONLY when its
    /// originating `query` still matches the view's live `query()` —
    /// older in-flight responses that arrive AFTER the user has typed
    /// more characters are silently dropped.
    pub(crate) fn handle_history_search_results(
        &mut self,
        query: String,
        matches: Vec<HistoryMatch>,
    ) {
        if let Some(v) = self.navigator.agent.search_view.as_mut() {
            if v.query() == query {
                v.set_matches(matches);
            }
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
    /// a follow-up `backend.list_sessions(project_path)` so the resume_view
    /// repaints without the deleted session. The resume_view's
    /// `delete_confirm` is cleared by the widget itself when the
    /// Primary outcome fires.
    ///
    /// RPC-427: resolves current project path and passes it to
    /// `list_sessions()` so the refreshed list remains project-filtered.
    pub(crate) fn handle_confirm_delete_session(&mut self, id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        // If the deleted session is currently open, remove it from
        // open_sessions and clamp the index.
        self.agent_view_store.remove_session_if_open(&id);
        // MUX-002: closing a session shrinks the open-session list —
        // re-clamp the mux agent window (agent slots stay in the grid).
        self.mux_sync_window();
        let project_path = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if backend.persistence_delete_session(id).await.is_ok() {
                if let Ok(sessions) = backend.list_sessions(project_path).await {
                    let _ = action_tx.send(Action::SessionListLoaded(sessions));
                }
            }
        });
        self.pending_tasks.push(handle);
    }
}
