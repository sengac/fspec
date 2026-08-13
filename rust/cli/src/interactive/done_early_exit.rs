//! CONT-005 — done() immediate termination (Option D).
//!
//! Feature: spec/features/done-immediate-termination.feature
//!
//! An accepted `done(summary)` used to be consumed only at the single
//! `FinalResponse` settle point (stream_loop.rs), but the patched rig-core
//! multi-turn loop unconditionally re-prompts the model after any turn that
//! executed tools (CONT-001) — and `done()` is itself a tool call, so an
//! accepted `done()` guaranteed at least one more model segment, making the
//! recorded summary stale.
//!
//! This module owns the two pieces the stream loop needs to terminate at the
//! `ToolResult` arm instead:
//!
//! - [`decide_tool_result_early_exit`] — the pure decision. done() is
//!   identified via the `DONE_ACCEPTANCE` registry (rig's `ToolResult` carries
//!   no tool name). CONT-006 lifted CONT-005's goal-mode deferral: an accepted
//!   done() now consumes the acceptance and exits early in goal mode too, so
//!   the passed Tier-2 verify can never be invalidated by post-acceptance
//!   work, re-run by a second done(), or have its summary overwritten.
//! - [`apply_finish_with_summary`] — the ONE shared FinishWithSummary
//!   teardown used by BOTH exit sites (the ToolResult-arm early exit and the
//!   FinalResponse fallback) so they cannot diverge. Its goal branch performs
//!   the full atomic goal teardown (CONT-006); CONT-008 marks the
//!   goal-clearing snapshot with `ContinueStateReason::GoalSatisfied` so the
//!   background twins perform the chrome/BackgroundSession goal write-back.
//!
//! Immediate break is history-safe: rig executes tools strictly sequentially
//! (each ToolCall is yielded, executed inline, and its ToolResult yielded
//! before the next provider chunk — rig streaming.rs:566-691), and
//! `handle_tool_result` pushes both the assistant tool_use and the user
//! tool_result into `session.messages` before the consult point. CancelSignal
//! is never reused — it routes into the compaction recovery cascade.

use uuid::Uuid;

use super::output::StreamOutput;
use crate::session::Session;

/// Literal stop_reason emitted when done() terminates the turn early.
///
/// Chosen over a `"stop"` passthrough: `StreamEvent::Done` consumers only
/// special-case `"max_tokens"` (CLI truncation warning) and otherwise treat
/// the value as opaque persisted metadata, so a literal `"done"` cannot
/// misfire truncation handling and records WHY the turn ended.
pub const DONE_EARLY_EXIT_STOP_REASON: &str = "done";

/// Pure decision for the ToolResult-arm early exit.
///
/// Returns `Some(summary)` when the stream loop must run the
/// [`apply_finish_with_summary`] teardown and break immediately.
///
/// * `take_acceptance` — read-and-clear accessor for the session's
///   `DONE_ACCEPTANCE` entry (`codelet_tools::take_done_acceptance`).
///
/// CONT-006: the decision no longer gates on goal state. An accepted
/// goal-mode done() already passed Tier 1 + Tier 2 inside `DoneTool::call`,
/// so consuming it here and exiting immediately is what makes the verify
/// run at most once per accepted completion and closes the DONE_ACCEPTANCE
/// last-writer-wins overwrite window (a second done() is unreachable after
/// the break).
///
/// Empty/whitespace summaries are filtered defensively, mirroring
/// `decide_continuation`'s guard (DoneTool already rejects them at Tier 0).
pub fn decide_tool_result_early_exit(
    take_acceptance: impl FnOnce() -> Option<String>,
) -> Option<String> {
    take_acceptance().filter(|summary| !summary.trim().is_empty())
}

/// The shared FinishWithSummary teardown (doc: FinalResponse arm behavior).
///
/// Used by BOTH the ToolResult-arm early exit and the FinalResponse
/// FinishWithSummary fallback so the two sites cannot diverge:
///
/// - goal active (CONT-006 atomic goal teardown): announce the satisfied
///   goal via [`super::goal::apply_goal_acceptance`] — which clears
///   `session.goal`, resets `session.done_rejections`, and removes the
///   CompletionContract reminder via `Session::clear_goal` — then clear the
///   registry goal (also resetting the registry rejection count), and
/// - no goal: surface the accepted summary as the turn's closing line, and
/// - always: reset the zero-progress nudge counter and emit the CONT-007
///   `ContinueState` snapshot so the TUI bar resets on BOTH exit sites
///   (ToolResult-arm early exit and FinalResponse fallback).
///
/// CONT-008: the snapshot reason is picked BEFORE the branch —
/// `GoalSatisfied` when an active goal is being cleared, `DoneAccepted`
/// otherwise. The background twins translate `GoalSatisfied` into the
/// chrome/BackgroundSession goal write-back + `goalCleared: true` on the
/// wire, so the engine-side goal clear propagates OUT (stale 🎯 bar +
/// satisfied-goal resurrection fix).
pub fn apply_finish_with_summary<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    summary: &str,
    output: &O,
) {
    // CONT-008: capture the goal-cleared signal before the branch clears
    // the goal — this is the ONE emission both exit sites share.
    let reason = if session.goal.is_some() {
        super::output::ContinueStateReason::GoalSatisfied
    } else {
        super::output::ContinueStateReason::DoneAccepted
    };
    if session.goal.is_some() {
        // CONT-003: accepted done() — announce the satisfied goal, auto-clear
        // it (falls back to the toggle), reset rejections, and sync the
        // registry.
        let announcement = super::goal::apply_goal_acceptance(session, summary);
        output.emit_status(&announcement);
        codelet_tools::set_session_goal(session_id, None);
    } else {
        // Surface the accepted summary as the turn's closing line.
        output.emit_status(&format!("✓ done: {summary}"));
    }
    session.continue_nudges_used = 0;
    // CONT-007: bar reset — emitted AFTER the goal clear + counter reset
    // so the snapshot carries goal_active=false, nudges_used=0.
    super::continue_state::emit_continue_state(session, output, reason);
}
