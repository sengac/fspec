#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/goal-immediate-termination.feature
//!
//! CONT-006: /goal immediate termination and atomic goal teardown on
//! accepted done().
//!
//! An accepted goal-mode done() (Tier 1 + Tier 2 passed inside
//! DoneTool::call) must terminate the stream at the ToolResult arm exactly
//! like non-goal mode: the CONT-005 goal-mode deferral gate is lifted
//! (decide_tool_result_early_exit loses the goal_active parameter) and the
//! shared apply_finish_with_summary teardown performs the full atomic goal
//! teardown (🎯 announcement, Session::clear_goal, registry clear,
//! nudge reset). Rejected done() still never exits early; settle-point
//! escalation semantics are unchanged.
//!
//! Test surfaces (CONT-005 precedent, cont005_done_immediate_termination.rs):
//! - behavioral tests on the REAL production helpers
//!   (done_early_exit / goal / auto_continue) plus the real codelet_tools
//!   registry (no mocks; RecordingOutput is a callback event sink only), and
//! - source-shape tests pinning the wiring that cannot be driven without a
//!   live provider stream (rpc082/083 precedent).

use std::sync::Mutex;

use codelet_cli::interactive::done_early_exit::{
    apply_finish_with_summary, decide_tool_result_early_exit, DONE_EARLY_EXIT_STOP_REASON,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::system_reminders::count_system_reminders_by_type;
use codelet_cli::session::{Session, SystemReminderType};
use codelet_tools::{DoneArgs, DoneTool, GoalSpec};
use rig::message::{
    AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent,
};
use rig::tool::Tool;
use rig::OneOrMany;

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
// Scenario: Accepted goal-mode done() terminates the turn at the ToolResult
// arm with atomic teardown
// ============================================================================

#[tokio::test]
async fn accepted_goal_mode_done_terminates_the_turn_at_the_tool_result_arm_with_atomic_teardown() {
    // @step Given a goal "make all tests pass" is active for an armed session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: None,
        }),
    );
    let mut session = fresh_session();
    session.set_goal("make all tests pass", None);
    session.continue_nudges_used = 3;
    assert_eq!(
        count_system_reminders_by_type(&session.messages, SystemReminderType::CompletionContract),
        1,
        "setting the goal must inject the CompletionContract reminder"
    );

    // Accumulate a rejection on both counters so the teardown's reset is
    // observable (registry via a Tier-1 rejected call; session mirror
    // manually, as the settle-point sync would).
    let tool = DoneTool::new(session_id);
    let rejected = tool
        .call(DoneArgs {
            summary: "premature".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;
    assert!(rejected.is_err(), "Tier-1 failure must reject");
    session.done_rejections = codelet_tools::done_rejection_count(session_id);
    assert_eq!(session.done_rejections, 1, "one rejection accumulated");

    // @step And the model's done call with summary "All tests green" was accepted into the registry
    tool.call(DoneArgs {
        summary: "All tests green".to_string(),
        evidence: Some(vec!["cargo test output: 0 failed".to_string()]),
        goal_assessment: Some("Every acceptance criterion is now covered".to_string()),
    })
    .await
    .expect("goal-mode done() with evidence and assessment must be accepted");

    // handle_tool_result flushes the paired assistant tool_use and user
    // tool_result into session.messages BEFORE the early-exit consult
    // (CONT-005 pairing invariant) — replicate that real history here
    // (compaction_tool_call_preservation_test.rs message-construction
    // precedent) so the teardown's effect on it is observable.
    session.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
            id: "toolu_done_1".to_string(),
            call_id: Some("call_done_1".to_string()),
            function: ToolFunction {
                name: "done".to_string(),
                arguments: serde_json::json!({ "summary": "All tests green" }),
            },
            signature: None,
            additional_params: None,
        })),
    });
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::tool_result_with_call_id(
            "toolu_done_1",
            "call_done_1".to_string(),
            OneOrMany::one(ToolResultContent::Text(rig::message::Text {
                text: "Completion recorded. The turn will finish with your summary.".to_string(),
            })),
        )),
    });

    // @step When the early-exit decision is consulted after the done tool result is processed
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));

    // @step Then the decision consumes the acceptance and returns the summary "All tests green"
    assert_eq!(
        decision.as_deref(),
        Some("All tests green"),
        "goal mode must exit early at the ToolResult arm (CONT-005 deferral lifted)"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "the acceptance must be consumed by the early exit"
    );

    // @step And the shared teardown announces "🎯 goal satisfied: All tests green" and never "✓ done:"
    let output = RecordingOutput::new();
    apply_finish_with_summary(&mut session, session_id, "All tests green", &output);
    let statuses = output.statuses();
    assert_eq!(
        statuses,
        vec!["🎯 goal satisfied: All tests green".to_string()],
        "goal-mode teardown must announce the satisfied goal"
    );
    assert!(
        statuses.iter().all(|s| !s.contains("✓ done:")),
        "goal-mode teardown must never surface the non-goal '✓ done:' line"
    );

    // @step And the session goal is cleared and the CompletionContract reminder is removed from the conversation
    assert!(session.goal.is_none(), "session goal must be auto-cleared");
    assert_eq!(
        count_system_reminders_by_type(&session.messages, SystemReminderType::CompletionContract),
        0,
        "the CompletionContract reminder must be removed from session.messages"
    );

    // @step And the registry goal is cleared and both rejection counters reset to zero
    assert!(
        codelet_tools::get_session_goal(session_id).is_none(),
        "registry goal must be cleared"
    );
    assert_eq!(
        codelet_tools::done_rejection_count(session_id),
        0,
        "registry rejection count must reset"
    );
    assert_eq!(
        session.done_rejections, 0,
        "session rejection count must reset"
    );

    // @step And the nudge counter resets and the turn terminates with the literal stop_reason "done"
    assert_eq!(
        session.continue_nudges_used, 0,
        "teardown must reset the zero-progress nudge counter"
    );
    assert_eq!(
        DONE_EARLY_EXIT_STOP_REASON, "done",
        "early termination uses the literal stop_reason 'done'"
    );

    // @step And session messages contain the paired done tool_use and tool_result
    let tool_use_idx = session
        .messages
        .iter()
        .position(|m| {
            matches!(
                m,
                Message::Assistant { content, .. }
                    if content.iter().any(|c| matches!(
                        c,
                        AssistantContent::ToolCall(tc)
                            if tc.function.name == "done"
                                && tc.call_id.as_deref() == Some("call_done_1")
                    ))
            )
        })
        .expect("the done tool_use must survive the goal teardown in session.messages");
    let tool_result_idx = session
        .messages
        .iter()
        .position(|m| {
            matches!(
                m,
                Message::User { content }
                    if content.iter().any(|c| matches!(
                        c,
                        UserContent::ToolResult(tr)
                            if tr.call_id.as_deref() == Some("call_done_1")
                    ))
            )
        })
        .expect("the done tool_result must survive the goal teardown in session.messages");
    assert!(
        tool_use_idx < tool_result_idx,
        "the assistant tool_use must immediately precede its user tool_result \
         — the CompletionContract reminder removal must not orphan the pair"
    );
    assert_eq!(
        tool_result_idx,
        tool_use_idx + 1,
        "the pair must stay adjacent after the teardown"
    );

    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Immediate termination prevents repeat verify runs and acceptance
