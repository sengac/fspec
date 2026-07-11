#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/goal-enforcement.feature
//!
//! CONT-003: Goal enforcement engine — derived mode, the done() acceptance
//! pipeline (Tier 1 schema check, Tier 2 verify command), escalation
//! (rejections >= 4, Goal-mode budget exhaustion, stall fast-path),
//! effective budget resolution max(explicit, 15), and compaction-proof
//! goal persistence via the CompletionContract system reminder.
//!
//! These tests import the intended REAL production API (CONT-002 precedent:
//! codelet/cli/tests/auto_continue_engine_test.rs — no copies, no mocks):
//! - codelet_cli::interactive::goal — effective_mode/EffectiveMode,
//!   effective_goal_budget, session goal state helpers
//! - codelet_cli::interactive::auto_continue — decide_goal_continuation /
//!   ContinueDecision::Escalate extension
//! - codelet_tools — done() registry goal extension (set_session_goal /
//!   GoalSpec) driving Tier 1/2 checks inside DoneTool::call()
//! - codelet_cli::session::system_reminders — CompletionContract reminder
//!   type + remove_system_reminders_of_type
//!
//! Red phase: the goal API surface does not exist yet, so this file fails
//! to compile with missing-API errors — the correct red-phase failure mode.

use codelet_cli::interactive::auto_continue::{decide_goal_continuation, ContinueDecision};
use codelet_cli::interactive::goal::{effective_mode, EffectiveMode};
use codelet_cli::session::Session;
use codelet_tools::{set_session_goal, DoneArgs, DoneTool, GoalSpec};
use rig::tool::Tool;

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

/// Arm a fresh session-scoped done tool with an active goal in the
/// codelet_tools registry (the dispatch sites sync Session.goal → registry).
fn armed_goal_tool(goal_text: &str, verify: Option<&str>) -> (uuid::Uuid, DoneTool) {
    let session_id = uuid::Uuid::new_v4();
    codelet_tools::set_continue_armed(session_id, true);
    set_session_goal(
        session_id,
        Some(GoalSpec {
            text: goal_text.to_string(),
            verify: verify.map(str::to_string),
        }),
    );
    (session_id, DoneTool::new(session_id))
}

// =============================================================================
// Scenario: Goal presence wins the derived mode over the continue toggle
// =============================================================================

#[test]
fn goal_presence_wins_the_derived_mode_over_the_continue_toggle() {
    // @step Given a session with auto-continue off
    let mut session = fresh_session();
    session.continue_enabled = false;
    assert_eq!(
        effective_mode(&session),
        EffectiveMode::Off,
        "no goal + continue off must derive Off"
    );

    // @step When a goal "make all tests pass" is set on the session
    session.set_goal("make all tests pass", None);

    // @step Then the effective mode is Goal
    assert_eq!(
        effective_mode(&session),
        EffectiveMode::Goal,
        "goal presence must win the derived mode even with continue off"
    );

    // @step And when the goal is cleared with auto-continue on the effective mode is AutoContinue
    session.continue_enabled = true;
    session.clear_goal();
    assert_eq!(
        effective_mode(&session),
        EffectiveMode::AutoContinue,
        "goal clear with continue_enabled=true must fall back to AutoContinue"
    );

    // @step And when the goal is cleared with auto-continue off the effective mode is Off
    session.set_goal("make all tests pass", None);
    session.continue_enabled = false;
    session.clear_goal();
    assert_eq!(
        effective_mode(&session),
        EffectiveMode::Off,
        "goal clear with continue_enabled=false must fall back to Off"
    );
}

// =============================================================================
// Scenario: done() without evidence or goal assessment is rejected at Tier 1
// =============================================================================

