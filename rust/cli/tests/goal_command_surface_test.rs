#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/goal-command-surface.feature
//!
//! CONT-003: /goal command surface — grammar parser (`/goal <text>`, bare
//! `/goal`, `/goal verify <cmd>`, `/goal clear`), the /continue-off refusal
//! while a goal is active, and the status-bar goal indicator.
//!
//! These tests import the intended REAL production API from
//! codelet_cli::interactive::goal (mirroring the CONT-002 precedent in
//! codelet_cli::interactive::auto_continue — see
//! rust/cli/tests/auto_continue_engine_test.rs and
//! rust/fspec-tui/tests/cont002_continue_command_test.rs). Red phase:
//! the module does not exist yet, so this file fails to compile with
//! missing-API errors — the correct red-phase failure mode per the
//! CONT-002/RPC-383 precedent.

use codelet_cli::interactive::auto_continue::{
    apply_continue_command, parse_continue_command, ContinueCommand,
};
use codelet_cli::interactive::goal::{
    apply_goal_command, goal_status_indicator, parse_goal_command, GoalCommand,
};
use codelet_cli::session::Session;

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

// =============================================================================
// Scenario: Setting a goal arms Goal mode even when auto-continue is off
// =============================================================================

#[test]
fn setting_a_goal_arms_goal_mode_even_when_auto_continue_is_off() {
    // @step Given a session with auto-continue off
    let mut session = fresh_session();
    session.continue_enabled = false;
    session.done_rejections = 2;
    session.continue_nudges_used = 3;

    // @step When the user enters "/goal make all tests pass"
    let cmd = parse_goal_command("/goal make all tests pass");
    assert_eq!(
        cmd,
        GoalCommand::Set("make all tests pass".to_string()),
        "/goal <text> must parse as Set carrying the goal text"
    );
    let result = apply_goal_command(&mut session, &cmd);

    // @step Then the goal is set to "make all tests pass"
    assert_eq!(
        session.goal.as_ref().map(|g| g.text.as_str()),
        Some("make all tests pass"),
        "applying Set must store the goal on the session (mode Goal is derived)"
    );

    // @step And the printed state confirms the goal is active
    assert!(result.changed, "setting a goal must report a state change");
    assert!(
        result.message.contains("goal") && result.message.contains("make all tests pass"),
        "state message must confirm the active goal; got: {:?}",
        result.message
    );

    // @step And the done rejection count and nudge count are reset
    assert_eq!(
        session.done_rejections, 0,
        "goal set must reset done_rejections"
    );
    assert_eq!(
        session.continue_nudges_used, 0,
        "goal set must reset continue_nudges_used"
    );
}

// =============================================================================
// Scenario: Bare /goal shows the contract state
// =============================================================================

#[test]
fn bare_goal_shows_the_contract_state() {
    // @step Given a session with an active goal and a verify command configured
    let mut session = fresh_session();
    apply_goal_command(
        &mut session,
        &GoalCommand::Set("make all tests pass".to_string()),
    );
    apply_goal_command(&mut session, &GoalCommand::Verify("cargo test".to_string()));
    session.continue_nudges_used = 2;
    session.done_rejections = 1;

    // @step When the user enters "/goal"
    let cmd = parse_goal_command("/goal");
    assert_eq!(cmd, GoalCommand::Show, "bare /goal must parse as Show");
    let result = apply_goal_command(&mut session, &cmd);

    // @step Then the state output shows the goal text
    assert!(
        result.message.contains("make all tests pass"),
        "state output must show the goal text; got: {:?}",
        result.message
    );

    // @step And the state output shows the verify command
    assert!(
        result.message.contains("cargo test"),
        "state output must show the verify command; got: {:?}",
        result.message
    );

    // @step And the state output shows the effective budget, nudges used, and rejections
    assert!(
        result.message.contains("15"),
        "state output must show the effective budget (Goal default 15); got: {:?}",
        result.message
    );
    assert!(
        result.message.contains('2'),
        "state output must show nudges used (2); got: {:?}",
        result.message
    );
    assert!(
        result.message.contains('1'),
        "state output must show rejections (1); got: {:?}",
        result.message
    );
    assert!(!result.changed, "bare /goal must not change state");
}

// =============================================================================
// Scenario: Bare /goal without an active goal reports no goal set
// =============================================================================

