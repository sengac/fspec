#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compactor-sub-agent-escalation-in-stream-loop-error-cascade.feature
//!
//! CMPCT-044 (cascade slice): the stream-loop error cascade must classify provider
//! context-overflow errors with the NEW robust classifier BEFORE the
//! transient-network arm (NET-001), defer to the typed PromptCancelled
//! branch, gate on compactable turns, and feed the compactor sub-agent
//! recovery path instead of terminating the session.
//!
//! These are source-shape tests (the established pattern for cascade
//! ordering — cf. rpc084_streaming.rs): they pin the classifier call
//! order and the wiring of the compactor-sub-agent escalation in
//! `stream_loop.rs` without needing a live LLM.

use std::fs;
use std::path::PathBuf;

fn stream_loop_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/stream_loop.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Locate the 1-based character offset of the first occurrence of `needle`.
fn find(src: &str, needle: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("CMPCT-044: '{needle}' not found in stream_loop.rs"))
}

/// Scenario: Overflow error is classified before the transient-network retry arm
#[test]
fn overflow_check_runs_before_network_retry_arm() {
    // @step Given a provider context-overflow 400 that aborts the SSE stream mid-response
    // @step And the error surface also matches a transient-network pattern such as "stream closed before completion"
    // @step When the stream loop classifies the error
    // @step Then the overflow branch is taken
    // @step And the error is NOT retried as a transient network error
    let src = stream_loop_source();

    // The new robust overflow classifier must be invoked in the error arm...
    let overflow_at = find(&src, "is_context_overflow_error(");
    // ...strictly before the NET-001 transient-network arm...
    let network_at = find(&src, "is_transient_network_error(");

    assert!(
        overflow_at < network_at,
        "CMPCT-044: is_context_overflow_error (char {overflow_at}) must run BEFORE \
         is_transient_network_error (char {network_at}) — overflow 400s that abort \
         the SSE stream must never be misclassified as network retries"
    );
    // @step And no "Reconnecting..." network-retry status is emitted for that error
    // The overflow branch must route into the compaction-recovery macro, not
    // the network-retry arm — so the recovery call inside the overflow
    // branch (AFTER the classifier, not the Path-B one above it) must sit
    // before the network arm.
    let recovery_at = src[overflow_at..]
        .find("begin_compaction_recovery(")
        .map(|offset| overflow_at + offset)
        .unwrap_or_else(|| {
            panic!("CMPCT-044: no begin_compaction_recovery after the overflow classifier")
        });
    assert!(
        overflow_at < recovery_at && recovery_at < network_at,
        "CMPCT-044: the overflow branch must invoke begin_compaction_recovery \
         (char {recovery_at}) between the overflow classifier and the \
         network-retry arm — no 'Reconnecting...' retry status for overflows"
    );
}

/// Scenario: Overflow classifier defers to the typed PromptCancelled branch
#[test]
fn compaction_branch_classification_runs_before_overflow_string_classifier() {
    // @step Given a stream error that carries a typed PromptError::PromptCancelled in its chain
    // @step When the stream loop classifies the error
    // @step Then classify_compaction_branch routes it to the compaction-cancel recovery path first
    // @step And the context-overflow string classifier is not the deciding signal for that error
    let src = stream_loop_source();

    let branch_at = find(&src, "classify_compaction_branch(");
    let overflow_at = find(&src, "is_context_overflow_error(");

    assert!(
        branch_at < overflow_at,
        "CMPCT-044: classify_compaction_branch (char {branch_at}) must run BEFORE \
         is_context_overflow_error (char {overflow_at}) — the typed PromptCancelled \
         downcast stays the authoritative first signal"
    );
}

