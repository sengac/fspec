//! RPC-093 + RPC-095 + BUG-194 — per-frame animation tick for
//! `AgentView`, split into a sibling module so `views/agent.rs` stays
//! under the 300-LoC source-shape ceiling while keeping canonical
//! rustfmt formatting.
//!
//! BUG-194: the tick is PER-SESSION — each rendered agent pane
//! advances its OWN session's `SessionTransition` slot from that
//! session's own `SessionStatus` (focused or not). Focus movement
//! never touches the slots: the only input to a session's transition
//! machine is that session's status.

use std::time::Instant;

use codelet_rpc_types::{SessionId, SessionStatus};

use super::session_transition::SessionTransition;
use super::transition_driver;
use super::AgentView;

impl AgentView {
    /// BUG-194: per-frame per-session animation tick. Advances `slot`
    /// (the session's own transition slot) from the session's own
    /// `session_status`. Returns `(session_status, is_loading)`.
    ///
    /// `is_focused` gates the view-level side effects: the COMPACTING-DIAG
    /// flip log and the `spinner_started_at` mirror that `is_busy`
    /// (RPC-093) reads.
    pub(super) fn tick_session_transition(
        &mut self,
        session_status: Option<SessionStatus>,
        sid: Option<&SessionId>,
        slot: &mut SessionTransition,
        is_focused: bool,
    ) -> (Option<SessionStatus>, bool) {
        let is_busy = matches!(
            session_status,
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting)
        );
        let is_loading = matches!(session_status, Some(SessionStatus::Running));
        // COMPACTING-DIAG: log the exact (status, display) decision that
        // picks "Thinking..." vs "Compacting..." — only for the FOCUSED
        // pane (the display decision the user is looking at) and only
        // when it changes, so the log shows every flip without
        // per-frame spam.
        if is_focused {
            let display_mode = if is_loading {
                "thinking"
            } else if is_busy {
                "compacting"
            } else {
                "idle"
            };
            if self.last_compaction_diag_display != display_mode {
                tracing::info!(
                    session_id = ?sid,
                    status = ?session_status,
                    display_mode,
                    "[compaction-status] TUI display decision: spinner shows {display_mode:?}"
                );
                self.last_compaction_diag_display = display_mode;
            }
        }
        // Per-session spinner clock: armed while the session is
        // Running/Compacting, cleared when it stops. Focus movement
        // NEVER resets it (BUG-194).
        if is_busy && slot.spinner_started_at.is_none() {
            slot.spinner_started_at = Some(Instant::now());
        } else if !is_busy {
            slot.spinner_started_at = None;
        }
        // RPC-093: mirror the focused pane's spinner clock into the
        // view-level field `is_busy` reads (shape test pins the fn).
        if is_focused {
            self.spinner_started_at = slot.spinner_started_at;
        }
        // Per-session frame clock — each pane's Hiding/Showing sweep
        // quantizes on its OWN 16ms/frame clock, so simultaneously
        // animating panes never speed each other up.
        slot.clock_ms = slot.clock_ms.saturating_add(16);
        let elapsed_ms = slot
            .spinner_started_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        slot.state = transition_driver::advance_transition(
            session_status,
            &slot.state,
            slot.last_spinner_line.as_deref(),
            elapsed_ms,
            slot.clock_ms,
        );
        (session_status, is_loading)
    }
}
