//! Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
use super::*;

// ============================================================================
// Scenario: Tool result with error flag persisted correctly
// @integration @napi
// ============================================================================
#[test]
fn test_tool_result_error_flag_persisted() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/tool_error");

    // @step Given a session with a tool execution
    let mut session =
        create_session("Tool Error Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Run command");
    append_message_with_metadata(&mut session, "user", "Run command", user_meta).unwrap();

    let tool_id = "toolu_error_test";
    let assistant_meta = create_assistant_tool_use_envelope(
        "Running command...",
        tool_id,
        "Bash",
        serde_json::json!({"command": "invalid_command"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "Running command...",
        assistant_meta,
    )
    .unwrap();

    // @step When the tool execution fails with an error
    let error_result = create_tool_result_envelope(
        tool_id,
        "Command not found: invalid_command",
        true, // is_error = true
    );
    append_message_with_metadata(&mut session, "user", "Command not found", error_result).unwrap();

    let session_id = session.id;
    drop(session);

    // @step Then the error flag should be preserved in the persisted message
    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    assert_eq!(messages.len(), 3);

    let tool_result = &messages[2];
    let content = tool_result
        .metadata
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .expect("Should have content array");

    let is_error = content[0]
        .get("is_error")
        .and_then(serde_json::Value::as_bool);
    assert_eq!(is_error, Some(true), "is_error flag should be true");
}
// ============================================================================
// Scenario: Large session maintains message order
// @integration @napi
// ============================================================================
#[test]
fn test_large_session_maintains_message_order() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/large_session");

    // @step Given a session with 50 messages
    let mut session =
        create_session("Large Session Test", &project).expect("create_session should succeed");

    add_alternating_messages(&mut session, 50);

    let session_id = session.id;
    drop(session);

    // @step When the session is reloaded
    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step Then all 50 messages should be present in correct order
    assert_eq!(messages.len(), 50, "All 50 messages should be present");

    for (i, msg) in messages.iter().enumerate() {
        let expected_role = if i % 2 == 0 { "user" } else { "assistant" };
        assert_eq!(
            msg.role, expected_role,
            "Message {i} should have role {expected_role}"
        );
        assert!(
            msg.content.contains(&format!("Message {i}"))
                || msg.content.contains(&format!("message {i}")),
            "Message {i} should contain its index"
        );
    }
}
// ============================================================================
// Scenario: Compaction with boundary at message count is valid
// @integration @napi
// ============================================================================
#[test]
fn test_compaction_at_message_count_boundary() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_at_end");

    // @step Given a session with exactly 5 messages
    let mut session =
        create_session("Compaction End Test", &project).expect("create_session should succeed");

    add_simple_alternating_messages(&mut session, 5);

    // @step When compaction is set at the exact message count (all messages compacted)
    let result = set_compaction_state(
        &mut session,
        "Summary of all 5 messages".to_string(),
        5, // Compact all messages
    );

    // @step Then the compaction should succeed
    assert!(
        result.is_ok(),
        "Compaction at message count should be valid"
    );

    let session_id = session.id;
    drop(session);

    // @step And when resuming, only the summary should be returned
    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // Only synthetic summary message (all original messages are "before" the compaction point)
    assert_eq!(messages.len(), 1, "Should only have synthetic summary");
    assert_eq!(
        messages[0].id,
        uuid::Uuid::nil(),
        "Should be synthetic message"
    );
}
