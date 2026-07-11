#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/auto-continue-engine.feature
//!
//! CONT-002: Auto-continue engine — session state, pure decision function
//! (doc §5 table), budget/refund arithmetic, and stream-loop wiring shape.
//! These tests import the REAL production API from
//! codelet_cli::interactive::auto_continue — no copies, no mocks
//! (PROV-041 test pattern, see thinking_exhaustion_recovery_test.rs).

use codelet_cli::interactive::auto_continue::{
    apply_segment_outcome, build_continue_exhaustion_warning, decide_continuation,
    reset_for_new_user_turn, ContinueDecision, AUTO_CONTINUE_NUDGE_PROMPT, DEFAULT_CONTINUE_BUDGET,
};
use codelet_cli::session::Session;
use rig::tool::Tool;

/// Build a Session without depending on a single-credential environment:
/// pick the first explicitly-selectable provider (mirrors
/// compaction_trigger_reliability_test.rs's ProviderManager usage).
fn fresh_session() -> Session {
    for name in ["claude", "openai", "gemini", "codex", "zai"] {
        if let Ok(pm) = codelet_providers::ProviderManager::with_provider(name) {
            return Session::from_provider_manager(pm);
        }
    }
    Session::new(None).expect("failed to create test session")
}

// =============================================================================
// Scenario: Session auto-continue state defaults and per-user-turn reset
// =============================================================================

#[test]
fn session_auto_continue_state_defaults_and_per_user_turn_reset() {
    // @step Given a newly constructed Session
    let mut session = fresh_session();

    // @step Then auto-continue is disabled by default
    assert!(
        !session.continue_enabled,
        "continue_enabled must default to false"
    );

    // @step And the continue budget defaults to 10
    assert_eq!(DEFAULT_CONTINUE_BUDGET, 10, "default budget constant is 10");
    assert_eq!(
        session.continue_budget, DEFAULT_CONTINUE_BUDGET,
        "continue_budget must default to DEFAULT_CONTINUE_BUDGET (10)"
    );

    // @step And the zero-progress nudge count defaults to 0
    assert_eq!(
        session.continue_nudges_used, 0,
        "continue_nudges_used must default to 0"
    );

    // @step When a session that has used 5 nudges begins a new real user turn
    session.continue_nudges_used = 5;
    reset_for_new_user_turn(&mut session);

    // @step Then the zero-progress nudge count is reset to 0
    assert_eq!(
        session.continue_nudges_used, 0,
        "reset_for_new_user_turn must zero continue_nudges_used"
    );
}

// =============================================================================
// Scenario: Off mode finishes exactly as today when the model stops
// =============================================================================

#[test]
fn off_mode_finishes_exactly_as_today_when_the_model_stops() {
    // @step Given auto-continue is off
    let armed = false;

    // @step When the model stops with stop_reason "stop" and no accepted done() call
    let decision = decide_continuation(armed, None, Some("stop"), 0, 10, false);

    // @step Then the continuation decision is Finish
    assert_eq!(decision, ContinueDecision::Finish);

    // @step And no nudge and no warning are produced
    assert_ne!(decision, ContinueDecision::Nudge);
    assert!(!matches!(decision, ContinueDecision::FinishWithWarning(_)));
}

// =============================================================================
// Scenario: Armed stop without done() produces a counted nudge
// =============================================================================

#[test]
fn armed_stop_without_done_produces_a_counted_nudge() {
    // @step Given auto-continue is armed with budget 10 and 0 nudges used
    let (armed, nudges_used, budget) = (true, 0u32, 10u32);

    // @step When the model stops with stop_reason "stop", "end_turn", or no stop_reason and no accepted done() call
    for stop_reason in [Some("stop"), Some("end_turn"), None] {
        let decision = decide_continuation(armed, None, stop_reason, nudges_used, budget, false);

        // @step Then the continuation decision is Nudge
        assert_eq!(
            decision,
            ContinueDecision::Nudge,
            "stop_reason {stop_reason:?} without done() must nudge while budget remains"
        );
    }

    // @step And the nudge text tells the model to call done(summary) if complete or otherwise continue working
    assert!(
        AUTO_CONTINUE_NUDGE_PROMPT.contains("done(summary)"),
        "nudge prompt must instruct calling done(summary); got: {AUTO_CONTINUE_NUDGE_PROMPT:?}"
    );
    assert!(
        AUTO_CONTINUE_NUDGE_PROMPT.contains("continue working"),
        "nudge prompt must instruct continuing work; got: {AUTO_CONTINUE_NUDGE_PROMPT:?}"
    );
}

// =============================================================================
// Scenario: Accepted done() finishes the turn and surfaces the summary
// =============================================================================

