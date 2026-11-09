//! Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
use super::*;

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

// ============================================================================
// Scenario: Invalid session operations fail gracefully
// @integration @napi
// ============================================================================
#[test]
fn test_invalid_session_operations_fail_gracefully() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given an invalid session ID
    let invalid_id = uuid::Uuid::nil();

    // @step When attempting to load the session
    let result = load_session(invalid_id);

    // @step Then the operation should fail with a clear error
    assert!(result.is_err(), "Loading non-existent session should fail");

    // @step And the error message should be informative
    let error = result.unwrap_err();
    assert!(
        error.to_lowercase().contains("not found")
            || error.to_lowercase().contains("does not exist")
            || error.to_lowercase().contains("no such file"),
        "Error should indicate session not found: {error}"
    );
}
// ============================================================================
// Scenario: Concurrent persistence operations are serialized
// @integration @napi
// ============================================================================
#[test]
fn test_concurrent_persistence_serialized() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/concurrent");

    // @step Given a session with active operations
    let mut session =
        create_session("Concurrent Test", &project).expect("create_session should succeed");

    // @step When multiple messages are appended rapidly
    add_simple_alternating_messages(&mut session, 10);

    // @step Then all messages should be persisted in order
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    assert_eq!(messages.len(), 10);

    // @step And no messages should be lost or duplicated
    for (i, msg) in messages.iter().enumerate() {
        assert!(
            msg.content.contains(&format!("Message {i}")),
            "Message {i} should be at index {i}"
        );
    }
}
// ============================================================================
// Scenario: Compaction boundary validates message index
// @integration @napi
// ============================================================================
#[test]
fn test_compaction_boundary_validation() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_boundary");

    // @step Given a session with 5 messages
    let mut session =
        create_session("Boundary Test", &project).expect("create_session should succeed");

    add_simple_alternating_messages(&mut session, 5);

    // @step When compaction is set with a valid boundary
    let result = set_compaction_state(&mut session, "Summary of first 3 messages".to_string(), 3);

    // @step Then the compaction should succeed
    assert!(result.is_ok(), "Valid compaction boundary should succeed");

    // @step And the boundary should be recorded correctly
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("reload should succeed");

    assert!(reloaded.compaction.is_some());
    assert_eq!(
        reloaded.compaction.as_ref().unwrap().compacted_before_index,
        3
    );
}
// ============================================================================
// Scenario: Session manifest integrity after operations
// @integration @napi
// ============================================================================
#[test]
fn test_session_manifest_integrity() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/manifest_integrity");

    // @step Given a session with messages and token state
    let mut session =
        create_session("Integrity Test", &project).expect("create_session should succeed");

    append_message(&mut session, "user", "Test message").expect("append should succeed");

    update_session_tokens(
        &mut session,
        1000, // input (current context)
        500,  // output (cumulative)
        100,  // cache_read
        50,   // cache_creation
        0,    // reasoning_tokens (TOKEN-003)
    )
    .expect("update tokens should succeed");

    let session_id = session.id;
    let original_name = session.name.clone();
    drop(session);

    // @step When the session is reloaded multiple times
    for _ in 0..3 {
        let reloaded = load_session(session_id).expect("reload should succeed");

        // @step Then the manifest should maintain integrity
        assert_eq!(reloaded.name, original_name);
        assert_eq!(reloaded.token_usage.current_context_tokens, 1000);
        assert_eq!(reloaded.token_usage.cumulative_billed_output, 500);
        assert_eq!(reloaded.token_usage.cache_read_tokens, 100);
        assert_eq!(reloaded.token_usage.cache_creation_tokens, 50);

        // @step And no data should be corrupted
        let messages = get_session_messages(&reloaded).expect("get messages should succeed");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("Test message"));
    }
}
// ============================================================================
// Scenario: Message with metadata preserves all envelope fields
// @integration @napi
// ============================================================================
#[test]
fn test_message_metadata_preserved_across_reload() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/metadata_preserve");

    // @step Given a session with a message containing full envelope metadata
    let mut session =
        create_session("Metadata Test", &project).expect("create_session should succeed");

    let metadata = create_user_envelope("Test with metadata");
    let original_uuid = metadata
        .get("uuid")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();
    let original_timestamp = metadata
        .get("timestamp")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    append_message_with_metadata(&mut session, "user", "Test with metadata", metadata)
        .expect("append should succeed");

    let session_id = session.id;
    drop(session);

    // @step When the session is reloaded
    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step Then all metadata fields should be preserved
    assert_eq!(messages.len(), 1);
    let msg = &messages[0];

    assert_eq!(
        msg.metadata.get("uuid").and_then(|v| v.as_str()),
        Some(original_uuid.as_str()),
        "UUID should be preserved"
    );
    assert_eq!(
        msg.metadata.get("timestamp").and_then(|v| v.as_str()),
        Some(original_timestamp.as_str()),
        "Timestamp should be preserved"
    );
    assert_eq!(
        msg.metadata.get("type").and_then(|v| v.as_str()),
        Some("user"),
        "Type should be preserved"
    );
}
// ============================================================================
// Scenario: Empty message content handled correctly
// @integration @napi
// ============================================================================
#[test]
fn test_empty_message_content_handled() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/empty_content");

    // @step Given a session
    let mut session =
        create_session("Empty Content Test", &project).expect("create_session should succeed");

    // @step When appending a message with empty content
    let result = append_message(&mut session, "user", "");

    // @step Then the operation should succeed (empty messages are valid)
    assert!(result.is_ok(), "Empty message should be allowed");

    let session_id = session.id;
    drop(session);

    // @step And the empty message should be persisted and retrievable
    let reloaded = load_session(session_id).expect("reload should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.is_empty());
}