#[tokio::test]
async fn done_without_evidence_or_goal_assessment_is_rejected_at_tier_1() {
    // @step Given a session with an active goal "make all tests pass"
    let (session_id, tool) = armed_goal_tool("make all tests pass", None);

    // @step When the model calls done with only a summary
    let result = tool
        .call(DoneArgs {
            summary: "all done".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;

    // @step Then the done call is rejected as a failed tool result
    let err = result.expect_err("Tier 1 must reject done() lacking evidence/goal_assessment");
    let message = err.to_string();
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "a rejected done() must not record acceptance"
    );

    // @step And the rejection message contains the goal text "make all tests pass"
    assert!(
        message.contains("make all tests pass"),
        "Tier 1 rejection must include the goal text; got: {message:?}"
    );

    // @step And the rejection message instructs the model to provide evidence and a goal_assessment
    assert!(
        message.contains("evidence") && message.contains("goal_assessment"),
        "Tier 1 rejection must instruct providing evidence and goal_assessment; got: {message:?}"
    );
}

// =============================================================================
// Scenario: done() with a trivial goal assessment is rejected at Tier 1
// =============================================================================

#[tokio::test]
async fn done_with_a_trivial_goal_assessment_is_rejected_at_tier_1() {
    // @step Given a session with an active goal "make all tests pass"
    let (session_id, tool) = armed_goal_tool("make all tests pass", None);

    // @step When the model calls done with evidence and a goal_assessment shorter than 20 characters
    let result = tool
        .call(DoneArgs {
            summary: "all done".to_string(),
            evidence: Some(vec!["cargo test output: 100 passed".to_string()]),
            goal_assessment: Some("done".to_string()),
        })
        .await;

    // @step Then the done call is rejected as a failed tool result
    assert!(
        result.is_err(),
        "a trivial (< 20 chars trimmed) goal_assessment must be rejected at Tier 1"
    );
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "a rejected done() must not record acceptance"
    );
}

// =============================================================================
// Scenario: done() with evidence and assessment is accepted when no verify
// command is configured
// =============================================================================

#[tokio::test]
async fn done_with_evidence_and_assessment_is_accepted_when_no_verify_command_is_configured() {
    // @step Given a session with an active goal "make all tests pass" and no verify command
    let (session_id, tool) = armed_goal_tool("make all tests pass", None);

    // @step When the model calls done with a summary, non-empty evidence, and a substantive goal_assessment
    let result = tool
        .call(DoneArgs {
            summary: "All tests pass".to_string(),
            evidence: Some(vec!["cargo test: 212 passed, 0 failed".to_string()]),
            goal_assessment: Some(
                "The full workspace test suite passes, satisfying the goal.".to_string(),
            ),
        })
        .await;

    // @step Then the done call is accepted
    assert!(
        result.is_ok(),
        "Tier 1-complete done() with no verify command must be accepted; got: {result:?}"
    );

    // @step And the acceptance is recorded for the session
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id).as_deref(),
        Some("All tests pass"),
        "acceptance must be recorded for the session"
    );
}

// =============================================================================
// Scenario: Failing verify command rejects done() with exit code and output tail
// =============================================================================

#[tokio::test]
async fn failing_verify_command_rejects_done_with_exit_code_and_output_tail() {
    // @step Given a session with an active goal and verify command "false"
    let (session_id, tool) = armed_goal_tool("make all tests pass", Some("false"));

    // @step When the model calls done with valid Tier 1 arguments
    let result = tool
        .call(DoneArgs {
            summary: "All tests pass".to_string(),
            evidence: Some(vec!["cargo test: 212 passed, 0 failed".to_string()]),
            goal_assessment: Some(
                "The full workspace test suite passes, satisfying the goal.".to_string(),
            ),
        })
        .await;

    // @step Then the done call is rejected as a failed tool result
    let err = result.expect_err("a failing verify command must reject done() at Tier 2");
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "a Tier 2 rejected done() must not record acceptance"
    );

    // @step And the rejection message reports the verification exit code
    let message = err.to_string();
    assert!(
        message.contains("verification") && message.contains("exit"),
        "Tier 2 rejection must report the verification exit code; got: {message:?}"
    );
}

// =============================================================================
// Scenario: Verify command exit code is surfaced in the rejection
// =============================================================================