#[test]
fn accepted_done_finishes_the_turn_and_surfaces_the_summary() {
    // @step Given auto-continue is armed
    let armed = true;

    // @step And the model called done with summary "Refactored parser; all tests green"
    let done_summary = Some("Refactored parser; all tests green");

    // @step When the model stops
    let decision = decide_continuation(armed, done_summary, Some("stop"), 3, 10, false);

    // @step Then the continuation decision is Finish with that summary surfaced
    assert_eq!(
        decision,
        ContinueDecision::FinishWithSummary("Refactored parser; all tests green".to_string())
    );

    // @step And no nudge is produced
    assert_ne!(decision, ContinueDecision::Nudge);
}

// =============================================================================
// Scenario: Budget exhaustion finishes with a visible warning
// =============================================================================

#[test]
fn budget_exhaustion_finishes_with_a_visible_warning() {
    // @step Given auto-continue is armed with budget 2 and 2 nudges used
    let (armed, nudges_used, budget) = (true, 2u32, 2u32);

    // @step When the model stops with stop_reason "stop" and no accepted done() call
    let decision = decide_continuation(armed, None, Some("stop"), nudges_used, budget, false);

    // @step Then the continuation decision is Finish with a warning
    let ContinueDecision::FinishWithWarning(warning) = decision else {
        panic!("expected FinishWithWarning at budget exhaustion, got {decision:?}");
    };

    // @step And the warning line reports the model never called done() after 2 retries
    assert!(
        warning.contains("auto-continue"),
        "warning must name auto-continue; got: {warning:?}"
    );
    assert!(
        warning.contains("never called done()") && warning.contains("2"),
        "warning must report done() never called after 2 retries; got: {warning:?}"
    );
    let built = build_continue_exhaustion_warning(2);
    assert_eq!(
        built, warning,
        "builder must produce the decision's warning"
    );
}

// =============================================================================
// Scenario: User interrupt always wins over nudging
// =============================================================================

#[test]
fn user_interrupt_always_wins_over_nudging() {
    // @step Given auto-continue is armed with remaining budget
    let (armed, nudges_used, budget) = (true, 1u32, 10u32);

    // @step And the user has interrupted the stream
    let interrupted = true;

    // @step When the model stops without an accepted done() call
    let decision = decide_continuation(armed, None, Some("stop"), nudges_used, budget, interrupted);

    // @step Then the continuation decision is Finish
    assert_eq!(decision, ContinueDecision::Finish);

    // @step And no nudge is produced even though budget remains
    assert_ne!(decision, ContinueDecision::Nudge);
}

// =============================================================================
// Scenario: Truncation recovery takes precedence over auto-continue
// =============================================================================

#[test]
fn truncation_recovery_takes_precedence_over_auto_continue() {
    // @step Given auto-continue is armed with remaining budget
    let (armed, nudges_used, budget) = (true, 0u32, 10u32);

    // @step When the model stops with stop_reason "max_tokens" or "length" without an accepted done() call
    for stop_reason in [Some("max_tokens"), Some("length")] {
        let decision = decide_continuation(armed, None, stop_reason, nudges_used, budget, false);

        // @step Then the continuation decision is Finish
        assert_eq!(
            decision,
            ContinueDecision::Finish,
            "truncation stop {stop_reason:?} must never nudge"
        );
    }

    // @step And the existing truncation recovery remains responsible for that stop
    // (decide_continuation defers: PROV-040/041 recovery runs earlier in the
    // FinalResponse arm and is unchanged — pinned by the wiring-shape scenario.)
}

// =============================================================================
// Scenario: A nudge followed by tool activity is refunded
// =============================================================================

#[test]
fn a_nudge_followed_by_tool_activity_is_refunded() {
    // @step Given auto-continue is armed and one nudge was just consumed
    let nudges_used = 1u32;

    // @step When the segment following the nudge produces at least one tool call
    let after_tool_activity = apply_segment_outcome(nudges_used, 2);

    // @step Then that nudge is refunded and does not consume budget
    assert_eq!(
        after_tool_activity, 0,
        "a nudge followed by tool activity must be refunded"
    );

    // @step And a following segment with no tool calls keeps the nudge counted
    let after_zero_progress = apply_segment_outcome(nudges_used, 0);
    assert_eq!(
        after_zero_progress, 1,
        "a zero-progress segment must keep the nudge counted"
    );
}

// =============================================================================
// Scenario: Stream loop nudges only at the clean FinalResponse settle point
// =============================================================================