// overwrite
// ============================================================================

#[tokio::test]
async fn immediate_termination_prevents_repeat_verify_runs_and_acceptance_overwrite() {
    // @step Given a goal with a verify command that appends a line to a counter file is active for an armed session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    let dir = std::env::temp_dir().join(format!("cont006-verify-{session_id}"));
    std::fs::create_dir_all(&dir).expect("temp dir must be creatable");
    let counter = dir.join("counter.txt");
    let verify_cmd = format!("echo run >> {}", counter.display());
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: Some(verify_cmd),
        }),
    );

    // @step When the model's done call with evidence and a goal assessment passes the verify command
    let tool = DoneTool::new(session_id);
    tool.call(DoneArgs {
        summary: "All tests green".to_string(),
        evidence: Some(vec!["cargo test output: 0 failed".to_string()]),
        goal_assessment: Some("Every acceptance criterion is now covered".to_string()),
    })
    .await
    .expect("Tier-2 passing verify must accept the done() call");

    // @step Then the counter file records exactly one verify run
    let contents = std::fs::read_to_string(&counter).expect("verify must have run");
    assert_eq!(
        contents.lines().count(),
        1,
        "the verify command must run exactly once per accepted completion"
    );

    // @step And the early-exit decision consumes the acceptance so a second take finds nothing
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision.as_deref(),
        Some("All tests green"),
        "the early exit must consume the acceptance"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "no acceptance remains — the DONE_ACCEPTANCE overwrite window is closed"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// Scenario: Tier 1 rejected done() records a rejection and never exits early
// ============================================================================

