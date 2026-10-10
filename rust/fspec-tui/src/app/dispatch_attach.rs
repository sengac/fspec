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
use crate::views::ViewMode;

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
        self.finish_session_attach_wiring(session);
    }

    /// Shared post-focus wiring for the attach / reattach / restore
    /// handlers: active-session publish, Agent-view flip (TUI-113 R1),
    /// R3 daemon-pointer publish, chrome hydration (RPC-018/052/061/430)
    /// and the RPC-049 durable-restore round-trip
    /// (`backend.resume_session` → `SessionResumeComplete` →
    /// `get_buffered_output` replay).
    pub(crate) fn finish_session_attach_wiring(&mut self, session: SessionId) {
        let _ = self.active_session_tx.send(Some(session.clone()));
        // TUI-113 R1: landing on a restored session must land in the
        // Agent view (mirrors `handle_open_agent_view(Some)`): set the
        // navigation target, flip the view, and drop the board bar's
        // ring focus (BUG-197 R3/R4) so no stale chip highlight
        // survives the flip. Guards: never steal the view while the
        // /resume list is open, and never leave the mux grid (a mux
        // attach focuses the agent pane in-grid instead).
        if self.navigator.active_view != ViewMode::Agent
            && self.navigator.active_view != ViewMode::Mux
            && self.navigator.agent.resume_view.is_none()
        {
            self.agent_view_store
                .set_navigation_target(Some(session.clone()));
            self.navigator.active_view = ViewMode::Agent;
            self.board_store.menu_focus_to_columns();
        }
        // TUI-113 R3: publish the attached session to the daemon's
        // active-session pointer (fire-and-forget) so the next client
        // start can restore it.
        self.spawn_set_active_session(session.clone());
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

    /// TUI-113: reconnect/restore re-attach — focus the session's EXISTING
    /// tab and run the durable-restore round-trip (RPC-049:
    /// `resume_session` + `get_buffered_output` replay), but NEVER
    /// re-open a closed tab. A session the user closed stays closed;
    /// the bootstrap lazy create-on-enter-work-unit path (RPC-012) is
    /// the only path that creates a session on a fresh start.
    ///
    /// When the tab is already open this mirrors
    /// `handle_attach_to_session` (focus + flip to Agent view + R3
    /// pointer publish + chrome/supervisor/debug hydration + resume
    /// round-trip). When the tab is closed it focuses nothing — only
    /// the R3 pointer publish fires (the daemon's sticky pointer still
    /// tracks the last-viewed session for a later restart).
    pub(crate) fn handle_reattach_session(&mut self, session: SessionId) {
        tracing::info!(
            session_id = %session.value,
            "TUI-113: handle_reattach_session"
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
                    "TUI-113: reattach — session tab open, focusing"
                );
                self.agent_view_store.focus_session_index(idx);
            }
            None => {
                tracing::debug!(
                    session_id = %session.value,
                    "TUI-113: reattach — session tab closed, NOT re-opening"
                );
                return;
            }
        }
        self.finish_session_attach_wiring(session);
    }

    /// TUI-113: bootstrap/fresh-start restore — re-open the last-viewed
    /// session (append a fresh tab when absent, focus it when present)
    /// and run the durable-restore round-trip. Unlike
    /// `handle_reattach_session` this RE-OPENS a closed/absent tab: a
    /// fresh client process restoring its last session from a live
    /// daemon is the RPC-049 durable-restore flow, not a reconnect
    /// re-attach.
    pub(crate) fn handle_restore_session(&mut self, session: SessionId) {
        tracing::info!(
            session_id = %session.value,
            "TUI-113: handle_restore_session"
        );
        let existing_idx = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|c| c.id == session);
        match existing_idx {
            Some(idx) => {
                self.agent_view_store.focus_session_index(idx);
            }
            None => {
                self.agent_view_store
                    .append_session(SessionContext::new(session.clone()));
            }
        }
        self.finish_session_attach_wiring(session);
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
