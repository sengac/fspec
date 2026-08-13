//! CMPCT-023 / CMPCT-024 / CMPCT-027: Unified compaction-recovery entry point
//! for the stream loop.
//!
//! This module owns the invariant-enforcing helpers that every "compact and
//! retry" entry point funnels through, so the behavior matrix (save partial
//! text, flush tracker, pop user prompt, set flag, clear progress callback,
//! emit lifecycle events, execute compaction, circuit-break retries) collapses
//! to a single column.
//!
//! ## Entry paths
//!
//! | Path | Site                                                    | `pop_user_prompt` |
//! |------|---------------------------------------------------------|-------------------|
//! | B    | `stream_loop.rs` "prompt is too long" API error         | `true`            |
//! | C    | `stream_loop.rs` hook-cancel (`PromptCancelled`)        | `true`            |
//! | D    | `gemini_continuation.rs` continuation cancel            | `false`           |
//!
//! Path A (pre-prompt compaction) is structurally separate — it runs BEFORE
//! streaming begins and does not need partial-state flushing — so it keeps
//! its bespoke call sequence in `stream_loop.rs:301-455`.
//!
//! ## Internal composition
//!
//! [`flush_partial_state_before_compaction`] (the CMPCT-024 helper) is kept as
//! a thin, separately-testable primitive. `begin_compaction_recovery` delegates
//! to it for the text-save + tracker-flush step, then handles the cross-cutting
//! responsibilities.
//!
//! [`execute_compaction_and_capture_events`] (the CMPCT-027 helper) performs
//! the actual DAG-condensation step plus debug-capture events. It is invoked
//! in-loop by the primary stream loop after `begin_compaction_recovery`, so
//! the new retry stream is subject to the same error cascade as the original
//! stream (BUG 5 is impossible by construction).
//!
//! ## Emission discipline
//!
//! `begin_compaction_recovery` emits `compaction_started` +
//! `compaction_progress("Context limit reached", 0, total_turns.max(1))`
//! exactly once per entry. `execute_compaction_and_capture_events` emits
//! `compaction_continuing` exactly once on success. Downstream callers must
//! NOT re-emit either of these lifecycle events.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::{ApiTokenUsage, StreamingTokenDisplay, TokenState};
use codelet_tools::set_tool_progress_callback;
use tracing::{debug, warn};
use uuid::Uuid;

use super::output::StreamOutput;
use super::stream_handlers::handle_final_response;
use crate::interactive_helpers::{compression_ratio, execute_compaction};
use crate::session::Session;

/// CMPCT-027: Maximum number of cascaded compaction attempts per user turn.
///
/// When the post-compaction stream yields another compaction-triggering error
/// (e.g. `prompt is too long`, `PromptCancelled`), the stream loop re-enters
/// the compaction-recovery path. This constant bounds how many times that can
/// happen before the session returns a terminal budget-exhausted error.
///
/// Follows the same shape as `recovery_network::MAX_NETWORK_RETRIES` and
/// `recovery_truncation::MAX_TRUNCATION_RETRIES`. Public for testing — tests
/// must import the real constant, not hardcode the value.
pub const MAX_COMPACTION_RETRIES: u32 = 3;