#[tokio::test]
async fn verify_command_exit_code_is_surfaced_in_the_rejection() {
    // @step Given a session with an active goal and verify command "sh -c 'echo boom; exit 3'"
    let (_session_id, tool) =
        armed_goal_tool("make all tests pass", Some("sh -c 'echo boom; exit 3'"));

    // @step When the model calls done with valid Tier 1 arguments
    let result = tool
        .call(DoneArgs {
            summary: "All tests pass".to_string(),
            evidence: Some(vec!["cargo test: 212 passed, 0 failed".to_string()]),
            goal_assessment: Some(
                "The full workspace test suite passes, satisfying the goal.".to_string(),
            ),
        })
        .await;
    let message = result
        .expect_err("verify exiting 3 must reject done() at Tier 2")
        .to_string();

    // @step Then the rejection message contains exit code 3
    assert!(
        message.contains('3'),
        "Tier 2 rejection must surface exit code 3; got: {message:?}"
    );

    // @step And the rejection message contains the verification output tail "boom"
    assert!(
        message.contains("boom"),
        "Tier 2 rejection must include the bounded output tail; got: {message:?}"
    );
}

// =============================================================================
// Scenario: Passing verify command accepts done() and auto-clears the goal
// =============================================================================

#[tokio::test]
async fn passing_verify_command_accepts_done_and_auto_clears_the_goal() {
    // @step Given a session with an active goal and verify command "true"
    let mut session = fresh_session();
    session.continue_enabled = true;
    session.set_goal("make all tests pass", Some("true"));
    session.done_rejections = 2;
    let (session_id, tool) = armed_goal_tool("make all tests pass", Some("true"));

    // @step When the model calls done with valid Tier 1 arguments
    let result = tool
        .call(DoneArgs {
            summary: "All tests pass".to_string(),
            evidence: Some(vec!["cargo test: 212 passed, 0 failed".to_string()]),
            goal_assessment: Some(
                "The full workspace test suite passes, satisfying the goal.".to_string(),
            ),
        })
        .await;

    // @step Then the done call is accepted
    assert!(
        result.is_ok(),
        "verify 'true' must accept a Tier 1-complete done(); got: {result:?}"
    );
    let summary =
        codelet_tools::take_done_acceptance(session_id).expect("acceptance must be recorded");

    // @step And the user sees the goal satisfied announcement with the summary
    let announcement =
        codelet_cli::interactive::goal::apply_goal_acceptance(&mut session, &summary);
    assert!(
        announcement.contains("🎯 goal satisfied:") && announcement.contains("All tests pass"),
        "announcement must be '🎯 goal satisfied: <summary>'; got: {announcement:?}"
    );

    // @step And the goal is auto-cleared falling back to the continue toggle
    assert!(
        session.goal.is_none(),
        "acceptance must auto-clear the goal"
    );
    assert_eq!(
        effective_mode(&session),
        EffectiveMode::AutoContinue,
        "with continue_enabled=true the mode must fall back to AutoContinue"
    );

    // @step And the done rejection count is reset
    assert_eq!(
        session.done_rejections, 0,
        "acceptance must reset done_rejections"
    );
}

// =============================================================================
// Scenario: Verify command exceeding the timeout rejects done() with a
// timeout message
// =============================================================================

#[tokio::test]
async fn verify_command_exceeding_the_timeout_rejects_done_with_a_timeout_message() {
    // @step Given a session with an active goal and a verify command that sleeps past a bounded timeout
    let (session_id, tool) = armed_goal_tool("make all tests pass", Some("sleep 5"));
    codelet_tools::set_verify_timeout_for_tests(session_id, std::time::Duration::from_millis(200));

    // @step When the model calls done with valid Tier 1 arguments
    let result = tool
        .call(DoneArgs {
            summary: "All tests pass".to_string(),
            evidence: Some(vec!["cargo test: 212 passed, 0 failed".to_string()]),
            goal_assessment: Some(
                "The full workspace test suite passes, satisfying the goal.".to_string(),
            ),
        })
        .await;

    // @step Then the done call is rejected as a failed tool result
    let err = result.expect_err("a verify command exceeding the timeout must reject done()");
    assert_eq!(
        codelet_tools::take_done_acceptance(session_id),
        None,
        "a timed-out verify must not record acceptance"
    );

    // @step And the rejection message reports a verification timeout
    let message = err.to_string().to_lowercase();
    assert!(
        message.contains("time") && message.contains("out"),
        "rejection must report a verification timeout; got: {message:?}"
    );
}

// =============================================================================
// Scenario: Fourth done() rejection escalates while the goal stays active
// =============================================================================

