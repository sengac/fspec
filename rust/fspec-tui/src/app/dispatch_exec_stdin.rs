//! TOOL-022 P2 — App::dispatch reducers for the inline exec-stdin
//! prompt + the probe that surfaces requests into the store.
//!
//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! Store-authoritative transitions over the per-session exec-stdin
//! slot (`store/agent_view/exec_stdin_state.rs`). Unlike the HITL
//! reducers (which always send a response before clearing), exec-stdin
//! is a PURE overlay:
//!
//!   - `handle_exec_stdin_prompt_fetched` — fold a fetched
//!     `ExecStdinRequest` into the slot (fresh detector fire = fresh
//!     quiet_seconds; overwrites any stale slot).
//!   - `handle_exec_stdin_submit` — fire-and-forget
//!     `backend.write_exec_stdin(agent_session, exec_session, text)`.
//!     On success the slot clears; on failure the slot is KEPT and the
//!     error is logged via `tracing` (NEVER a scrollback notice).
//!   - `handle_exec_stdin_dismissed` — clear the slot only. Nothing is
//!     sent, cancelled, or killed; the session keeps running.
//!
//! The probe (`probe_exec_stdin_for`) re-checks the focused session's
//! `get_exec_stdin_request` on focus switch and clears the slot when
//! the backend now returns `None` (the exec session no longer exists
//! / is no longer quiet) — mirroring the HITL Running/Idle clear but
//! driven by a probe rather than a status change (exec-stdin
//! performs NO status flip).

use std::sync::Arc;

use codelet_rpc_types::{ExecStdinRequest, SessionId};

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

impl App {
    /// Probe the focused session's exec-stdin request and fold the
    /// result into the store: `Some` → dispatch
    /// `Action::ExecStdinPromptFetched` (slot populated / refreshed);
    /// `None` → clear the slot. No-op when no runtime is available
    /// (sync unit-test fallback).
    pub(crate) fn probe_exec_stdin_for(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            match backend.get_exec_stdin_request(id.clone()).await {
                Ok(Some(request)) => {
                    let _ = action_tx.send(Action::ExecStdinPromptFetched {
                        agent_session: id,
                        request,
                    });
                }
                Ok(None) => {
                    let _ = action_tx.send(Action::ExecStdinDismissed {
                        agent_session: id,
                    });
                }
                Err(err) => {
                    tracing::debug!(
                        target: "tool022",
                        session = &id.value,
                        error = %err,
                        "get_exec_stdin_request probe failed (silently dropped)",
                    );
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// TOOL-022 P2: fold a fetched `ExecStdinRequest` into the
    /// per-session store slot. Overwrites any prior slot (fresh
    /// detector fire = fresh quiet_seconds). Guard: never populate
    /// while a HITL prompt occupies the slot for the same session —
    /// the exec-stdin overlay must not show behind a HITL prompt
    /// (rule [9]: "never re-shown while a HITL prompt occupies the
    /// slot").
    pub(crate) fn handle_exec_stdin_prompt_fetched(
        &mut self,
        agent_session: SessionId,
        request: ExecStdinRequest,
    ) {
        if self
            .agent_view_store
            .hitl_prompt_for(&agent_session)
            .is_some()
        {
            self.agent_view_store.clear_exec_stdin(&agent_session);
            return;
        }
        self.agent_view_store.set_exec_stdin(agent_session, request);
        self.should_render = true;
    }

    /// TOOL-022 P2: Enter on the exec-stdin prompt. The key handler
    /// already read + cleared the SHARED composer input and carried
    /// the value in the action. Fire-and-forget
    /// `backend.write_exec_stdin`; the slot clears on success and is
    /// KEPT (with a `tracing::error!`) on failure — NEVER a scrollback
    /// notice.
    pub(crate) fn handle_exec_stdin_submit(
        &mut self,
        agent_session: SessionId,
        exec_session: String,
        text: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = agent_session;
        let handle = tokio::spawn(async move {
            match backend
                .write_exec_stdin(id.clone(), exec_session.clone(), text)
                .await
            {
                Ok(()) => {
                    // Success — clear the slot (the exec session now
                    // has fresh output / the user answered).
                    let _ = action_tx.send(Action::ExecStdinDismissed {
                        agent_session: id,
                    });
                }
                Err(err) => {
                    // Failure — KEEP the slot (re-probe will re-fire)
                    // and log. NEVER a scrollback notice.
                    tracing::error!(
                        target: "tool022",
                        session = &id.value,
                        exec_session = %exec_session,
                        error = %err,
                        "write_exec_stdin failed — keeping the exec-stdin slot",
                    );
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// TOOL-022 P2: Esc on the exec-stdin prompt — clear the slot only.
    /// NOTHING is sent, cancelled, or killed; the session keeps running.
    pub(crate) fn handle_exec_stdin_dismissed(&mut self, agent_session: &SessionId) {
        self.agent_view_store.clear_exec_stdin(agent_session);
        self.should_render = true;
    }

    /// Route the TOOL-022 P2 Action variants through their reducers.
    /// Called from `dispatch.rs` capability fallbacks.
    pub(crate) fn try_dispatch_exec_stdin(&mut self, action: &Action) -> bool {
        match action {
            Action::ExecStdinPromptFetched {
                agent_session,
                request,
            } => {
                self.handle_exec_stdin_prompt_fetched(agent_session.clone(), request.clone());
            }
            Action::ExecStdinSubmit {
                agent_session,
                exec_session,
                text,
            } => {
                self.handle_exec_stdin_submit(
                    agent_session.clone(),
                    exec_session.clone(),
                    text.clone(),
                );
            }
            Action::ExecStdinDismissed { agent_session } => {
                self.handle_exec_stdin_dismissed(agent_session);
            }
            _ => return false,
        }
        true
    }
}
