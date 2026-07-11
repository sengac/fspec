#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/done-immediate-termination.feature
//!
//! CONT-005: done() immediate termination — Option D.
//!
//! An accepted done() must terminate the stream loop at the ToolResult arm
//! (identified via the DONE_ACCEPTANCE registry, never tool-name matching),
//! with the FinalResponse-arm check kept as fallback. Both exit sites run ONE
//! shared FinishWithSummary teardown helper. CONT-005's original goal-mode
//! deferral (do not consume the acceptance while a goal is active) was an
//! explicit assumption superseded by CONT-006: the early exit now fires in
//! goal mode too, and the shared helper performs the full atomic goal
//! teardown (see cont006_goal_immediate_termination.rs).
//!
//! Test surfaces (CONT-002 precedent, see auto_continue_engine_test.rs):
//! - behavioral tests on the REAL production decision/teardown helpers in
//!   codelet_cli::interactive::done_early_exit plus the real codelet_tools
//!   registry (no mocks; RecordingOutput is a callback event sink only), and
//! - source-shape tests on stream_loop.rs (rpc082/083 precedent) pinning the
//!   wiring order that cannot be driven without a live provider stream.

use std::sync::Mutex;

use codelet_cli::interactive::done_early_exit::{
    apply_finish_with_summary, decide_tool_result_early_exit, DONE_EARLY_EXIT_STOP_REASON,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_tools::{DoneArgs, DoneTool, GoalSpec};
use rig::tool::Tool;

/// Recording StreamOutput — callback event sink so tests can assert on the
/// emitted status lines (vi.fn()-style sink, not a module replacement).
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn statuses(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Status(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Build a Session without depending on a single-credential environment
/// (mirrors auto_continue_engine_test.rs::fresh_session).
fn fresh_session() -> Session {
    for name in ["claude", "openai", "gemini", "codex", "zai"] {
        if let Ok(pm) = codelet_providers::ProviderManager::with_provider(name) {
            return Session::from_provider_manager(pm);
        }
    }
    Session::new(None).expect("failed to create test session")
}

fn stream_loop_source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/stream_loop.rs"
    ))
    .expect("stream_loop.rs must be readable")
}

/// The `StreamUserItem(ToolResult)` arm region of stream_loop.rs — from the
/// arm's pattern to the start of the next (`Usage`) arm.
fn tool_result_arm_region(source: &str) -> &str {
    let start = source
        .find("StreamedUserContent::ToolResult")
        .expect("stream_loop.rs must have a StreamUserItem(ToolResult) arm");
    let end = source[start..]
        .find("MultiTurnStreamItem::Usage")
        .expect("Usage arm must follow the ToolResult arm")
        + start;
    &source[start..end]
}

// ============================================================================
// Scenario: Accepted done() terminates the turn at the ToolResult arm
// ============================================================================