#[tokio::test]
async fn fourth_done_rejection_escalates_while_the_goal_stays_active() {
    // @step Given a session with an active goal and 3 recorded done rejections
    let (session_id, tool) = armed_goal_tool("make all tests pass", None);
    for _ in 0..3 {
        let rejected = tool
            .call(DoneArgs {
                summary: "all done".to_string(),
                evidence: None,
                goal_assessment: None,
            })
            .await;
        assert!(rejected.is_err(), "Tier 1-failing done() must be rejected");
    }
    assert_eq!(
        codelet_tools::done_rejection_count(session_id),
        3,
        "the registry must count 3 rejections"
    );

    // @step When the model's done call is rejected a 4th time and the stream settles
    let rejected = tool
        .call(DoneArgs {
            summary: "all done".to_string(),
            evidence: None,
            goal_assessment: None,
        })
        .await;
    assert!(rejected.is_err(), "the 4th done() must also be rejected");
    let rejections = codelet_tools::done_rejection_count(session_id);
    assert_eq!(rejections, 4, "the registry must count 4 rejections");
    let decision = decide_goal_continuation(None, Some("stop"), 0, 15, rejections, 0, false);

    // @step Then the engine decides to escalate for human review
    let ContinueDecision::Escalate(message) = decision else {
        panic!("4 rejections must produce Escalate, got {decision:?}");
    };

    // @step And the escalation message says the model repeatedly claims completion but verification fails
    assert!(
        message.contains("repeatedly claims completion"),
        "escalation message must describe repeated completion claims; got: {message:?}"
    );

    // @step And the goal remains active
    assert!(
        codelet_tools::get_session_goal(session_id).is_some(),
        "escalation must NOT clear the goal"
    );
}

// =============================================================================
// Scenario: Budget exhaustion in Goal mode escalates instead of the
// AutoContinue warning finish
// =============================================================================

#[test]
fn budget_exhaustion_in_goal_mode_escalates_instead_of_the_auto_continue_warning_finish() {
    // @step Given a session in Goal mode with all zero-progress nudges consumed
    let (nudges_used, budget) = (15u32, 15u32);

    // @step When the model stops cleanly without calling done and the stream settles
    let decision = decide_goal_continuation(None, Some("stop"), nudges_used, budget, 0, 0, false);

    // @step Then the engine decides to escalate for human review
    assert!(
        matches!(decision, ContinueDecision::Escalate(_)),
        "Goal-mode budget exhaustion must escalate, got {decision:?}"
    );

    // @step And the AutoContinue budget-exhaustion warning is not used
    assert!(
        !matches!(decision, ContinueDecision::FinishWithWarning(_)),
        "Goal mode must NOT use the AutoContinue silent-warning finish"
    );
}

// =============================================================================
// Scenario: Two consecutive zero-activity nudges escalate immediately
// =============================================================================

#[test]
fn two_consecutive_zero_activity_nudges_escalate_immediately() {
    // @step Given a session in Goal mode with remaining nudge budget
    let (nudges_used, budget) = (2u32, 15u32);
    assert!(
        nudges_used < budget,
        "budget must remain for the fast-path to matter"
    );

    // @step When two consecutive nudged segments produce no tool calls and no done call
    let consecutive_zero_activity_nudges = 2u32;
    let decision = decide_goal_continuation(
        None,
        Some("stop"),
        nudges_used,
        budget,
        0,
        consecutive_zero_activity_nudges,
        false,
    );

    // @step Then the engine decides to escalate immediately without burning the remaining budget
    assert!(
        matches!(decision, ContinueDecision::Escalate(_)),
        "the stall fast-path (2 consecutive zero-activity nudges) must escalate \
         immediately even with budget remaining, got {decision:?}"
    );
    assert!(
        !matches!(decision, ContinueDecision::Nudge),
        "the fast-path must not keep nudging into the remaining budget"
    );
}

// =============================================================================
// Scenario: Escalation pauses the session when a pause handler is registered
// =============================================================================