/// CMPCT-028: Resume-prompt policy selected by [`begin_compaction_recovery`]
/// for the in-loop compaction restart.
///
/// The in-loop restart in `stream_loop.rs` must send *some* prompt string to
/// rig after compaction. Before CMPCT-028 that prompt was the hardcoded
/// literal `"Continue"`, which was only meaningful because
/// [`crate::interactive_helpers::execute_compaction`] embeds the original
/// user prompt into the compaction instruction — the agent reads it out of
/// the instruction and resumes the task. When CMPCT-024 began preserving
/// partial assistant text mid-stream, that semantic became ambiguous:
/// `"Continue"` conflicts with the preserved Assistant message that now sits
/// at the tail of `session.messages`.
///
/// This enum makes the choice explicit. Callers of
/// [`begin_compaction_recovery`] receive the selected policy and thread it
/// into [`compaction_retry_prompt`] to obtain the rig-facing prompt string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionRecoveryPolicy {
    /// Compaction fired BEFORE any Assistant text was streamed and preserved.
    ///
    /// [`crate::interactive_helpers::execute_compaction`] embeds the original
    /// user prompt into the compaction instruction, so the retry prompt sent
    /// to rig is the literal `"Continue"` — rig does not double-send the
    /// embedded prompt.
    EmbedInInstruction,

    /// Compaction fired AFTER partial Assistant text was streamed and preserved
    /// via [`flush_partial_state_before_compaction`] (CMPCT-024).
    ///
    /// The preserved Assistant message is at the tail of
    /// `session.messages`, so the retry prompt explicitly asks the agent to
    /// resume from the preserved work rather than start over from the
    /// embedded instruction.
    ResumeFromPartial,
}

/// CMPCT-028: Map a [`CompactionRecoveryPolicy`] to the rig-facing prompt
/// string used by the in-loop compaction restart in `stream_loop.rs`.
///
/// Callers never synthesize the prompt string directly; they obtain it from
/// this helper so the mapping lives in one auditable place and the selected
/// policy can be logged alongside the prompt that actually goes to rig.
pub fn compaction_retry_prompt(policy: CompactionRecoveryPolicy) -> &'static str {
    match policy {
        CompactionRecoveryPolicy::EmbedInInstruction => "Continue",
        CompactionRecoveryPolicy::ResumeFromPartial => {
            "Please continue from where you left off before the context limit was reached."
        }
    }
}

/// CMPCT-027: Build the terminal error message surfaced to the user when the
/// compaction retry budget is exhausted.
///
/// The message mentions the numeric attempt count so the user understands
/// this is a bounded-retry budget, not a provider failure. The wording is
/// intentionally similar to `build_truncation_budget_exhausted_message` and
/// `build_stall_timeout_message` so the UX family stays consistent.
pub fn build_compaction_budget_exhausted_message(max_retries: u32) -> String {
    format!(
        "Compaction retry budget exhausted after {max_retries} attempts. \
         The conversation context still exceeds the provider limit even after \
         {max_retries} consecutive compaction rounds — the session cannot make \
         further automatic progress. Consider starting a fresh session or \
         removing large attachments before retrying."
    )
}

/// Flush partial streaming state onto the `Session` before a compaction break.
///
/// Responsibilities (all three happen even if the first one is a no-op):
/// - Append `assistant_text` as a trailing Assistant message via
///   `handle_final_response`, iff it is non-empty.
/// - Clear `assistant_text` so the buffer does not leak into any downstream
///   path that may re-read it (defensive).
/// - Snapshot the current `StreamingTokenDisplay` values and flush them to
///   `session.token_tracker` via `update_from_usage`, accumulating into the
///   `cumulative_billed_*` fields.
///
/// This function performs no I/O and emits no output events. It is safe to
/// call with an empty `assistant_text` and a zeroed display (in which case
/// only the cumulative tracker gets a no-op nudge of zeros).
///
/// # Returns
///
/// `true` iff a non-empty `assistant_text` was appended as an Assistant
/// message. Callers — notably [`begin_compaction_recovery`] — use this signal
/// to pick the correct [`CompactionRecoveryPolicy`] without having to
/// re-inspect `session.messages`.
///
/// # Errors
///
/// Only surfaces errors from `handle_final_response` (which today is
/// infallible for the simple text-append case, but the signature remains
/// fallible for forward-compatibility).
pub fn flush_partial_state_before_compaction(
    session: &mut Session,
    assistant_text: &mut String,
    display: &StreamingTokenDisplay,
) -> Result<bool> {
    // CMPCT-024 / BUG 2: preserve any text that was streamed to the user
    // before the hook cancel. Five of rig's six cancel sites can fire
    // AFTER text has been emitted.
    let appended = if !assistant_text.is_empty() {
        handle_final_response(assistant_text, &mut session.messages)?;
        assistant_text.clear();
        true
    } else {
        false
    };

    // CMPCT-024 / BUG 6: flush the token tracker so the pre-cancel usage
    // is billed. Without this, one turn of input/output tokens silently
    // disappears from `session.token_tracker` on every compaction cancel.
    let current = display.current();
    // TOKEN-001: compute per-turn delta via the single TokenTracker helper
    // BEFORE update_from_usage mutates self.output_tokens. Passing 0 here
    // (the previous behavior) silently dropped every per-turn output from
    // the cumulative billing accumulator.
    let per_turn_output_delta = session
        .token_tracker
        .compute_output_delta(current.output_tokens);
    let usage = ApiTokenUsage::new(
        current.input_tokens,
        current.cache_read_tokens,
        current.cache_creation_tokens,
        per_turn_output_delta,
    );
    session
        .token_tracker
        .update_from_usage(&usage, current.output_tokens);

    Ok(appended)
}