#[tokio::test]
async fn tier_1_rejected_done_records_a_rejection_and_never_exits_early() {
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
    let mut session = fresh_session();
    session.set_goal("make all tests pass", None);

    // @step When the model calls done without evidence or a goal assessment
    let tool = DoneTool::new(session_id);
    let result = tool
        .call(DoneArgs {
            summary: "done!".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;

    // @step Then the done call is rejected as a tool error and the rejection count becomes 1
    assert!(result.is_err(), "Tier 1 failure must reject the done() call");
    assert_eq!(
        codelet_tools::done_rejection_count(session_id),
        1,
        "the rejection must be counted in the registry"
    );

    // @step And the early-exit decision finds no acceptance and the loop continues
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision, None,
        "a rejected done() records no acceptance, so no early exit fires"
    );

    // @step And the session goal and CompletionContract reminder stay intact
    assert!(
        session.goal.is_some(),
        "a rejected done() must leave the session goal active"
    );
    assert_eq!(
        count_system_reminders_by_type(&session.messages, SystemReminderType::CompletionContract),
        1,
        "the CompletionContract reminder must stay in the conversation"
    );
    assert!(
        codelet_tools::get_session_goal(session_id).is_some(),
        "the registry goal must stay active"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Failing Tier 2 verify rejects done() without early exit
// ============================================================================

#[tokio::test]
async fn failing_tier_2_verify_rejects_done_without_early_exit() {
    // @step Given a goal with the failing verify command "exit 1" is active for an armed session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: Some("exit 1".to_string()),
        }),
    );

    // @step When the model calls done with evidence and a goal assessment
    let tool = DoneTool::new(session_id);
    let result = tool
        .call(DoneArgs {
            summary: "All tests green".to_string(),
            evidence: Some(vec!["cargo test output: 0 failed".to_string()]),
            goal_assessment: Some("Every acceptance criterion is now covered".to_string()),
        })
        .await;

    // @step Then the done call is rejected with the verify failure and the rejection is counted
    assert!(result.is_err(), "Tier 2 verify failure must reject");
    assert_eq!(
        codelet_tools::done_rejection_count(session_id),
        1,
        "the Tier-2 rejection must be counted"
    );

    // @step And the early-exit decision finds no acceptance and the loop continues
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision, None,
        "a Tier-2 rejected done() records no acceptance, so no early exit fires"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
}

// ============================================================================
// Scenario: Settle-point escalation semantics are unchanged
// ============================================================================

#[test]
fn settle_point_escalation_semantics_are_unchanged() {
    use codelet_cli::interactive::auto_continue::{decide_goal_continuation, ContinueDecision};

    // @step Given no acceptance is pending at the FinalResponse settle point
    let done_summary: Option<&str> = None;

    // @step When the goal continuation decision runs with four rejections
    let decision = decide_goal_continuation(done_summary, Some("stop"), 0, 15, 4, 0, false);

    // @step Then the decision escalates for human review
    assert!(
        matches!(decision, ContinueDecision::Escalate(_)),
        "rejections >= 4 with no acceptance must still Escalate; got {decision:?}"
    );

    // @step And the stall fast-path and budget exhaustion still escalate
    let stall = decide_goal_continuation(None, Some("stop"), 0, 15, 0, 2, false);
    assert!(
        matches!(stall, ContinueDecision::Escalate(_)),
        "two consecutive zero-activity nudges must still Escalate; got {stall:?}"
    );
    let exhausted = decide_goal_continuation(None, Some("stop"), 15, 15, 0, 0, false);
    assert!(
        matches!(exhausted, ContinueDecision::Escalate(_)),
        "goal budget exhaustion must still Escalate; got {exhausted:?}"
    );

    // @step And a pending acceptance at the fallback still finishes with the summary before escalation is evaluated
    let raced = decide_goal_continuation(Some("All tests green"), Some("stop"), 0, 15, 4, 2, false);
    assert_eq!(
        raced,
        ContinueDecision::FinishWithSummary("All tests green".to_string()),
        "the FinalResponse fallback keeps acceptance-first ordering (assumption 1)"
    );
}

// ============================================================================
// Scenario: Stream-loop wiring pins goal-mode early exit and single-teardown
// invariants
// ============================================================================