#[test]
fn bare_goal_without_an_active_goal_reports_no_goal_set() {
    // @step Given a session with no active goal
    let mut session = fresh_session();
    assert!(session.goal.is_none(), "fresh session must have no goal");

    // @step When the user enters "/goal"
    let result = apply_goal_command(&mut session, &parse_goal_command("/goal"));

    // @step Then the output reports that no goal is set
    assert!(
        result.message.contains("no goal"),
        "bare /goal without a goal must report 'no goal'; got: {:?}",
        result.message
    );
    assert!(!result.changed, "showing state must not change state");
}

// =============================================================================
// Scenario: /goal verify attaches a verify command to the active goal
// =============================================================================

#[test]
fn goal_verify_attaches_a_verify_command_to_the_active_goal() {
    // @step Given a session with an active goal and no verify command
    let mut session = fresh_session();
    apply_goal_command(
        &mut session,
        &GoalCommand::Set("make all tests pass".to_string()),
    );
    assert_eq!(
        session.goal.as_ref().and_then(|g| g.verify.clone()),
        None,
        "a freshly set goal must have no verify command"
    );

    // @step When the user enters "/goal verify cargo test"
    let cmd = parse_goal_command("/goal verify cargo test");
    assert_eq!(
        cmd,
        GoalCommand::Verify("cargo test".to_string()),
        "/goal verify <cmd> must parse as Verify carrying the command"
    );
    let result = apply_goal_command(&mut session, &cmd);

    // @step Then the goal's verify command is "cargo test"
    assert_eq!(
        session.goal.as_ref().and_then(|g| g.verify.as_deref()),
        Some("cargo test"),
        "verify command must be attached to the active goal"
    );

    // @step And the printed state confirms the verify command
    assert!(
        result.changed,
        "attaching a verify command is a state change"
    );
    assert!(
        result.message.contains("cargo test"),
        "state message must confirm the verify command; got: {:?}",
        result.message
    );
}

// =============================================================================
// Scenario: /goal verify without an active goal is an error
// =============================================================================

#[test]
fn goal_verify_without_an_active_goal_is_an_error() {
    // @step Given a session with no active goal
    let mut session = fresh_session();
    assert!(session.goal.is_none(), "fresh session must have no goal");
    let enabled_before = session.continue_enabled;

    // @step When the user enters "/goal verify cargo test"
    let result = apply_goal_command(&mut session, &parse_goal_command("/goal verify cargo test"));

    // @step Then the command errors telling the user to set a goal first
    assert!(
        result.message.contains("no active goal") && result.message.contains("/goal <text>"),
        "error must be 'no active goal — set one first with /goal <text>'; got: {:?}",
        result.message
    );

    // @step And no goal state is changed
    assert!(!result.changed, "a refused verify must not change state");
    assert!(session.goal.is_none(), "goal must remain unset");
    assert_eq!(
        session.continue_enabled, enabled_before,
        "continue toggle must be untouched"
    );
}

// =============================================================================
// Scenario: /goal clear drops the goal and prints the fallback state
// =============================================================================

#[test]
fn goal_clear_drops_the_goal_and_prints_the_fallback_state() {
    // @step Given a session with an active goal and auto-continue on
    let mut session = fresh_session();
    session.continue_enabled = true;
    apply_goal_command(
        &mut session,
        &GoalCommand::Set("make all tests pass".to_string()),
    );
    assert!(session.goal.is_some(), "goal must be active before clear");

    // @step When the user enters "/goal clear"
    let cmd = parse_goal_command("/goal clear");
    assert_eq!(cmd, GoalCommand::Clear, "/goal clear must parse as Clear");
    let result = apply_goal_command(&mut session, &cmd);

    // @step Then the goal is cleared
    assert!(session.goal.is_none(), "clear must drop the goal");
    assert!(result.changed, "clearing an active goal is a state change");

    // @step And the printed state shows the fallback to auto-continue
    assert!(
        result.message.contains("auto-continue"),
        "fallback state with continue_enabled=true must mention auto-continue; got: {:?}",
        result.message
    );
}

// =============================================================================
// Scenario: /goal clear without an active goal reports no goal set
// =============================================================================

#[test]
fn goal_clear_without_an_active_goal_reports_no_goal_set() {
    // @step Given a session with no active goal
    let mut session = fresh_session();
    assert!(session.goal.is_none(), "fresh session must have no goal");
    let enabled_before = session.continue_enabled;
    let budget_before = session.continue_budget;

    // @step When the user enters "/goal clear"
    let result = apply_goal_command(&mut session, &parse_goal_command("/goal clear"));

    // @step Then the output reports that no goal is set
    assert!(
        result.message.contains("no goal"),
        "/goal clear without a goal must report 'no goal'; got: {:?}",
        result.message
    );

    // @step And no state is changed
    assert!(
        !result.changed,
        "clearing a non-existent goal must not change state"
    );
    assert!(session.goal.is_none());
    assert_eq!(session.continue_enabled, enabled_before);
    assert_eq!(session.continue_budget, budget_before);
}