#[tokio::test]
async fn accepted_done_terminates_the_turn_at_the_tool_result_arm() {
    // @step Given auto-continue is armed for a session with no active goal
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    let mut session = fresh_session();
    assert!(session.goal.is_none(), "no goal active in this scenario");
    session.continue_nudges_used = 3;

    // @step And the model's done call with summary "Task complete" was accepted into the registry
    let tool = DoneTool::new(session_id);
    tool.call(DoneArgs {
        summary: "Task complete".to_string(),
        evidence: None,
        goal_assessment: None,
    })
    .await
    .expect("Tier-0 done() must be accepted while armed");

    // @step When the early-exit decision is consulted after the done tool result is processed
    assert!(session.goal.is_none(), "still no goal before the consult");
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));

    // @step Then the decision consumes the acceptance and returns the summary "Task complete"
    assert_eq!(
        decision.as_deref(),
        Some("Task complete"),
        "non-goal mode with a recorded acceptance must exit early"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "the acceptance must be consumed by the early exit"
    );

    // @step And the shared teardown surfaces "✓ done: Task complete" and resets the nudge counter
    let output = RecordingOutput::new();
    apply_finish_with_summary(&mut session, session_id, "Task complete", &output);
    assert_eq!(
        output.statuses(),
        vec!["✓ done: Task complete".to_string()],
        "non-goal teardown must surface the accepted summary as the closing line"
    );
    assert_eq!(
        session.continue_nudges_used, 0,
        "teardown must reset the zero-progress nudge counter"
    );

    // @step And the early exit terminates the turn with the literal stop_reason "done"
    assert_eq!(
        DONE_EARLY_EXIT_STOP_REASON, "done",
        "early termination uses the literal stop_reason 'done' (assumption 2)"
    );

    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Goal-mode acceptance exits at the ToolResult arm (deferral
// superseded by CONT-006)
// ============================================================================

#[tokio::test]
async fn goal_mode_acceptance_exits_at_the_tool_result_arm_deferral_superseded_by_cont006() {
    // @step Given a goal is active for the session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: None,
        }),
    );

    // @step And the model's done call was accepted into the registry
    let tool = DoneTool::new(session_id);
    tool.call(DoneArgs {
        summary: "All tests green".to_string(),
        evidence: Some(vec!["cargo test output".to_string()]),
        goal_assessment: Some("Every acceptance criterion is now covered".to_string()),
    })
    .await
    .expect("goal-mode done() with evidence and assessment must be accepted");

    // @step When the early-exit decision is consulted at the ToolResult arm
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));

    // @step Then the decision consumes the acceptance and returns the summary
    assert_eq!(
        decision.as_deref(),
        Some("All tests green"),
        "goal mode now exits at the ToolResult arm — CONT-005's deferral was \
         superseded by CONT-006"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "the acceptance must be consumed by the early exit"
    );

    // @step And the goal-mode teardown is delegated to the shared helper owned by CONT-006
    let helper_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/done_early_exit.rs"
    ))
    .expect("done_early_exit.rs must exist");
    assert!(
        helper_source.contains("apply_goal_acceptance"),
        "apply_finish_with_summary's goal branch must delegate to \
         goal::apply_goal_acceptance (atomic teardown asserted by CONT-006 tests)"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Rejected done() records no acceptance and never exits early
// ============================================================================

#[tokio::test]
async fn rejected_done_records_no_acceptance_and_never_exits_early() {
    // @step Given a goal is active for an armed session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: None,
        }),
    );

    // @step When the model calls done without evidence or a goal assessment
    let tool = DoneTool::new(session_id);
    let result = tool
        .call(DoneArgs {
            summary: "done!".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;

    // @step Then the done call is rejected as a tool error
    assert!(result.is_err(), "Tier 1 failure must reject the done() call");

    // @step And the early-exit decision finds no acceptance and the loop continues
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision, None,
        "a rejected done() records no acceptance, so no early exit fires"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Stale done() while auto-continue is off never exits early
// ============================================================================

#[tokio::test]
async fn stale_done_while_auto_continue_is_off_never_exits_early() {
    // @step Given auto-continue has been toggled off for the session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, false);

    // @step When the model calls done with summary "late but harmless"
    let tool = DoneTool::new(session_id);
    let result = tool
        .call(DoneArgs {
            summary: "late but harmless".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;

    // @step Then the call is acknowledged inertly without recording an acceptance
    assert!(
        result.is_ok(),
        "a stale done() after toggle-off must never error"
    );

    // @step And the early-exit decision finds no acceptance and the loop continues
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision, None,
        "no acceptance exists, so the turn ends exactly as today"
    );
}

// ============================================================================
// Scenario: FinalResponse fallback runs the identical shared teardown
// ============================================================================

#[test]
fn final_response_fallback_runs_the_identical_shared_teardown() {
    // @step Given an acceptance that survives to the FinalResponse settle point
    let source = stream_loop_source();

    // @step When the settle point decision is FinishWithSummary
    let finish_arm = source
        .find("ContinueDecision::FinishWithSummary")
        .expect("stream_loop.rs must match on ContinueDecision::FinishWithSummary");

    // @step Then the FinishWithSummary arm routes through the same shared teardown helper as the early exit
    let call_sites = source.matches("apply_finish_with_summary").count();
    assert_eq!(
        call_sites, 2,
        "exactly two apply_finish_with_summary call sites: the ToolResult-arm \
         early exit and the FinalResponse FinishWithSummary fallback"
    );
    let arm_end = source[finish_arm..]
        .find("ContinueDecision::FinishWithWarning")
        .expect("FinishWithWarning arm must follow FinishWithSummary")
        + finish_arm;
    assert!(
        source[finish_arm..arm_end].contains("apply_finish_with_summary"),
        "the FinishWithSummary arm body must call the shared teardown helper"
    );

    // @step And in goal mode the teardown announces the satisfied goal, clears the session goal, and clears the registry goal
    let session_id = uuid::Uuid::new_v4();
    let mut session = fresh_session();
    session.set_goal("make all tests pass", None);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: None,
        }),
    );
    session.continue_nudges_used = 4;
    let output = RecordingOutput::new();
    apply_finish_with_summary(&mut session, session_id, "All tests green", &output);
    assert_eq!(
        output.statuses(),
        vec!["🎯 goal satisfied: All tests green".to_string()],
        "goal-mode teardown must announce the satisfied goal"
    );
    assert!(session.goal.is_none(), "session goal must be auto-cleared");
    assert!(
        codelet_tools::get_session_goal(session_id).is_none(),
        "registry goal must be cleared"
    );
    assert_eq!(session.continue_nudges_used, 0, "nudge counter must reset");

    // @step And in non-goal mode the teardown surfaces "✓ done: <summary>" and resets the nudge counter
    let mut plain_session = fresh_session();
    plain_session.continue_nudges_used = 2;
    let plain_output = RecordingOutput::new();
    apply_finish_with_summary(&mut plain_session, session_id, "Refactor complete", &plain_output);
    assert_eq!(
        plain_output.statuses(),
        vec!["✓ done: Refactor complete".to_string()],
        "non-goal teardown must surface the summary as the closing line"
    );
    assert_eq!(plain_session.continue_nudges_used, 0, "nudge counter must reset");
}