#[test]
fn escalation_pauses_the_session_when_a_pause_handler_is_registered() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // @step Given a session with a registered pause handler
    let session_id = uuid::Uuid::new_v4();
    let received = Arc::new(AtomicBool::new(false));
    let received_clone = received.clone();
    let handler: codelet_tools::tool_pause::PauseHandler = Arc::new(move |request| {
        assert_eq!(
            request.tool_name, "goal",
            "goal escalation must identify itself as the 'goal' tool"
        );
        received_clone.store(true, Ordering::SeqCst);
        codelet_tools::tool_pause::PauseResponse::Resumed
    });
    codelet_tools::tool_pause::set_pause_handler(session_id, Some(handler));

    // @step When a goal escalation is raised for the session
    let response = codelet_cli::interactive::goal::raise_goal_escalation(
        session_id,
        "🎯 goal: model repeatedly claims completion but verification fails — human review needed",
    );

    // @step Then the pause handler receives the goal escalation request
    assert!(
        received.load(Ordering::SeqCst),
        "the registered pause handler must receive the goal escalation"
    );

    // @step And the turn finishes after the pause resolves
    assert_eq!(
        response,
        codelet_tools::tool_pause::PauseResponse::Resumed,
        "the escalation must return once the pause resolves so the turn can finish"
    );
    codelet_tools::tool_pause::set_pause_handler(session_id, None);
}

// =============================================================================
// Scenario: Escalation in plain CLI repl finishes the turn with a prominent
// blocked message
// =============================================================================

#[test]
fn escalation_in_plain_cli_repl_finishes_the_turn_with_a_prominent_blocked_message() {
    // @step Given a session with no registered pause handler
    let session_id = uuid::Uuid::new_v4();
    assert!(
        !codelet_tools::tool_pause::has_pause_handler(session_id),
        "plain CLI repl sessions register no pause handler"
    );

    // @step When a goal escalation is raised for the session
    let blocked_message = codelet_cli::interactive::goal::build_goal_blocked_message();
    let response =
        codelet_cli::interactive::goal::raise_goal_escalation(session_id, &blocked_message);

    // @step Then pause resolution returns immediately
    assert_eq!(
        response,
        codelet_tools::tool_pause::PauseResponse::Resumed,
        "with no handler, pause_for_user must return Resumed immediately"
    );

    // @step And the turn finishes with the prominent blocked message
    assert!(
        blocked_message.contains("🎯 goal:") && blocked_message.contains("human review needed"),
        "the blocked message must be the prominent '🎯 goal: … — human review needed' \
         line; got: {blocked_message:?}"
    );
}

// =============================================================================
// Scenario: Larger explicit continue budget overrides the Goal default of 15
// =============================================================================

#[test]
fn larger_explicit_continue_budget_overrides_the_goal_default_of_15() {
    // @step Given a session where the user set an explicit continue budget of 40
    let mut session = fresh_session();
    session.continue_enabled = true;
    session.continue_budget = 40;

    // @step When a goal is set on the session
    session.set_goal("make all tests pass", None);

    // @step Then the effective Goal-mode budget is 40
    assert_eq!(
        codelet_cli::interactive::goal::effective_goal_budget(&session),
        40,
        "max(explicit 40, Goal default 15) must be 40"
    );
}

// =============================================================================
// Scenario: Goal default budget of 15 overrides a smaller explicit continue
// budget
// =============================================================================

#[test]
fn goal_default_budget_of_15_overrides_a_smaller_explicit_continue_budget() {
    // @step Given a session where the user set an explicit continue budget of 5
    let mut session = fresh_session();
    session.continue_enabled = true;
    session.continue_budget = 5;

    // @step When a goal is set on the session
    session.set_goal("make all tests pass", None);

    // @step Then the effective Goal-mode budget is 15
    assert_eq!(
        codelet_cli::interactive::goal::effective_goal_budget(&session),
        15,
        "max(explicit 5, Goal default 15) must be 15"
    );
}

// =============================================================================
// Scenario: Goal text survives compaction via the CompletionContract system
// reminder
// =============================================================================