/// Unified entry point for all compaction-recovery paths.
///
/// Guarantees the following invariants regardless of caller:
/// 1. Partial `assistant_text` is saved via `handle_final_response` and the
///    buffer is cleared.
/// 2. The token tracker is updated from the current `StreamingTokenDisplay`.
/// 3. If `pop_user_prompt` is true, the last message is popped iff it is a
///    `Message::User` (defensive match). For Paths B and C the user prompt
///    is at the tail of `session.messages` but has not been consumed by the
///    API; for Path D the user message is mid-flight and must remain.
/// 4. `token_state.compaction_needed` is set to `true`. If it was already
///    `true` on entry (disagreement between the cancel path and the flag
///    bookkeeping), a structured `warn!` is emitted. The flag remains `true`
///    afterwards regardless.
/// 5. The global tool progress callback is cleared via
///    `set_tool_progress_callback(Uuid::nil(), None)` so a stale per-turn
///    callback cannot survive into the in-loop compaction restart (CMPCT-027).
/// 6. `output.emit_compaction_started()` and
///    `output.emit_compaction_progress("Context limit reached", 0,
///    total_turns.max(1))` are emitted exactly once, where `total_turns` is
///    `session.messages.len() / 2` (computed AFTER any pop so the discarded
///    prompt is not counted).
///
/// # Returns
///
/// A [`CompactionRecoveryPolicy`] that tells the caller which retry prompt
/// to send to rig after `execute_compaction_and_capture_events` runs:
/// - [`CompactionRecoveryPolicy::ResumeFromPartial`] when partial assistant
///   text was preserved by `flush_partial_state_before_compaction` — the
///   agent should pick up from the preserved Assistant message.
/// - [`CompactionRecoveryPolicy::EmbedInInstruction`] when no partial text
///   existed — the agent reads the embedded original prompt out of the
///   compaction instruction, so the rig prompt is the literal `"Continue"`.
///
/// Callers thread the returned policy into [`compaction_retry_prompt`] in
/// `stream_loop.rs` so the prompt string is chosen in exactly one place.
///
/// # Errors
///
/// Propagates errors from `flush_partial_state_before_compaction`
/// (see its doc comment).
///
/// # Order of operations
///
/// Steps are ordered so that:
/// - The trailing user prompt is popped BEFORE the partial assistant text
///   is appended. If we reversed this, the appended Assistant would sit at
///   the tail and the User (now second-to-last) would never be popped.
/// - Lifecycle events are emitted LAST so `total_turns` reflects the
///   post-pop + post-append length.
///
/// 1. Conditional user-pop (must run before flush)
/// 2. `flush_partial_state_before_compaction` (save text, flush tracker)
///    — captures whether partial text was appended for the returned policy
/// 3. Set `compaction_needed` (with warn-on-disagreement)
/// 4. Clear tool progress callback
/// 5. Emit lifecycle events
/// 6. Return the selected [`CompactionRecoveryPolicy`] (with a debug log)
pub fn begin_compaction_recovery<O: StreamOutput>(
    session: &mut Session,
    token_state: &Arc<Mutex<TokenState>>,
    streaming_display: &StreamingTokenDisplay,
    assistant_text: &mut String,
    output: &O,
    pop_user_prompt: bool,
) -> Result<CompactionRecoveryPolicy> {
    // Step 1: pop last user message when it is at the tail of session.messages
    // but has not been consumed by the API yet (Paths B and C). Path D passes
    // false because the continuation prompt is mid-flight. This MUST happen
    // before flushing partial text, otherwise the appended Assistant ends up
    // at the tail and the pop becomes a no-op.
    if pop_user_prompt {
        if let Some(last) = session.messages.last() {
            if matches!(last, rig::message::Message::User { .. }) {
                session.messages.pop();
                debug!(
                    "[begin_compaction_recovery] Popped trailing User message (pop_user_prompt=true)"
                );
            } else {
                debug!(
                    "[begin_compaction_recovery] pop_user_prompt=true but tail is not a User message; leaving messages unchanged"
                );
            }
        }
    }

    // Step 2: save partial text + flush token tracker. The boolean return
    // drives the CompactionRecoveryPolicy selection at the tail of this
    // function.
    let partial_text_saved =
        flush_partial_state_before_compaction(session, assistant_text, streaming_display)?;

    // Step 3: set compaction_needed, warning if it was already true
    // (indicates a disagreement between the cancel path and the flag
    // bookkeeping; this is defense-in-depth for CMPCT-026).
    match token_state.lock() {
        Ok(mut state) => {
            if state.compaction_needed {
                warn!(
                    "[begin_compaction_recovery] compaction_needed was already true on entry — \
                     signals that a previous path or the hook has already flagged this turn. \
                     The helper leaves the flag set and proceeds."
                );
            }
            state.compaction_needed = true;
        }
        Err(poisoned) => {
            // Mutex poisoning is non-fatal for our purposes: we still want to
            // set the flag. Recover via `into_inner()` and proceed.
            warn!(
                "[begin_compaction_recovery] token_state mutex was poisoned; recovering and \
                 setting compaction_needed=true anyway"
            );
            let mut state = poisoned.into_inner();
            state.compaction_needed = true;
        }
    }

    // Step 4: clear the global tool progress callback so a stale per-turn
    // handler cannot survive into the compaction retry stream.
    set_tool_progress_callback(Uuid::nil(), None);

    // Step 5: emit lifecycle events exactly once.
    output.emit_compaction_started();
    // `total_turns` is computed AFTER any pop so the discarded prompt is
    // not counted. saturating division by 2 gives the approximate
    // user/assistant pair count.
    let total_turns = (session.messages.len() as u32) / 2;
    output.emit_compaction_progress("Context limit reached", 0, total_turns.max(1));

    // Step 6: CMPCT-028 — select the retry-prompt policy based on whether
    // partial assistant text was preserved. `EmbedInInstruction` is the
    // pre-CMPCT-024 behavior (retry prompt is the literal `"Continue"`);
    // `ResumeFromPartial` is the new branch that references the preserved
    // Assistant message. We log the selection so operators can audit which
    // branch fired per turn.
    let policy = if partial_text_saved {
        CompactionRecoveryPolicy::ResumeFromPartial
    } else {
        CompactionRecoveryPolicy::EmbedInInstruction
    };
    debug!(
        policy = ?policy,
        partial_text_saved = partial_text_saved,
        "[begin_compaction_recovery] CompactionRecoveryPolicy selected (EmbedInInstruction vs ResumeFromPartial)"
    );

    Ok(policy)
}