// =============================================================================
// Scenario: /goal with replacement text replaces the goal and resets counters
// =============================================================================

#[test]
fn goal_with_replacement_text_replaces_the_goal_and_resets_counters() {
    // @step Given a session with an active goal and recorded rejections and nudges
    let mut session = fresh_session();
    apply_goal_command(
        &mut session,
        &GoalCommand::Set("make all tests pass".to_string()),
    );
    session.done_rejections = 3;
    session.continue_nudges_used = 4;

    // @step When the user enters "/goal ship the release"
    let cmd = parse_goal_command("/goal ship the release");
    assert_eq!(
        cmd,
        GoalCommand::Set("ship the release".to_string()),
        "/goal with text must parse as Set even when a goal is already active"
    );
    apply_goal_command(&mut session, &cmd);

    // @step Then the goal text becomes "ship the release"
    assert_eq!(
        session.goal.as_ref().map(|g| g.text.as_str()),
        Some("ship the release"),
        "replacement must overwrite the goal text"
    );

    // @step And the done rejection count and nudge count are reset
    assert_eq!(
        session.done_rejections, 0,
        "goal replacement must reset done_rejections"
    );
    assert_eq!(
        session.continue_nudges_used, 0,
        "goal replacement must reset continue_nudges_used"
    );
}

// =============================================================================
// Scenario: /continue off is refused while a goal is active
// =============================================================================

#[test]
fn continue_off_is_refused_while_a_goal_is_active() {
    // @step Given a session with an active goal and auto-continue armed
    let (enabled, budget) = (true, 10u32);
    let goal_active = true;

    // @step When the user enters "/continue off"
    let cmd = parse_continue_command("/continue off");
    assert_eq!(cmd, ContinueCommand::Off, "/continue off parses as Off");
    let result = apply_continue_command(enabled, budget, goal_active, &cmd);

    // @step Then the command is refused with the message to clear the goal first
    assert!(
        result.message.contains("clear the goal first") && result.message.contains("/goal clear"),
        "refusal must say 'clear the goal first (/goal clear)'; got: {:?}",
        result.message
    );

    // @step And the continue toggle and budget are unchanged
    assert!(
        !result.changed,
        "a refused /continue off must not change state"
    );
    assert!(result.enabled, "continue toggle must remain on");
    assert_eq!(result.budget, 10, "budget must remain 10");
}

// =============================================================================
// Scenario: /continue off works normally when no goal is active
// =============================================================================

#[test]
fn continue_off_works_normally_when_no_goal_is_active() {
    // @step Given a session with no active goal and auto-continue on
    let (enabled, budget) = (true, 10u32);
    let goal_active = false;

    // @step When the user enters "/continue off"
    let result = apply_continue_command(
        enabled,
        budget,
        goal_active,
        &parse_continue_command("/continue off"),
    );

    // @step Then auto-continue turns off
    assert!(!result.enabled, "explicit off with no goal must disable");
    assert!(result.changed, "turning off is a state change");
}

// =============================================================================
// Scenario: Status indicator shows the goal marker while a goal is active
// =============================================================================

#[test]
fn status_indicator_shows_the_goal_marker_while_a_goal_is_active() {
    // @step Given a session with an active goal and nudge accounting
    let (goal_active, continue_enabled, nudges_used, explicit_budget) = (true, true, 2u32, 10u32);

    // @step When the status indicator is computed
    let indicator =
        goal_status_indicator(goal_active, continue_enabled, nudges_used, explicit_budget);

    // @step Then it shows the goal indicator with nudges used over the effective budget
    assert_eq!(
        indicator.as_deref(),
        Some("🎯 goal (2/15)"),
        "goal indicator must render nudges_used over the effective budget max(10,15)=15"
    );

    // @step And it replaces the auto-continue indicator
    assert!(
        !indicator.unwrap().contains("auto-continue"),
        "the goal indicator must replace the ⏩ auto-continue indicator"
    );

    // @step And after the goal is cleared with auto-continue on the auto-continue indicator returns
    let after_clear = goal_status_indicator(false, true, 2, 10);
    assert_eq!(
        after_clear.as_deref(),
        Some("⏩ auto-continue (2/10)"),
        "clearing the goal with auto-continue on must fall back to the ⏩ indicator"
    );
}
