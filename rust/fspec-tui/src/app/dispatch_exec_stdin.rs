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
//!     On success the slot clears; on failure the slot is KEPT and
//!     `ExecStdinWriteFailed` restores the text (TUI-117,
//!     `dispatch_exec_stdin_recovery.rs`).
//!   - `handle_exec_stdin_dismissed` — clear the slot and close the
//!     session's answer box. Nothing is sent, cancelled, or killed.
//!
//! The probe (`probe_exec_stdin_for`) re-checks the focused session's
//! `get_exec_stdin_request` on focus switch and clears the slot when
//! the backend now returns `None` (the exec session no longer exists
//! / is no longer quiet) — mirroring the HITL Running/Idle clear but
//! driven by a probe rather than a status change (exec-stdin
//! performs NO status flip).

use std::sync::Arc;

use codelet_rpc_types::{ExecStdinControl, ExecStdinRequest, SessionId};

use crate::components::Action;
use crate::transport::FspecBackend;

use super::dispatch_exec_stdin_recovery::send_to_app;
use super::state::App;

impl App {
    /// Probe the focused session and apply its result only if no newer
    /// probe or overlay transition has superseded it for that session.
    pub(crate) fn probe_exec_stdin_for(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            // BUG-408: report the skipped probe instead of returning silently.
            tracing::error!(
                session = &session_id.value,
                "no tokio runtime: exec-stdin probe not sent"
            );
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let generation = self.agent_view_store.advance_exec_stdin_generation(&id);
        let handle = tokio::spawn(async move {
            let action = match backend.get_exec_stdin_request(id.clone()).await {
                Ok(request) => Action::ExecStdinProbeFetched {
                    agent_session: id,
                    generation,
                    request,
                },
                // BUG-364: the failure reaches the app; never dropped.
                Err(err) => Action::ExecStdinProbeFailed {
                    agent_session: id,
                    generation,
                    error: err.to_string(),
                },
            };
            send_to_app(&action_tx, action);
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
            self.handle_exec_stdin_dismissed(&agent_session);
            return;
        }
        // TUI-117: Ctrl+X muted this command run.
        if self
            .agent_view_store
            .exec_stdin_is_muted(&agent_session, &request.exec_session_id)
        {
            return;
        }
        self.agent_view_store.set_exec_stdin(agent_session, request);
        self.should_render = true;
    }

    /// TOOL-022 P2: Enter on the exec-stdin prompt. The key handler
    /// already read + cleared the session's answer box and carried the
    /// value in the action. Fire-and-forget `backend.write_exec_stdin`;
    /// the slot clears on success. TUI-117: on failure the slot is KEPT
    /// and `ExecStdinWriteFailed` carries the text back to the box.
    pub(crate) fn handle_exec_stdin_submit(
        &mut self,
        agent_session: SessionId,
        exec_session: String,
        text: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            // BUG-408: the key handler already took the text out of the box;
            // put it back with the failure instead of dropping it.
            self.handle_exec_stdin_write_failed(
                &agent_session,
                &exec_session,
                &text,
                "no tokio runtime: write_exec_stdin not sent",
            );
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = agent_session;
        let handle = tokio::spawn(async move {
            let result = backend
                .write_exec_stdin(id.clone(), exec_session.clone(), text.clone())
                .await;
            let action = match result {
                // Success — clear the slot (the user answered).
                Ok(()) => Action::ExecStdinDismissed { agent_session: id },
                Err(err) => Action::ExecStdinWriteFailed {
                    agent_session: id,
                    exec_session,
                    text,
                    error: err,
                },
            };
            send_to_app(&action_tx, action);
        });
        self.pending_tasks.push(handle);
    }

    /// Clear the slot (exit, new output, a re-probe with no request, a
    /// successful send, Ctrl+X). NOTHING is sent, cancelled, or killed.
    /// TUI-117: the session's answer box closes and keeps unsent text in
    /// its history.
    pub(crate) fn handle_exec_stdin_dismissed(&mut self, agent_session: &SessionId) {
        self.agent_view_store.clear_exec_stdin(agent_session);
        self.navigator.agent.close_stdin_box(agent_session);
        self.should_render = true;
    }

    /// TUI-116: Ctrl+C / Ctrl+D in the stdin answer box. Sends
    /// `backend.control_exec_stdin`; a success clears the slot (the backend
    /// cleared its request too). TUI-117: a failure keeps it and sends
    /// `ExecStdinControlFailed` for the box's red error row.
    pub(crate) fn handle_exec_stdin_control(
        &mut self,
        agent_session: SessionId,
        exec_session: String,
        control: ExecStdinControl,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::error!(
                target: "tui116",
                session = &agent_session.value,
                "no tokio runtime: control_exec_stdin {control:?} not sent",
            );
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let result = backend
                .control_exec_stdin(agent_session.clone(), exec_session.clone(), control)
                .await;
            let action = match result {
                Ok(()) => Action::ExecStdinDismissed { agent_session },
                Err(err) => Action::ExecStdinControlFailed {
                    agent_session,
                    exec_session,
                    control,
                    error: err.to_string(),
                },
            };
            send_to_app(&action_tx, action);
        });
        self.pending_tasks.push(handle);
    }

    /// Route the TOOL-022 P2 Action variants through their reducers.
    /// Called from `dispatch.rs` capability fallbacks.
    pub(crate) fn try_dispatch_exec_stdin(&mut self, action: &Action) -> bool {
        match action {
            Action::ExecStdinProbeFetched {
                agent_session,
                generation,
                request,
            } => {
                if self
                    .agent_view_store
                    .exec_stdin_generation_is_current(agent_session, *generation)
                {
                    match request {
                        Some(request) => self.handle_exec_stdin_prompt_fetched(
                            agent_session.clone(),
                            request.clone(),
                        ),
                        None => self.handle_exec_stdin_dismissed(agent_session),
                    }
                }
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
            Action::ExecStdinControl {
                agent_session,
                exec_session,
                control,
            } => {
                self.handle_exec_stdin_control(
                    agent_session.clone(),
                    exec_session.clone(),
                    *control,
                );
            }
            _ => return self.try_dispatch_exec_stdin_recovery(action),
        }
        true
    }
}
