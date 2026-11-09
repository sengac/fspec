// Feature: spec/features/starting-a-turn-clears-only-the-submitted-draft.feature
//! BUG-554 — starting a turn (and steering or queueing) clears the
//! session's pending-input draft only while it is the submitted text.
//!
//! Drives a real `BackgroundSession` whose prompt channel has no agent
//! loop behind it (the RPC-409 test shape): `send_input` runs the daemon's
//! `start_turn` to completion and parks the prompt on the channel, so "the
//! send reaches the daemon" is one deterministic call and no model is ever
//! called. The TUI's draft saves are `set_pending_input` calls, exactly as
//! the RPC `set_pending_input` handler makes them.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{InputMode, SessionId, SessionStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_test_helpers::{OrFail, TestResult};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// A live session with no agent loop: the returned receiver holds the
/// prompts its turns would run.
fn idle_session(
) -> Result<(Arc<BackgroundSession>, mpsc::Receiver<PromptInput>), Box<dyn std::error::Error>> {
    // One data directory for the whole binary: receipts are retained under
    // the process-wide data directory, so parallel tests must not move it.
    static DATA: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let data = DATA
        .get_or_init(|| {
            let data = codelet_test_helpers::shared_temp_subdirectory()?;
            codelet_common::set_data_directory(data.clone())?;
            Ok(data)
        })
        .clone();
    data.or_fail("a temp data directory")?;
    let inner = codelet_cli::session::Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Claude,
        None,
        None,
    ));
    let (chunks_tx, _chunks_rx) = broadcast::channel::<(SessionId, StreamChunk)>(64);
    let (status_tx, _status_rx) = broadcast::channel::<(SessionId, SessionStatus)>(64);
    let (cleared_tx, _cleared_rx) = broadcast::channel::<SessionId>(8);
    let (input_tx, input_rx) = mpsc::channel(8);
    let session = Arc::new(BackgroundSession::new(
        Uuid::new_v4(),
        "bug554".to_string(),
        "/tmp".to_string(),
        None,
        None,
        inner,
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        status_tx,
        cleared_tx,
    ));
    Ok((session, input_rx))
}

/// The prompt text of the next turn the session started.
fn started_turn(
    prompts: &mut mpsc::Receiver<PromptInput>,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(prompts.try_recv().or_fail("a turn was started")?.input)
}

// ─────────────────────────────────────────────────────────────────────
// A new turn (rules 1 and 4)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A draft saved while a slow send was in flight survives the turn start
#[tokio::test]
async fn a_draft_saved_while_a_slow_send_was_in_flight_survives_the_turn_start() -> TestResult {
    // @step Given an idle session whose draft was cleared when "deploy the fix" was submitted
    let (session, mut prompts) = idle_session()?;
    session.set_pending_input(Some("deploy the fix".to_string()));
    session.set_pending_input(None);
    // @step And the new draft "next idea" was saved before the send reached the daemon
    session.set_pending_input(Some("next idea".to_string()));
    // @step When the send of "deploy the fix" starts the turn
    session
        .send_input("deploy the fix".to_string(), None)
        .map_err(|error| format!("the send was refused: {error}"))?;
    assert_eq!(started_turn(&mut prompts)?, "deploy the fix");
    // @step Then the session's draft is "next idea"
    assert_eq!(session.get_pending_input(), Some("next idea".to_string()));
    // @step And a client attaching to the session reads the draft "next idea"
    let attached = Arc::clone(&session);
    assert_eq!(attached.get_pending_input().as_deref(), Some("next idea"));
    Ok(())
}

// Scenario: Starting a turn clears the draft that was submitted
#[tokio::test]
async fn starting_a_turn_clears_the_draft_that_was_submitted() -> TestResult {
    // @step Given an idle session whose draft is "deploy the fix"
    let (session, mut prompts) = idle_session()?;
    session.set_pending_input(Some("deploy the fix".to_string()));
    // @step When the send of "deploy the fix" starts the turn
    session
        .send_input("deploy the fix".to_string(), None)
        .map_err(|error| format!("the send was refused: {error}"))?;
    assert_eq!(started_turn(&mut prompts)?, "deploy the fix");
    // @step Then the session has no draft
    assert_eq!(session.get_pending_input(), None);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Steer and queue (rule 2)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Steering a running turn keeps a newer draft and clears the submitted one
#[tokio::test]
async fn steering_a_running_turn_keeps_a_newer_draft_and_clears_the_submitted_one() -> TestResult {
    // @step Given a running session whose draft is "next idea"
    let (session, _prompts) = idle_session()?;
    session.set_status(SessionStatus::Running);
    session.set_pending_input(Some("next idea".to_string()));
    // @step When "also check the logs" is steered into the running turn
    session
        .submit_input("also check the logs".to_string(), None, InputMode::Steer)
        .map_err(|error| format!("the steer was refused: {error}"))?;
    // @step Then the session's draft is "next idea"
    assert_eq!(session.get_pending_input(), Some("next idea".to_string()));
    // @step When "next idea" is steered into the running turn
    session
        .submit_input("next idea".to_string(), None, InputMode::Steer)
        .map_err(|error| format!("the steer was refused: {error}"))?;
    // @step Then the session has no draft
    assert_eq!(session.get_pending_input(), None);
    Ok(())
}

// Scenario: Queueing behind a running turn keeps a newer draft and clears the submitted one
#[tokio::test]
async fn queueing_behind_a_running_turn_keeps_a_newer_draft_and_clears_the_submitted_one(
) -> TestResult {
    // @step Given a running session whose draft is "next idea"
    let (session, _prompts) = idle_session()?;
    session.set_status(SessionStatus::Running);
    session.set_pending_input(Some("next idea".to_string()));
    // @step When "task two" is queued behind the running turn
    session
        .submit_input("task two".to_string(), None, InputMode::Queue)
        .map_err(|error| format!("the queue was refused: {error}"))?;
    // @step Then the session's draft is "next idea"
    assert_eq!(session.get_pending_input(), Some("next idea".to_string()));
    // @step When "next idea" is queued behind the running turn
    session
        .submit_input("next idea".to_string(), None, InputMode::Queue)
        .map_err(|error| format!("the queue was refused: {error}"))?;
    // @step Then the session has no draft
    assert_eq!(session.get_pending_input(), None);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Turns not started from the draft (rule 3)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A queued input starting after the running turn keeps the current draft
#[tokio::test]
async fn a_queued_input_starting_after_the_running_turn_keeps_the_current_draft() -> TestResult {
    // @step Given a running session with "task two" queued behind its turn
    let (session, mut prompts) = idle_session()?;
    session.set_status(SessionStatus::Running);
    session
        .submit_input("task two".to_string(), None, InputMode::Queue)
        .map_err(|error| format!("the queue was refused: {error}"))?;
    // @step And the session's draft is "next idea"
    session.set_pending_input(Some("next idea".to_string()));
    // @step When the running turn ends and "task two" starts as the next turn
    session.set_status(SessionStatus::Idle);
    let started = session
        .continue_with_inbox()
        .map_err(|error| format!("the queued input was refused: {error}"))?;
    assert!(started, "the queued input must start as the next turn");
    assert_eq!(started_turn(&mut prompts)?, "task two");
    // @step Then the session's draft is "next idea"
    assert_eq!(session.get_pending_input(), Some("next idea".to_string()));
    Ok(())
}