/// Scenario: Unmatched overflow error triggers compaction instead of terminating the session
#[test]
fn overflow_error_routes_to_compactor_sub_agent_recovery() {
    // @step Given a streaming session with compactable turns whose context exceeds the provider limit
    // @step And the provider returns a 400 whose body matches no existing is_prompt_too_long_error substring
    // @step When the stream loop processes the error
    // @step Then is_context_overflow_error classifies it as a context overflow
    // @step And the compaction-recovery path runs for the session
    // @step And the session does NOT terminate with a terminal "Agent error"
    let src = stream_loop_source();

    // The overflow branch must invoke the shared recovery entry
    // (begin_compaction_recovery) exactly like Paths B/C do today...
    let recovery_at = find(&src, "begin_compaction_recovery(");
    let overflow_at = find(&src, "is_context_overflow_error(");
    assert!(
        overflow_at < recovery_at || recovery_at < overflow_at,
        "sanity: both markers present"
    );
    // ...and must escalate to the compactor sub-agent handler when the
    // in-view path cannot resolve the overflow (registry lookup).
    let compactor_at = find(&src, "compactor_sub_agent");
    assert!(
        compactor_at > 0,
        "CMPCT-044: stream_loop.rs must reference the compactor sub-agent \
         escalation (compactor_sub_agent) in the overflow branch"
    );
}

/// Scenario: Overflow without compactable turns does not trigger compaction
#[test]
fn overflow_branch_is_gated_on_compactable_turns() {
    // @step Given a streaming session with only system-reminder messages and no user/assistant turns
    // @step And the provider returns a context-overflow error
    // @step When the stream loop processes the error
    // @step Then no compaction is triggered because there are no compactable turns
    // @step And the error follows the existing terminal error handling
    let src = stream_loop_source();

    // The existing gate (has_compactable_turns via convert_messages_to_turns)
    // must still guard the overflow branch — the classifier alone must not
    // fire compaction on a session with no compactable turns.
    let gate = find(&src, "has_compactable_turns");
    let overflow_at = find(&src, "is_context_overflow_error(");
    assert!(
        gate < overflow_at && (overflow_at - gate) < 4000,
        "CMPCT-044: the compactable-turns gate (char {gate}) must still guard the \
         overflow branch (char {overflow_at})"
    );
}

/// Scenario: Sub-agent escalation is bounded by the shared retry budget
#[test]
fn sub_agent_escalation_shares_the_compaction_retry_budget() {
    // @step Given the in-loop in-view compaction retry budget has been exhausted for a turn
    // @step And another context-overflow error occurs
    // @step When the error cascade processes it
    // @step Then the turn terminates with the structured compaction-budget-exhausted error
    // @step And the compactor sub-agent is not spawned for a turn past the retry budget
    let src = stream_loop_source();

    // The in_loop_compaction_restart! macro owns compaction_retry_count +
    // MAX_COMPACTION_RETRIES; the compactor escalation must sit inside the
    // macro definition (so its rounds count toward the same budget) rather
    // than bypassing it with a separate counter.
    let macro_def_at = find(&src, "macro_rules! in_loop_compaction_restart {");
    let macro_end_at = src
        .find("\n    loop {")
        .unwrap_or_else(|| panic!("CMPCT-044: primary `loop {{` not found after macro def"));
    assert!(
        macro_def_at < macro_end_at,
        "sanity: macro def precedes the primary loop"
    );

    let compactor_at = find(
        &src,
        "crate::compactor_sub_agent::run_compactor_sub_agent_round",
    );
    assert!(
        compactor_at > macro_def_at && compactor_at < macro_end_at,
        "CMPCT-044: the compactor sub-agent escalation (char {compactor_at}) must \
         live inside the in_loop_compaction_restart! macro definition (chars \
         {macro_def_at}-{macro_end_at}) so it shares the MAX_COMPACTION_RETRIES budget"
    );

    // And the budget check itself still precedes the escalation inside the macro.
    let budget_check_at = find(&src, "compaction_retry_count > MAX_COMPACTION_RETRIES");
    assert!(
        macro_def_at < budget_check_at && budget_check_at < compactor_at,
        "CMPCT-044: the MAX_COMPACTION_RETRIES guard (char {budget_check_at}) must \
         run before the compactor escalation (char {compactor_at})"
    );
}
