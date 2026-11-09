use super::*;

#[test]
fn test_token_and_compaction_state_persist_together() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/full_state");

    // @step Given I have a session
    let mut session = create_session("Full State Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When I set both token usage and compaction state
    set_session_tokens(
        &mut session,
        &TokenUsage {
            current_context_tokens: 2000,
            current_context_output_tokens: 0,
            cumulative_billed_input: 2000,
            cumulative_billed_output: 1000,
            cache_read_tokens: 200,
            cache_creation_tokens: 100,
            reasoning_tokens: 0,
        },
    )
    .expect("set tokens");
    set_compaction_state(&mut session, "Compacted 15 turns".to_string(), 15)
        .expect("set compaction");

    // @step And I reload the session
    let reloaded = load_session(session_id).expect("reload");

    // @step Then both token usage and compaction state should be restored
    // CTX-003: Now uses dual-metric fields
    assert_eq!(reloaded.token_usage.current_context_tokens, 2000);
    assert_eq!(reloaded.token_usage.cumulative_billed_input, 2000);
    assert_eq!(reloaded.token_usage.cumulative_billed_output, 1000);
    assert!(reloaded.compaction.is_some());
    assert_eq!(
        reloaded.compaction.as_ref().unwrap().compacted_before_index,
        15
    );

    // @step And the message count should still be correct
    assert_eq!(reloaded.messages.len(), 20);
}

// ============================================================================
// CRITICAL FIX: get_session_messages respects compaction state
// Tests that restored sessions use compacted context, not full history
// ============================================================================
#[test]
fn test_get_session_messages_respects_compaction() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_restore");

    // @step Given I have a session with 20 messages
    let mut session = create_session("Compaction Test", &project).expect("create");
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has been compacted at index 15 with a summary
    let summary = "Previous conversation covered messages 0-14, discussing authentication flow.";
    set_compaction_state(&mut session, summary.to_string(), 15).expect("set compaction");

    // @step When I retrieve messages using get_session_messages
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // @step Then I should receive:
    // - 1 synthetic summary message
    // - 5 messages from index 15 onward (messages 15, 16, 17, 18, 19)
    // Total: 6 messages (not 20!)
    assert_eq!(
        messages.len(),
        6,
        "Expected 1 summary + 5 post-compaction messages, got {}",
        messages.len()
    );

    // @step And the first message should be the synthetic summary
    let first_msg = &messages[0];
    assert!(
        first_msg.content.contains("Previous conversation summary"),
        "First message should be synthetic summary"
    );
    assert!(
        first_msg.content.contains(summary),
        "Summary content should be included"
    );
    assert_eq!(
        first_msg.id,
        uuid::Uuid::nil(),
        "Synthetic message should have nil UUID"
    );
    assert_eq!(
        first_msg.role, "user",
        "Synthetic summary should be 'user' role"
    );

    // @step And the remaining messages should be the post-compaction messages
    assert!(
        messages[1].content.contains("Message 15"),
        "Second message should be original message 15"
    );
    assert!(
        messages[5].content.contains("Message 19"),
        "Last message should be original message 19"
    );
}

#[test]
fn test_get_session_messages_full_ignores_compaction() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/full_restore_unique");

    // @step Given I have a session with 10 messages
    let mut session = create_session("Full Restore Test Unique", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("FullRestore Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // Verify we have 10 messages before compaction
    assert_eq!(
        session.messages.len(),
        10,
        "Should have 10 messages initially"
    );

    // @step And the session has been compacted at index 8
    set_compaction_state(&mut session, "Summary of 0-7".to_string(), 8).expect("set compaction");

    // @step When I retrieve messages using get_session_messages_full
    let reloaded = load_session(session_id).expect("reload");

    // Verify reloaded session still has 10 message refs
    assert_eq!(
        reloaded.messages.len(),
        10,
        "Reloaded session should have 10 message refs"
    );

    let full_messages = get_session_messages_full(&reloaded).expect("get full messages");

    // @step Then I should receive all 10 original messages (no synthetic summary)
    assert_eq!(
        full_messages.len(),
        10,
        "get_session_messages_full should return all messages, got {}",
        full_messages.len()
    );

    // @step And the first message should be the original message 0
    assert!(
        full_messages[0].content.contains("FullRestore Message 0"),
        "First message should be original message 0, got: {}",
        full_messages[0].content
    );

    // @step Compare with compaction-aware retrieval
    let compacted_messages = get_session_messages(&reloaded).expect("get compacted messages");
    assert_eq!(
        compacted_messages.len(),
        3,
        "get_session_messages should return 1 summary + 2 post-compaction"
    );
}

#[test]
fn test_get_session_messages_no_compaction_returns_all() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/no_compaction");

    // @step Given I have a session with 5 messages and NO compaction
    let mut session = create_session("No Compaction Test", &project).expect("create");
    for i in 0..5 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has no compaction state
    assert!(session.compaction.is_none());

    // @step When I retrieve messages using get_session_messages
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // @step Then I should receive all 5 messages (no synthetic summary)
    assert_eq!(
        messages.len(),
        5,
        "Should return all messages when no compaction"
    );

    // @step And they should be the original messages
    assert!(messages[0].content.contains("Message 0"));
    assert!(messages[4].content.contains("Message 4"));
}
