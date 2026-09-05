//! TOOL-022 P2 — per-session inline exec-stdin prompt slot held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/exec-stdin-prompt.feature
//!
//! The exec-stdin prompt is a NON-BLOCKING overlay in the same composer
//! input area as HITL. Unlike HITL it carries NO response channel and
//! performs NO status flip: it is a pure overlay. The slot is populated
//! by the chunk-driven probe (the focused agent session's
//! `backend.get_exec_stdin_request`), which dispatches
//! `Action::ExecStdinPromptFetched` whose reducer writes the wire
//! request here. The AgentView paints the inline prompt from this slot
//! only when the session is FOCUSED, and the slot is cleared on submit /
//! dismiss / focus-loss / re-probe on None.
//!
//! The slot is ephemeral and per-agent-session isolated: multiple
//! sessions can show exec-stdin prompts independently, and a HITL prompt
//! always wins over the exec-stdin slot for the same session (the render
//! precedence chain in `input_area.rs` is HITL > exec-stdin > pause >
//! composer).

use std::collections::HashMap;

use codelet_rpc_types::{ExecStdinRequest, SessionId};

use super::AgentViewStore;

/// Slot map type held by [`AgentViewStore`].
pub type ExecStdinBySession = HashMap<SessionId, ExecStdinRequest>;

impl AgentViewStore {
    /// Read the active exec-stdin prompt for `session`. `None` when no
    /// request is pending (or it was submitted / dismissed / cleared).
    pub fn exec_stdin_for(&self, session: &SessionId) -> Option<&ExecStdinRequest> {
        self.exec_stdin_by_session.get(session)
    }

    /// Persist a fetched [`ExecStdinRequest`] for `session`. Overwrites
    /// any prior slot (a fresh detector fire = fresh quiet_seconds).
    pub fn set_exec_stdin(&mut self, session: SessionId, request: ExecStdinRequest) {
        self.exec_stdin_by_session.insert(session, request);
    }

    /// Drop the exec-stdin slot for `session` — called after a
    /// successful submit, on Esc dismiss, and when a re-probe returns
    /// `None` (the exec session no longer exists / is no longer quiet).
    pub fn clear_exec_stdin(&mut self, session: &SessionId) {
        self.exec_stdin_by_session.remove(session);
    }
}
