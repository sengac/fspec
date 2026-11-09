//! TOOL-022 P2 — per-session inline exec-stdin prompt slot held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/exec-stdin-prompt.feature
//!
//! The exec-stdin prompt is a NON-BLOCKING overlay in the same composer
//! input area as HITL. Unlike HITL it carries NO response channel and
//! performs NO status flip: it is a pure overlay. The slot is populated
//! by push chunks and generation-checked asynchronous probe results.
//! Empty slots retain their generation to reject delayed snapshots.
//! The AgentView paints the inline prompt from this slot
//! only when the session is FOCUSED, and the slot is cleared on submit /
//! dismiss / focus-loss / re-probe on None.
//!
//! The slot is ephemeral and per-agent-session isolated: multiple
//! sessions can show exec-stdin prompts independently, and a HITL prompt
//! always wins over the exec-stdin slot for the same session (the render
//! precedence chain in `input_area.rs` is HITL > exec-stdin > pause >
//! composer).

use std::collections::{HashMap, HashSet};

use codelet_rpc_types::{ExecStdinRequest, SessionId};

use super::AgentViewStore;
use crate::terminal::sanitize::sanitize_for_terminal;

/// Slot map type held by [`AgentViewStore`].
pub type ExecStdinBySession = HashMap<SessionId, ExecStdinState>;

#[derive(Debug, Default)]
pub struct ExecStdinState {
    generation: u64,
    request: Option<ExecStdinRequest>,
    /// TUI-117: exec sessions (command runs) muted by Ctrl+X.
    muted: HashSet<String>,
    /// BUG-364: the current probe failed while a request was open; the
    /// notice is possibly stale until the next set / clear.
    probe_error: Option<String>,
}

impl ExecStdinState {
    fn advance(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }
}

impl AgentViewStore {
    /// Read the active exec-stdin prompt for `session`. `None` when no
    /// request is pending (or it was submitted / dismissed / cleared).
    pub fn exec_stdin_for(&self, session: &SessionId) -> Option<&ExecStdinRequest> {
        self.exec_stdin_by_session.get(session)?.request.as_ref()
    }

    pub(crate) fn advance_exec_stdin_generation(&mut self, session: &SessionId) -> u64 {
        self.exec_stdin_by_session
            .entry(session.clone())
            .or_default()
            .advance()
    }

    pub(crate) fn exec_stdin_generation_is_current(
        &self,
        session: &SessionId,
        generation: u64,
    ) -> bool {
        self.exec_stdin_by_session
            .get(session)
            .is_some_and(|state| state.generation == generation)
    }

    /// Persist a fetched [`ExecStdinRequest`] for `session`. Overwrites
    /// any prior slot (a fresh detector fire = fresh quiet_seconds).
    ///
    /// **TUI-111**: the command display string is sanitized on ingress
    /// so the exec-stdin prompt always paints clean text.
    pub fn set_exec_stdin(&mut self, session: SessionId, request: ExecStdinRequest) {
        let clean = ExecStdinRequest {
            command: sanitize_for_terminal(&request.command),
            ..request
        };
        let state = self.exec_stdin_by_session.entry(session).or_default();
        state.advance();
        state.request = Some(clean);
        state.probe_error = None;
    }

    /// Drop the exec-stdin slot for `session` — called after a
    /// successful submit, on Esc dismiss, and when a re-probe returns
    /// `None` (the exec session no longer exists / is no longer quiet).
    pub fn clear_exec_stdin(&mut self, session: &SessionId) {
        let state = self
            .exec_stdin_by_session
            .entry(session.clone())
            .or_default();
        state.advance();
        state.request = None;
        state.probe_error = None;
    }

    /// BUG-364: mark `session`'s open request as possibly stale because its
    /// current probe failed. Returns false (nothing recorded) when no
    /// request is open.
    pub(crate) fn mark_exec_stdin_probe_failed(
        &mut self,
        session: &SessionId,
        error: String,
    ) -> bool {
        match self.exec_stdin_by_session.get_mut(session) {
            Some(state) if state.request.is_some() => {
                state.probe_error = Some(error);
                true
            }
            _ => false,
        }
    }

    /// BUG-364: the failure of the current probe for `session`'s open
    /// request, if any.
    pub fn exec_stdin_probe_error(&self, session: &SessionId) -> Option<&str> {
        self.exec_stdin_by_session
            .get(session)?
            .probe_error
            .as_deref()
    }

    /// TUI-117: mute further requests from `exec_session` (one command
    /// run) for `session`.
    pub fn mute_exec_stdin(&mut self, session: &SessionId, exec_session: String) {
        self.exec_stdin_by_session
            .entry(session.clone())
            .or_default()
            .muted
            .insert(exec_session);
    }

    /// TUI-117: true when `exec_session` was muted for `session`.
    pub fn exec_stdin_is_muted(&self, session: &SessionId, exec_session: &str) -> bool {
        self.exec_stdin_by_session
            .get(session)
            .is_some_and(|state| state.muted.contains(exec_session))
    }
}