#[test]
fn goal_text_survives_compaction_via_the_completion_contract_system_reminder() {
    use codelet_cli::session::system_reminders::{add_system_reminder, partition_for_compaction};
    use codelet_cli::session::SystemReminderType;
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;

    // @step Given a session with an active goal injected as a CompletionContract system reminder
    let mut messages = vec![Message::User {
        content: OneOrMany::one(UserContent::text("please make all tests pass")),
    }];
    messages = add_system_reminder(
        &messages,
        SystemReminderType::CompletionContract,
        "Active goal: make all tests pass. done() requires evidence and a goal_assessment.",
    );
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("keep working")),
    });

    // @step When the conversation is partitioned for compaction
    let (system_reminders, compactable) = partition_for_compaction(&messages);

    // @step Then the CompletionContract reminder is preserved as the latest of its type
    let preserved: Vec<&Message> = system_reminders
        .iter()
        .filter(|msg| match msg {
            Message::User { content } => match content.first() {
                UserContent::Text(t) => t.text.contains("type:completionContract"),
                _ => false,
            },
            _ => false,
        })
        .collect();
    assert_eq!(
        preserved.len(),
        1,
        "exactly one CompletionContract reminder must be preserved"
    );
    assert_eq!(
        compactable.len(),
        2,
        "the two plain user messages must remain compactable"
    );

    // @step And the preserved reminder contains the goal text
    let Message::User { content } = preserved[0] else {
        panic!("preserved reminder must be a user message");
    };
    let UserContent::Text(text) = content.first() else {
        panic!("preserved reminder must be text content");
    };
    assert!(
        text.text.contains("make all tests pass"),
        "the preserved reminder must carry the goal text; got: {:?}",
        text.text
    );
}

// =============================================================================
// Scenario: Clearing the goal removes the CompletionContract system reminder
// =============================================================================

#[test]
fn clearing_the_goal_removes_the_completion_contract_system_reminder() {
    use codelet_cli::session::system_reminders::{
        add_system_reminder, count_system_reminders_by_type, remove_system_reminders_of_type,
    };
    use codelet_cli::session::SystemReminderType;
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;

    // @step Given a session with an active goal injected as a CompletionContract system reminder
    let mut messages = vec![Message::User {
        content: OneOrMany::one(UserContent::text("please make all tests pass")),
    }];
    messages = add_system_reminder(
        &messages,
        SystemReminderType::Environment,
        "Platform: linux",
    );
    messages = add_system_reminder(
        &messages,
        SystemReminderType::CompletionContract,
        "Active goal: make all tests pass. done() requires evidence and a goal_assessment.",
    );
    assert_eq!(
        count_system_reminders_by_type(&messages, SystemReminderType::CompletionContract),
        1,
        "the CompletionContract reminder must be present before removal"
    );

    // @step When the CompletionContract reminders are removed from the conversation
    let cleaned =
        remove_system_reminders_of_type(&messages, SystemReminderType::CompletionContract);

    // @step Then no CompletionContract reminder remains in the conversation
    assert_eq!(
        count_system_reminders_by_type(&cleaned, SystemReminderType::CompletionContract),
        0,
        "removal must drop every CompletionContract reminder"
    );

    // @step And other system reminder types are untouched
    assert_eq!(
        count_system_reminders_by_type(&cleaned, SystemReminderType::Environment),
        1,
        "removal must not touch other reminder types"
    );
    assert_eq!(
        cleaned.len(),
        messages.len() - 1,
        "exactly one message (the CompletionContract reminder) must be removed"
    );
}

// =============================================================================
// Scenario: done() tool description includes the goal text while a goal is
// active
// =============================================================================

#[tokio::test]
async fn done_tool_description_includes_the_goal_text_while_a_goal_is_active() {
    // @step Given a session with an active goal "make all tests pass"
    let (session_id, tool) = armed_goal_tool("make all tests pass", None);

    // @step When the done tool definition is built for the session
    let definition = tool.definition(String::new()).await;

    // @step Then the tool description contains "The current goal is: make all tests pass"
    assert!(
        definition
            .description
            .contains("The current goal is: make all tests pass"),
        "definition() must append the active goal to the description; got: {:?}",
        definition.description
    );

    // @step And after the goal is cleared the tool description no longer mentions a goal
    set_session_goal(session_id, None);
    let cleared = tool.definition(String::new()).await;
    assert!(
        !cleared.description.contains("The current goal is:"),
        "after clearing the goal the description must not mention a goal; got: {:?}",
        cleared.description
    );
}