/// CMPCT-027: Execute in-view DAG compaction and capture the lifecycle debug
/// events that the old `compaction_retry::handle_compaction_retry` used to
/// emit.
///
/// Called from the primary stream loop IN-LOOP after
/// [`begin_compaction_recovery`], so the new retry stream stays governed by
/// the same error classifier cascade as the original stream. This replaces
/// the obsolete `run_retry_stream` flow (BUG 5 — incomplete post-compaction
/// error handling).
///
/// ## Responsibilities
///
/// - Capture `compaction.triggered` debug event with pre-compaction usage.
/// - Call [`execute_compaction`] with the original user prompt so the DAG
///   instruction embeds a resume hint for the agent.
/// - On success: capture `context.update` debug event, emit
///   [`StreamOutput::emit_compaction_continuing`], and call
///   `session.token_tracker.reset_after_compaction()`.
/// - On failure: capture `compaction.failed` debug event, emit
///   [`StreamOutput::emit_compaction_failed`], and surface the error.
///
/// ## Emission discipline
///
/// The helper emits **only** `compaction_continuing` (or
/// `compaction_failed`). It does NOT emit `compaction_started` or
/// `compaction_progress` — those are the responsibility of
/// [`begin_compaction_recovery`], which runs upstream at every Path B/C/D
/// entry. Re-emitting them here would double-count the lifecycle events
/// (this was a pre-existing bug before CMPCT-023).
///
/// ## Arguments
///
/// - `session`: the session whose `messages` and `token_tracker` will be
///   reset in place.
/// - `compaction_in_progress`: the shared atomic flag that gates
///   SessionSearch Layer-0 trimming during DAG construction.
/// - `prompt`: the original user prompt that triggered the current turn.
///   `execute_compaction` embeds this into the compaction instruction so the
///   agent knows what to resume after calling `inject_summary`.
/// - `threshold`, `context_window`: forwarded into the `compaction.triggered`
///   debug event so downstream consumers can correlate the trigger with the
///   provider limits at the moment of the cancel.
/// - `token_state`: the shared `Arc<Mutex<TokenState>>` — only read to
///   populate the `compaction.triggered` event. This helper does NOT mutate
///   the flag; the caller is responsible for clearing `compaction_needed`
///   after the helper returns.
/// - `output`: the sink that receives `compaction_continuing` (or
///   `compaction_failed`) events.
///
/// # Errors
///
/// Returns the underlying `execute_compaction` error on failure. Callers are
/// expected to `?`-propagate it so the stream loop terminates cleanly with a
/// surfaced error.
pub async fn execute_compaction_and_capture_events<O: StreamOutput>(
    session: &mut Session,
    compaction_in_progress: Arc<AtomicBool>,
    prompt: &str,
    threshold: u64,
    context_window: u64,
    token_state: &Arc<Mutex<TokenState>>,
    output: &O,
) -> Result<()> {
    debug!(
        "[execute_compaction_and_capture_events] messages_len={}, approx_turns={}",
        session.messages.len(),
        session.messages.len() / 2
    );

    // Capture compaction.triggered event.
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                if let Ok(state) = token_state.lock() {
                    manager.capture(
                        "compaction.triggered",
                        serde_json::json!({
                            "timing": "hook-triggered",
                            "inputTokens": state.input_tokens,
                            "cacheReadInputTokens": state.cache_read_input_tokens,
                            "threshold": threshold,
                            "contextWindow": context_window,
                        }),
                        None,
                    );
                }
            }
        }
    }

    let original_tokens = session.token_tracker.input_tokens;

    match execute_compaction(session, compaction_in_progress, Some(prompt)).await {
        Ok(()) => {
            let compacted_tokens = session.token_tracker.input_tokens;
            let ratio = compression_ratio(original_tokens, compacted_tokens);

            // Capture context.update event after compaction.
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "context.update",
                            serde_json::json!({
                                "type": "compaction",
                                "originalTokens": original_tokens,
                                "compactedTokens": compacted_tokens,
                                "compressionRatio": ratio,
                            }),
                            None,
                        );
                    }
                }
            }

            output.emit_compaction_continuing();
            session.token_tracker.reset_after_compaction();
            Ok(())
        }
        Err(e) => {
            output.emit_compaction_failed(&format!("{e} - will retry on next turn"));

            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "compaction.failed",
                            serde_json::json!({
                                "error": e.to_string(),
                                "inputTokens": session.token_tracker.input_tokens,
                            }),
                            None,
                        );
                    }
                }
            }

            Err(anyhow::anyhow!("Compaction failed: {e}"))
        }
    }
}
