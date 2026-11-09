//! TUI-117 — App::dispatch reducers for stdin prompt recovery: failed
//! sends, muting one command run, and closing a session's answer box.
//!
//! Feature: spec/features/stdin-prompt-recovery-and-per-session-state.feature
//!
//!   - `handle_exec_stdin_write_failed` — the text goes back into the
//!     session's answer box with a red "Not sent: <reason>" row while the
//!     same request is still open; once it closed, the text joins the box
//!     history like any other unsent text.
//!   - `handle_exec_stdin_control_failed` — a red "Interrupt failed" /
//!     "End of input failed" row while the request is still open.
//!   - `handle_exec_stdin_muted` — Ctrl+X: mute this exec session for the
//!     agent session and close the notice.
//!   - `handle_exec_stdin_probe_failed` — BUG-364: a failed focus / Paused
//!     probe is logged at error level; while it is still the current probe
//!     an open notice is marked as possibly stale.

use codelet_rpc_types::{ExecStdinControl, SessionId};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::Action;

use super::state::App;

/// Send `action` to the app loop, logging when the loop has closed.
pub(crate) fn send_to_app(action_tx: &UnboundedSender<Action>, action: Action) {
    if let Err(err) = action_tx.send(action) {
        tracing::warn!(
            target: "tui117",
            error = %err,
            "app loop closed before an exec-stdin action was delivered",
        );
    }
}

impl App {
    /// True while `agent_session`'s open request belongs to `exec_session`.
    fn exec_stdin_request_open(&self, agent_session: &SessionId, exec_session: &str) -> bool {
        self.agent_view_store
            .exec_stdin_for(agent_session)
            .is_some_and(|request| request.exec_session_id == exec_session)
    }

    pub(crate) fn handle_exec_stdin_write_failed(
        &mut self,
        agent_session: &SessionId,
        exec_session: &str,
        text: &str,
        error: &str,
    ) {
        tracing::warn!(
            target: "tui117",
            session = &agent_session.value,
            exec_session,
            error,
            "write_exec_stdin failed — text restored to the answer box",
        );
        if self.exec_stdin_request_open(agent_session, exec_session) {
            self.navigator
                .agent
                .stdin_send_failed(agent_session, text, error);
        } else {
            self.navigator
                .agent
                .stdin_send_failed_after_close(agent_session, text.to_string());
        }
        self.should_render = true;
    }

    pub(crate) fn handle_exec_stdin_control_failed(
        &mut self,
        agent_session: &SessionId,
        exec_session: &str,
        control: ExecStdinControl,
        error: &str,
    ) {
        tracing::warn!(
            target: "tui117",
            session = &agent_session.value,
            exec_session,
            error,
            "control_exec_stdin {control:?} failed",
        );
        if self.exec_stdin_request_open(agent_session, exec_session) {
            self.navigator
                .agent
                .stdin_control_failed(agent_session, control, error);
            self.should_render = true;
        }
    }

    pub(crate) fn handle_exec_stdin_muted(
        &mut self,
        agent_session: &SessionId,
        exec_session: &str,
    ) {
        self.agent_view_store
            .mute_exec_stdin(agent_session, exec_session.to_string());
        self.handle_exec_stdin_dismissed(agent_session);
    }

    pub(crate) fn handle_exec_stdin_probe_failed(
        &mut self,
        agent_session: &SessionId,
        generation: u64,
        error: &str,
    ) {
        let current = self
            .agent_view_store
            .exec_stdin_generation_is_current(agent_session, generation);
        tracing::error!(
            target: "bug364",
            session = &agent_session.value,
            generation,
            current,
            error,
            "get_exec_stdin_request probe failed",
        );
        if current
            && self
                .agent_view_store
                .mark_exec_stdin_probe_failed(agent_session, error.to_string())
        {
            self.should_render = true;
        }
    }

    /// Route the TUI-117 Action variants. Called from
    /// `try_dispatch_exec_stdin` for actions it does not own.
    pub(crate) fn try_dispatch_exec_stdin_recovery(&mut self, action: &Action) -> bool {
        match action {
            Action::ExecStdinWriteFailed {
                agent_session,
                exec_session,
                text,
                error,
            } => self.handle_exec_stdin_write_failed(agent_session, exec_session, text, error),
            Action::ExecStdinControlFailed {
                agent_session,
                exec_session,
                control,
                error,
            } => {
                self.handle_exec_stdin_control_failed(agent_session, exec_session, *control, error)
            }
            Action::ExecStdinMuted {
                agent_session,
                exec_session,
            } => self.handle_exec_stdin_muted(agent_session, exec_session),
            Action::ExecStdinProbeFailed {
                agent_session,
                generation,
                error,
            } => self.handle_exec_stdin_probe_failed(agent_session, *generation, error),
            _ => return false,
        }
        true
    }
}
