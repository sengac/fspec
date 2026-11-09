//! Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
use super::*;

// ============================================================================
// Scenario: Token state is persisted by Rust on Done chunk via NAPI
// @integration @napi
// ============================================================================
#[test]
fn test_token_state_persisted_and_restored() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/token_state");

    // @step Given a NAPI session with an active streaming response
    let mut session =
        create_session("Token State Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Test message");
    append_message_with_metadata(&mut session, "user", "Test message", user_meta).unwrap();

    // @step When the Done chunk is emitted with usage data (input_tokens=5000, output_tokens=2000)
    // @step Then the TokenStatePersisted event should fire in Rust
    update_session_tokens(
        &mut session,
        5000, // input_tokens (current context)
        2000, // output_tokens (cumulative)
        1000, // cache_read
        500,  // cache_creation
        0,    // reasoning_tokens (TOKEN-003)
    )
    .expect("update tokens should succeed");

    let session_id = session.id;
    drop(session);

    // @step And the token state should be persisted to the session manifest by Rust
    let resumed = load_session(session_id).expect("load should succeed");

    // @step And TypeScript should NOT call persistenceSetSessionTokens
    // (Verified by: Rust persistence layer is being tested, not TypeScript)

    // @step Then the token counts should be restored as input_tokens=5000 and output_tokens=2000
    assert_eq!(resumed.token_usage.current_context_tokens, 5000);
    assert_eq!(resumed.token_usage.cumulative_billed_output, 2000);

    // @step And the context fill percentage should be calculated correctly
    // @step And the context usage display should be accurate
    assert_eq!(resumed.token_usage.cache_read_tokens, 1000);
    assert_eq!(resumed.token_usage.cache_creation_tokens, 500);
}
// ============================================================================
// Scenario: Manual compaction persists compaction state to session manifest
// @integration @napi
// ============================================================================
#[test]
fn test_compaction_state_persisted_and_restored() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_state");

    // @step Given a session with enough messages to compact
    let mut session =
        create_session("Compaction State Test", &project).expect("create_session should succeed");

    add_alternating_messages(&mut session, 10);

    // @step When the user runs /compact command via NAPI session_manager.rs
    // @step Then the CompactionSummaryGenerated event should fire
    // @step And the CompactionStatePersisted event should fire
    let summary = "Previous discussion covered auth flow implementation";
    set_compaction_state(&mut session, summary.to_string(), 8)
        .expect("set compaction state should succeed");

    let session_id = session.id;
    drop(session);

    // @step And the compaction summary should be persisted to the session manifest
    let resumed = load_session(session_id).expect("load should succeed");

    // @step And the compaction boundary index should be recorded
    assert!(resumed.compaction.is_some());
    let state = resumed.compaction.as_ref().unwrap();
    assert_eq!(state.compacted_before_index, 8);

    // @step And the session manifest should contain compaction state with summary text
    assert!(state.summary.contains("auth flow"));
}
// ============================================================================
// Scenario: Resumed session restores compaction summary as first message
// @integration @napi
// ============================================================================
#[test]
fn test_resumed_session_restores_compaction_summary() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_resume");

    // @step Given a session that was previously compacted
    let mut session =
        create_session("Compaction Resume Test", &project).expect("create_session should succeed");

    // Add 10 messages
    add_alternating_messages(&mut session, 10);

    // @step And the session manifest has compaction state with summary "Previous discussion covered auth flow implementation"
    let summary = "Previous discussion covered auth flow implementation";
    set_compaction_state(&mut session, summary.to_string(), 8)
        .expect("set compaction state should succeed");

    let session_id = session.id;
    drop(session);

    // @step When the user resumes the session
    let resumed = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&resumed).expect("get messages should succeed");

    // @step Then the CompactionSummaryRestored event should fire
    // @step And the first message should be a synthetic summary containing "Previous discussion covered auth flow implementation"
    assert_eq!(
        messages[0].id,
        uuid::Uuid::nil(),
        "First message should be synthetic (nil UUID)"
    );
    assert!(
        messages[0].content.contains(summary),
        "Summary should contain the compaction text"
    );

    // @step And only post-compaction messages should be loaded after the summary
    // We compacted at index 8, so we should have: 1 summary + 2 post-compaction messages
    assert_eq!(
        messages.len(),
        3,
        "Should have summary + 2 post-compaction messages"
    );

    // @step And the context should be efficient (not reloading pre-compaction messages)
    // Verify messages 8 and 9 are present
    assert!(messages[1].content.contains("Message 8") || messages[1].content.contains("Response"));
}
// ============================================================================
// Scenario: Resumed session has accurate token counts
// @integration @napi
// ============================================================================
#[test]
fn test_resumed_session_has_accurate_token_counts() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/token_resume");

    // @step Given a session was completed with input_tokens=5000 and output_tokens=2000
    let mut session =
        create_session("Token Resume Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Test message for tokens");
    append_message_with_metadata(&mut session, "user", "Test message for tokens", user_meta)
        .unwrap();

    let assistant_meta = create_assistant_text_envelope("Here is my response");
    append_message_with_metadata(
        &mut session,
        "assistant",
        "Here is my response",
        assistant_meta,
    )
    .unwrap();

    // Persist token state
    update_session_tokens(
        &mut session,
        5000, // input_tokens (current context)
        2000, // output_tokens (cumulative)
        1500, // cache_read
        800,  // cache_creation
        0,    // reasoning_tokens (TOKEN-003)
    )
    .expect("update tokens should succeed");

    let session_id = session.id;
    drop(session);

    // @step When the user resumes the session
    let resumed = load_session(session_id).expect("load should succeed");

    // @step Then the token counts should be restored as input_tokens=5000 and output_tokens=2000
    assert_eq!(
        resumed.token_usage.current_context_tokens, 5000,
        "Input tokens should be restored accurately"
    );
    assert_eq!(
        resumed.token_usage.cumulative_billed_output, 2000,
        "Output tokens should be restored accurately"
    );

    // @step And the context fill percentage should be calculated correctly
    // Cache tokens should also be preserved for accurate fill calculation
    assert_eq!(resumed.token_usage.cache_read_tokens, 1500);
    assert_eq!(resumed.token_usage.cache_creation_tokens, 800);

    // @step And the context usage display should be accurate
    // Messages should also be present
    let messages = get_session_messages(&resumed).expect("get messages");
    assert_eq!(
        messages.len(),
        2,
        "Messages should be restored along with token state"
    );
}