#[test]
fn stream_loop_nudges_only_at_the_clean_final_response_settle_point() {
    // @step Given the stream loop settle points
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/stream_loop.rs"
    ))
    .expect("stream_loop.rs must be readable");

    // @step Then the continuation decision is consulted only before the FinalResponse emit_done_with_stop_reason
    let occurrences = source.matches("decide_continuation").count();
    assert!(
        occurrences >= 1,
        "stream_loop.rs must consult decide_continuation at the FinalResponse settle point"
    );
    let decision_pos = source
        .find("decide_continuation")
        .expect("decide_continuation call site");
    let settle_pos = source
        .rfind("output.emit_done_with_stop_reason(final_stop_reason.take())")
        .expect("clean FinalResponse emit site");
    assert!(
        decision_pos < settle_pos,
        "decide_continuation must run BEFORE the clean FinalResponse emit"
    );

    // @step And the interruption, stall-timeout, and error emit sites never consult the continuation decision
    assert_eq!(
        occurrences, 1,
        "exactly one decide_continuation consultation is allowed (the clean \
         FinalResponse settle point); interrupt/stall/error sites must not nudge"
    );

    // @step And a Nudge decision reuses the PROV-041 re-prompt recipe and counts the nudge on the session
    assert!(
        source.contains("AUTO_CONTINUE_NUDGE_PROMPT"),
        "stream_loop.rs must inject the shared nudge prompt constant"
    );
    assert!(
        source.contains("continue_nudges_used"),
        "stream_loop.rs must count nudges on the session"
    );
}

// =============================================================================
// Scenario: done() tool records a session-scoped Tier-0 acceptance
// (moved from codelet/tools/tests — codelet-cli depends on codelet-tools, and
// the spec workflow enforces one test file per feature file)
// =============================================================================

#[tokio::test]
async fn done_tool_records_a_session_scoped_tier0_acceptance() {
    // @step Given a done tool bound to a session
    let session_id = uuid::Uuid::new_v4();
    let tool = codelet_tools::DoneTool::new(session_id);
    codelet_tools::set_continue_armed(session_id, true);

    // @step When the model calls done with summary "task complete"
    let result = tool
        .call(codelet_tools::DoneArgs {
            summary: "task complete".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;
    assert!(
        result.is_ok(),
        "non-empty summary is accepted at face value (Tier 0)"
    );

    // @step Then the acceptance and summary are readable for that session and cleared once taken
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id).as_deref(),
        Some("task complete"),
        "acceptance must be recorded for this session"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "acceptance must be cleared once taken"
    );
    let other_session = uuid::Uuid::new_v4();
    assert_eq!(
        codelet_tools::take_done_acceptance(other_session),
        None,
        "acceptance must be session-scoped"
    );

    // @step And calling done with an empty summary is rejected without recording acceptance
    let rejected = tool
        .call(codelet_tools::DoneArgs {
            summary: String::new(),
            evidence: None,
            goal_assessment: None,
        })
        .await;
    assert!(rejected.is_err(), "empty summary must be rejected");
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "rejected call must not record acceptance"
    );

    // @step And a stale done call arriving while auto-continue is off is accepted inertly without error
    codelet_tools::set_continue_armed(session_id, false);
    let stale = tool
        .call(codelet_tools::DoneArgs {
            summary: "late but harmless".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;
    assert!(
        stale.is_ok(),
        "stale done() after toggle-off must never error"
    );
}

// =============================================================================
// Scenario: done() is registered only while auto-continue is armed
// (moved from codelet/providers/tests — source-shape sweep per the
// rpc082/083/085 precedent; paths are relative to the cli crate root)
// =============================================================================

/// The seven production builder chains (doc §3 registration list).
const PROVIDER_CHAINS: &[&str] = &[
    "../providers/src/claude.rs",
    "../providers/src/openai.rs",
    "../providers/src/gemini.rs",
    "../providers/src/zai.rs",
    "../providers/src/codex/mod.rs",
    "../providers/src/copilot/rig_agent.rs",
    "../providers/src/custom/custom_provider.rs",
];

fn chain_source(rel: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

#[test]
fn done_is_registered_only_while_auto_continue_is_armed() {
    // @step Given a provider agent is built for a session marked armed
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    assert!(
        codelet_tools::is_continue_armed(session_id),
        "armed registry must report the session as armed"
    );

    // @step Then the agent tool set includes the done tool
    // (source-shape: every chain conditionally adds DoneTool when armed)
    for chain in PROVIDER_CHAINS {
        let source = chain_source(chain);
        assert!(
            source.contains("DoneTool"),
            "{chain} must register DoneTool in its create_rig_agent chain"
        );
    }

    // @step And an agent built for an unarmed session does not include the done tool
    codelet_tools::set_continue_armed(session_id, false);
    assert!(
        !codelet_tools::is_continue_armed(session_id),
        "disarming must clear the armed registry entry"
    );

    // @step And all 7 provider builder chains register the done tool conditionally on the armed state
    for chain in PROVIDER_CHAINS {
        let source = chain_source(chain);
        assert!(
            source.contains("is_continue_armed"),
            "{chain} must gate DoneTool registration on is_continue_armed(session_id)"
        );
    }

    // @step And the DeepSearch sub-agent toolset never includes the done tool
    let deep_search = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../agent-loop/src/deep_search_handler.rs"
    ))
    .expect("deep_search_handler.rs must be readable");
    assert!(
        !deep_search.contains("DoneTool"),
        "DeepSearch sub-agent toolset must never include done()"
    );
}