#[test]
fn stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let source = stream_loop_source();

    // @step Then the ToolResult arm consults the early-exit decision without any goal gate
    let arm = tool_result_arm_region(&source);
    let consult = arm
        .find("decide_tool_result_early_exit")
        .expect("the ToolResult arm must consult the early-exit decision");
    let consult_block = &arm[consult..];
    let take = consult_block
        .find("take_done_acceptance")
        .expect("the consult must take from the DONE_ACCEPTANCE registry");
    assert!(
        !consult_block[..take].contains("goal"),
        "the early-exit consult must not gate on goal state — the CONT-005 \
         goal_active argument is lifted by CONT-006"
    );

    // @step And the FinalResponse fallback routes through the same shared teardown helper as the early exit
    let call_sites = source.matches("apply_finish_with_summary").count();
    assert_eq!(
        call_sites, 2,
        "exactly two apply_finish_with_summary call sites: the ToolResult-arm \
         early exit and the FinalResponse FinishWithSummary fallback"
    );
    let finish_arm = source
        .find("ContinueDecision::FinishWithSummary")
        .expect("stream_loop.rs must match on ContinueDecision::FinishWithSummary");
    let arm_end = source[finish_arm..]
        .find("ContinueDecision::FinishWithWarning")
        .expect("FinishWithWarning arm must follow FinishWithSummary")
        + finish_arm;
    assert!(
        source[finish_arm..arm_end].contains("apply_finish_with_summary"),
        "the FinishWithSummary arm body must call the shared teardown helper"
    );

    // @step And the goal announcement formatting lives only in the goal acceptance helper
    assert!(
        !source.contains("🎯 goal satisfied"),
        "stream_loop.rs must not format the goal announcement inline"
    );
    let helper_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/done_early_exit.rs"
    ))
    .expect("done_early_exit.rs must exist");
    assert!(
        !helper_source.contains("🎯 goal satisfied"),
        "done_early_exit.rs must delegate the announcement to goal::apply_goal_acceptance"
    );
    let goal_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/goal.rs"
    ))
    .expect("goal.rs must exist");
    assert!(
        goal_source.contains("🎯 goal satisfied"),
        "goal::apply_goal_acceptance must own the announcement formatting"
    );

    // @step And CompletionContract reminder removal lives only in the session goal clearing method
    assert!(
        !source.contains("remove_system_reminders_of_type"),
        "stream_loop.rs must not remove the CompletionContract reminder inline"
    );
    assert!(
        !helper_source.contains("remove_system_reminders_of_type"),
        "done_early_exit.rs must not remove the CompletionContract reminder inline"
    );
    let session_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/session/mod.rs"
    ))
    .expect("session/mod.rs must exist");
    let clear_goal = session_source
        .find("pub fn clear_goal")
        .expect("Session::clear_goal must exist");
    let after_sig = clear_goal + "pub fn clear_goal".len();
    let clear_goal_end = session_source[after_sig..]
        .find("pub fn ")
        .map_or(session_source.len(), |n| after_sig + n);
    assert!(
        session_source[clear_goal..clear_goal_end].contains("CompletionContract"),
        "Session::clear_goal must own the CompletionContract reminder removal"
    );
}

// ============================================================================
// Scenario: Verify command exceeding the timeout rejects done() without early
// exit
// ============================================================================

#[tokio::test]
async fn verify_command_exceeding_the_timeout_rejects_done_without_early_exit() {
    // @step Given a goal with a verify command that sleeps beyond the configured test timeout is active for an armed session
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    codelet_tools::set_session_goal(
        session_id,
        Some(GoalSpec {
            text: "make all tests pass".to_string(),
            verify: Some("sleep 5".to_string()),
        }),
    );
    // CONT-003 test hook (done.rs): shrink the Tier-2 verify timeout so the
    // sleeping command reliably exceeds it without slowing the suite.
    codelet_tools::set_verify_timeout_for_tests(
        session_id,
        std::time::Duration::from_millis(200),
    );

    // @step When the model calls done with evidence and a goal assessment
    let tool = DoneTool::new(session_id);
    let result = tool
        .call(DoneArgs {
            summary: "All tests green".to_string(),
            evidence: Some(vec!["cargo test output: 0 failed".to_string()]),
            goal_assessment: Some("Every acceptance criterion is now covered".to_string()),
        })
        .await;

    // @step Then the done call is rejected with a verify timeout and the rejection is counted
    let err = result.expect_err("a verify timeout must reject the done() call");
    assert!(
        err.to_string().contains("timed out"),
        "the rejection must carry the verify-timeout message; got: {err}"
    );
    assert_eq!(
        codelet_tools::done_rejection_count(session_id),
        1,
        "the timeout rejection must be counted"
    );

    // @step And the early-exit decision finds no acceptance and the loop continues
    let decision =
        decide_tool_result_early_exit(|| codelet_tools::take_done_acceptance(session_id));
    assert_eq!(
        decision, None,
        "a timed-out done() records no acceptance, so no early exit fires"
    );
    assert!(
        codelet_tools::get_session_goal(session_id).is_some(),
        "the registry goal must stay active after a timeout rejection"
    );

    codelet_tools::set_session_goal(session_id, None);
    codelet_tools::set_continue_armed(session_id, false);
}
