//! App::dispatch routing for session attach + RPC-049 durable restore.
//!
//! Factored out of `app/dispatch_resume_search_views.rs` so that file
//! stays under the 300-LoC ceiling. Hosts the attach round-trip
//! (index move or fresh SessionContext append + `backend.resume_session`
//! spawn), the `SessionResumeComplete` buffered-output replay, and the
//! `list_sessions` spawn shared with `handle_open_resume_view`.

use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::store::SessionContext;

use super::state::App;

impl App {
    /// Attach to a session — index move if already in open_sessions,
    /// else append a fresh SessionContext. Also publishes to
    /// active_session_tx and runs refresh_session_chrome.
    ///
    /// RPC-049: also spawns a tokio task that awaits
    /// `backend.resume_session(session_id)` so the durable-restore
    /// round-trip lands the session's prior messages + token state.
    /// On Ok the task dispatches `Action::SessionResumeComplete(id)`,
    /// which `handle_session_resume_complete` reacts to by seeding the
    /// session's scrollback from `backend.get_buffered_output`. On Err
    /// the task dispatches `Action::EmitSessionNotice` so the failure
    /// is surfaced into the originating session's scrollback.
    pub(crate) fn handle_attach_to_session(&mut self, session: SessionId) {
        tracing::info!(
            session_id = %session.value,
            "TUI: handle_attach_to_session: closing resume view and attaching to session"
        );
        self.navigator.agent.resume_view = None;
        let existing_idx = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|c| c.id == session);
        match existing_idx {
            Some(idx) => {
                tracing::debug!(
                    session_id = %session.value,
                    index = idx,
                    "TUI: handle_attach_to_session: session already in open_sessions, focusing"
                );
                self.agent_view_store.focus_session_index(idx);
            }
            None => {
                tracing::debug!(
                    session_id = %session.value,
                    "TUI: handle_attach_to_session: session not in open_sessions, appending"
                );
                self.agent_view_store
                    .append_session(SessionContext::new(session.clone()));
            }
        }
        let _ = self.active_session_tx.send(Some(session.clone()));
        self.refresh_session_chrome(session.clone());

        // RPC-052: hydrate the per-session draft from the backend so
        // attaching to a session restores its pending input.
        self.spawn_hydrate_pending_input(session.clone());

        // RPC-061 rule [9]: re-load the supervisor snapshot on session
        // activation so re-attaching to an existing session via
        // /resume paints a fresh `[Subordinate of: …]` badge instead
        // of a stale (possibly empty) one.
        self.spawn_load_supervisors(session.clone());

        // RPC-430: hydrate debug state from the backend so the [DEBUG]
        // badge reflects the ground-truth on session attach.
        self.spawn_hydrate_debug_state(session.clone());

        // RPC-049: spawn the durable-restore round-trip. Honour the
        // synchronous unit-test path so tests that don't drive a tokio
        // runtime can still observe the open_sessions move/append.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            tracing::info!(
                session_id = %session.value,
                "TUI: spawning backend.resume_session round-trip"
            );
            match backend.resume_session(session.clone()).await {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.value,
                        "TUI: backend.resume_session succeeded, dispatching SessionResumeComplete"
                    );
                    let _ = action_tx.send(Action::SessionResumeComplete(session));
                }
                Err(e) => {
                    tracing::error!(
                        session_id = %session.value,
                        error = %e,
                        "TUI: backend.resume_session failed"
                    );
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session,
                        format!("[error] /resume failed: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-049: react to the success outcome of `backend.resume_session`
    /// by fetching `backend.get_buffered_output(id, 1000)` and replaying
    /// each returned chunk into the action bus as
    /// `Action::ChunkReceived(id, chunk)`. Silently no-ops in
    /// non-runtime contexts (synchronous unit-test fallback).
    pub(crate) fn handle_session_resume_complete(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let chunks = backend
                .get_buffered_output(session.clone(), 1000)
                .await
                .unwrap_or_default();
            for chunk in chunks {
                let _ = action_tx.send(Action::ChunkReceived(session.clone(), chunk));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-427: resolve current project path and pass it to
    /// `backend.list_sessions(project_path)` so the resume list is
    /// filtered to the current project.
    pub(crate) fn spawn_list_sessions(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let project_path = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Ok(sessions) = backend.list_sessions(project_path).await {
                let _ = action_tx.send(Action::SessionListLoaded(sessions));
            }
        });
        self.pending_tasks.push(handle);
    }
}
