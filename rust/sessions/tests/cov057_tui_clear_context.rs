//! Feature: spec/features/tui-clear-command-only-clears-react-state-not-actual-session-context.feature
//!
//! COV-057: the TUI `/clear` dispatches `backend.clear_history(session)`,
//! which lands on `SessionManagerHandle::clear_history` ->
//! `BackgroundSession::clear_history`. These tests drive that production path
//! on a real manager-owned session and observe what the model receives on
//! the NEXT turn through an offline wiremock Anthropic endpoint
//! (cov057_support/wire.rs), plus the backend state the TUI reads.

#[path = "cmpct060/support.rs"]
pub mod support;
#[path = "cov057_support/wire.rs"]
pub mod wire;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionState, StreamChunk};
use support::{manager, TestResult};
use wire::turn;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_ai_has_no_memory_of_prior_conversation_after_clear() -> TestResult {
    // @step Given I have a TUI session with an active conversation
    let (_directory, manager, session_id, _) = manager().await?;
    // @step And I have discussed "topic X" with the AI for 5 turns
    for index in 0..5 {
        let prompt = format!("Let us keep discussing topic X, part {index}");
        turn(&manager, &session_id, &prompt, "More about topic X", 100).await?;
    }
    let session = manager.get_session(&session_id.value)?;
    let before = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert_eq!(before.matches("More about topic X").count(), 5, "{before}");
    // @step When I type "/clear" and press Enter
    SessionManagerHandle::clear_history(&*manager, &session_id)?;
    // @step And I ask "what were we talking about?"
    let wire = turn(
        &manager,
        &session_id,
        "what were we talking about?",
        "I have no record of an earlier topic.",
        20,
    )
    .await?;
    // @step Then the AI should NOT know about "topic X"
    let encoded = wire.to_string();
    assert!(encoded.contains("what were we talking about?"), "{wire}");
    assert!(
        !encoded.contains("topic X"),
        "cleared history leaked: {wire}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_system_reminders_preserved_after_clear() -> TestResult {
    // @step Given I have a TUI session with AGENTS.md loaded
    // BUG-598: AGENTS.md comes from the session's own directory, so the
    // session is created over a project holding one.
    let (directory, manager, _, _) = manager().await?;
    let marker = "This project is fspec (AGENTS.md marker)".to_owned();
    std::fs::write(directory.path().join("AGENTS.md"), format!("{marker}\n"))?;
    let session_id = codelet_rpc_types::SessionId::new(
        manager
            .create_session(
                "anthropic/claude-sonnet-4",
                &directory.path().to_string_lossy(),
            )
            .await?,
    );
    let session = manager.get_session(&session_id.value)?;
    let before = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert!(before.contains(&marker), "AGENTS.md was not loaded");
    // @step And environment info shows the current date
    let date_line = format!("Date: {}", chrono::Local::now().format("%Y-%m-%d"));
    assert!(before.contains(&date_line), "no current date in {before}");
    // @step When I type "/clear" and press Enter
    SessionManagerHandle::clear_history(&*manager, &session_id)?;
    // @step And I ask "what project are you working on?"
    let wire = turn(
        &manager,
        &session_id,
        "what project are you working on?",
        "The fspec project.",
        20,
    )
    .await?;
    let encoded = wire.to_string();
    // @step Then the AI should still know it's working on fspec project
    assert!(
        encoded.contains(&marker),
        "AGENTS.md lost after clear: {wire}"
    );
    // @step And the AI should still know the current date
    assert!(
        encoded.contains(&date_line),
        "date lost after clear: {wire}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_token_counters_reset_after_clear() -> TestResult {
    // @step Given I have a TUI session with token usage showing 5000 input tokens
    let (_directory, manager, session_id, _) = manager().await?;
    let session = manager.get_session(&session_id.value)?;
    session.inner.lock().await.token_tracker.input_tokens = 5000;
    // The live counters clients read (get_session_tokens), as a turn sets them.
    session.update_tokens(5000, 120);
    let mut events = session.subscribe_to_stream();
    // @step When I type "/clear" and press Enter
    SessionManagerHandle::clear_history(&*manager, &session_id)?;
    // @step Then the displayed token counter should show 0
    // The TUI zeroes its displayed counter on the SessionStateChange(Cleared)
    // chunk (fspec-tui dispatch_stream_chunks, RPC-100).
    let mut cleared = false;
    while let Ok(chunk) = events.try_recv() {
        cleared |= matches!(
            chunk,
            StreamChunk::SessionStateChange {
                state: SessionState::Cleared
            }
        );
    }
    assert!(cleared, "clear_history emitted no Cleared state change");
    // @step And the Rust session token tracker should show 0
    let inner = session.inner.lock().await;
    assert_eq!(inner.token_tracker.input_tokens, 0);
    assert_eq!(inner.token_tracker.output_tokens, 0);
    drop(inner);
    // BUG-321: the per-session counters every client reads reset too.
    let tokens = SessionManagerHandle::get_session_tokens(&*manager, &session_id)?;
    assert_eq!((tokens.input_tokens, tokens.output_tokens), (0, 0));
    Ok(())
}