// ============================================================================
// Scenario: Pending assistant text is preserved in history before the early
// break
// ============================================================================

#[test]
fn pending_assistant_text_is_preserved_in_history_before_the_early_break() {
    // @step Given the model streamed explanation text before calling done
    let source = stream_loop_source();
    let arm = tool_result_arm_region(&source);

    // @step When the early exit fires at the ToolResult arm
    let consult = arm
        .find("decide_tool_result_early_exit")
        .expect("the ToolResult arm must consult the early-exit decision");
    let early_exit_block = &arm[consult..];

    // @step Then the early-exit block flushes pending assistant text into message history before breaking
    let flush = early_exit_block
        .find("handle_final_response(&assistant_text, &mut session.messages)")
        .expect("the early-exit block must flush pending assistant text (interrupt-path pattern)");
    let brk = early_exit_block
        .find("break;")
        .expect("the early-exit block must break out of the stream loop");
    assert!(
        flush < brk,
        "assistant text must be flushed into history BEFORE the break"
    );

    // @step And the early-exit block processes turn annotations like the other clean-exit paths
    let annotations = early_exit_block
        .find("process_turn_annotations")
        .expect("the early-exit block must process turn annotations (clean-exit parity)");
    assert!(
        annotations < brk,
        "turn annotations must be processed BEFORE the break"
    );
}

// ============================================================================
// Scenario: Stream-loop wiring pins early-exit ordering and single-teardown
// invariants
// ============================================================================

#[test]
fn stream_loop_wiring_pins_early_exit_ordering_and_single_teardown_invariants() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let source = stream_loop_source();

    // @step Then the ToolResult arm consults the early-exit decision only after handle_tool_result has paired the tool messages
    let arm = tool_result_arm_region(&source);
    let handle_pos = arm
        .find("handle_tool_result")
        .expect("the ToolResult arm must call handle_tool_result");
    let consult_pos = arm
        .find("decide_tool_result_early_exit")
        .expect("the ToolResult arm must consult decide_tool_result_early_exit");
    assert!(
        handle_pos < consult_pos,
        "the early-exit consult must run AFTER handle_tool_result has pushed \
         both the tool_use and tool_result into session.messages"
    );
    assert!(
        arm.contains("take_done_acceptance"),
        "the early exit must identify done() via the DONE_ACCEPTANCE registry \
         (rig's ToolResult has no tool name)"
    );

    // @step And the loop-top interrupt check precedes the early-exit consultation
    let interrupt_pos = source
        .find("is_interrupted.load(Acquire)")
        .expect("the loop-top interrupt check must exist");
    let global_consult_pos = source
        .find("decide_tool_result_early_exit")
        .expect("stream_loop.rs must consult the early-exit decision");
    assert!(
        interrupt_pos < global_consult_pos,
        "the loop-top is_interrupted check must precede all chunk handling \
         including the early-exit consult"
    );

    // @step And the early-exit consultation precedes the FinalResponse settle point decision
    let settle_pos = source
        .find("decide_continuation")
        .expect("the FinalResponse settle point must consult decide_continuation");
    assert!(
        global_consult_pos < settle_pos,
        "the ToolResult-arm early exit must come before the FinalResponse \
         settle point in the chunk match"
    );

    // @step And the early exit emits done with the shared stop_reason constant and breaks
    let early_exit_block = &arm[consult_pos..];
    let emit = early_exit_block
        .find("emit_done_with_stop_reason")
        .expect("the early exit must emit done");
    assert!(
        early_exit_block.contains("DONE_EARLY_EXIT_STOP_REASON"),
        "the early exit must use the shared stop_reason constant"
    );
    let brk = early_exit_block
        .find("break;")
        .expect("the early exit must break");
    assert!(emit < brk, "emit_done must run before the break");

    // @step And the done summary status formatting lives only in the shared teardown helper
    assert!(
        !source.contains("✓ done:"),
        "stream_loop.rs must not format the done summary inline — the \
         formatting lives only in done_early_exit::apply_finish_with_summary \
         so the two exit sites cannot diverge"
    );
    let helper_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/done_early_exit.rs"
    ))
    .expect("done_early_exit.rs must exist");
    assert!(
        helper_source.contains("✓ done:"),
        "the shared teardown helper must own the done summary formatting"
    );

    // @step And the stream loop never reuses CancelSignal for done() termination
    assert!(
        !source.contains("CancelSignal"),
        "CancelSignal routes into the compaction recovery cascade and must \
         never be used for done() termination"
    );
}
