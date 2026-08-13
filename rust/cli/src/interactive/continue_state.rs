//! CONT-007 — live continue/goal counter snapshot builders.
//!
//! Feature: spec/features/live-continue-status-indicator.feature
//!
//! The stream loop (and the shared done() teardown in
//! [`super::done_early_exit`]) emit a [`StreamEvent::ContinueState`]
//! snapshot at every counter transition so the TUI footer indicator can
//! paint the REAL `nudges_used` instead of a hard-coded 0:
//!
//! - turn start (post per-turn reset),
//! - refund settle (previously invisible everywhere),
//! - nudge consumption (replaces the per-nudge chat print),
//! - budget exhaustion, and
//! - accepted done() (both the ToolResult-arm early exit and the
//!   FinalResponse fallback — the emission lives inside
//!   `apply_finish_with_summary`, which both sites share).
//!
//! `/continue` and `/goal` state changes do NOT emit — no stream is
//! active when they apply; the TUI dispatch drops its stale live cache
//! and the next TurnStart emission re-syncs the bar.

use super::output::{ContinueStateEvent, ContinueStateReason, StreamEvent, StreamOutput};
use crate::session::Session;

/// Build the counter snapshot from the session's live state.
///
/// `effective_budget` is `max(explicit, 15)` while a goal is active
/// (Goal-mode display budget, CONT-003 doc §2) and the explicit
/// `/continue` budget otherwise — fixing the old nudging line that
/// printed `continue_budget` in goal mode.
pub fn continue_state_event(session: &Session, reason: ContinueStateReason) -> ContinueStateEvent {
    let goal_active = session.goal.is_some();
    let effective_budget = if goal_active {
        super::goal::effective_goal_budget(session)
    } else {
        session.continue_budget
    };
    ContinueStateEvent {
        enabled: session.continue_enabled,
        budget: session.continue_budget,
        nudges_used: session.continue_nudges_used,
        goal_active,
        effective_budget,
        // CONT-008: real rejection count for the TUI bare-/goal display.
        done_rejections: session.done_rejections,
        reason,
    }
}

/// Emit the snapshot for `session` into `output`.
pub fn emit_continue_state<O: StreamOutput + ?Sized>(
    session: &Session,
    output: &O,
    reason: ContinueStateReason,
) {
    output.emit(StreamEvent::ContinueState(continue_state_event(
        session, reason,
    )));
}
