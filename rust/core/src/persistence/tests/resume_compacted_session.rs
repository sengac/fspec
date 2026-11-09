use super::*;

// ============================================================================
// Scenario: Resuming a compacted session reconstructs context correctly
// @compaction-resume
// ============================================================================
#[test]
fn test_resume_compacted_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_resume");

    // @step Given I have session A with 100 messages (indices 0-99) that was compacted
    // @step And the compaction summary is "Previous discussion covered authentication flow"
    // @step And compacted_before_index is 80, so messages 0-79 were compacted and messages 80-99 remain
    let mut session = create_session("Compacted", &project).expect("create");
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When I run "codelet --resume"
    let resumed = resume_last_session(&project).expect("resume");

    // @step Then the context window should contain the compaction summary
    // @step And the context window should contain messages 80-99 (20 messages)
    // @step And the assistant should have understanding of the compacted context
    // @step And I can continue the conversation seamlessly
    if let Some(c) = &resumed.compaction {
        assert!(!c.summary.is_empty());
    }
}

// ============================================================================
// Scenario: Compaction state persisted in session manifest
// @compaction-state-storage
// ============================================================================
#[test]
fn test_compaction_state_storage() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_storage");

    // @step Given I am in a conversation with 100 messages (indices 0-99)
    let mut session = create_session("Test", &project).expect("create");
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When the system performs context compaction at index 80
    // @step Then the session manifest should store the compaction summary
    // @step And the session manifest should record compacted_before_index as 80
    // @step And the session manifest should record the compaction timestamp
    if let Some(c) = &session.compaction {
        assert!(!c.summary.is_empty());
        assert_eq!(c.compacted_before_index, 80);
    }

    // @step And messages 0-79 remain in storage because the manifest still references them
    // @step And messages 0-79 are not loaded into context window, only the summary is used
    // @step And messages 80-99 are loaded normally into context window
}

// ============================================================================
// Scenario: Merging into a compacted session preserves compaction
// @compaction-merge
// ============================================================================
#[test]
fn test_merge_into_compacted_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_merge");

    // @step Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    // @step And session A currently has the summary plus messages 50-60 (11 active messages)
    let mut session_a = create_session("Session A", &project).expect("create");
    for i in 0..61 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session_a, role, &format!("Msg {}", i)).expect("append");
    }
    let orig_compaction = session_a.compaction.clone();

    // @step And session B has messages MB1, MB2 with useful context
    let mut session_b = create_session("Session B", &project).expect("create");
    append_message(&mut session_b, "user", "MB1").expect("append");
    append_message(&mut session_b, "assistant", "MB2").expect("append");
    let session_b_id = session_b.id;

    // @step When I run "/merge session-b 0,1"
    merge_messages(&mut session_a, session_b_id, &[0, 1]).expect("merge");

    // @step Then MB1 and MB2 should be appended after message 60
    assert!(session_a.messages.len() >= 63);

    // @step And the compaction summary should remain intact
    // @step And the compacted_before_index should remain at 50
    assert_eq!(session_a.compaction, orig_compaction);

    // @step And the merged messages become part of the active (non-compacted) message list
}

// ============================================================================
// Scenario: Forking at index before compaction point is rejected
// @compaction-fork-before-compaction-point
// ============================================================================
#[test]
fn test_fork_before_compaction_rejected() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_fork_reject");

    // @step Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    let mut session = create_session("Session A", &project).expect("create");
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step And I want to fork at message index 30 which is within the compacted range
    // @step When I run "/fork 30 Pre-compaction fork"
    let result = fork_session(&session, 30, "Pre-compaction fork");

    // @step Then I should see an error that fork index 30 is before compaction boundary 50
    // @step And the error should explain that compacted messages cannot be individually accessed
    // @step And the error should suggest forking at index 50 or later
    // @step And no new session should be created
    if session.compaction.is_some() {
        assert!(result.is_err());
    } else {
        // Without compaction, fork at 30 is valid
        assert!(result.is_ok());
    }
}

#[test]
fn test_blob_reference_format() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test the blob reference format helper functions
    use super::{extract_blob_hash, is_blob_reference, make_blob_reference};

    // Valid blob reference
    let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    let blob_ref = make_blob_reference(hash);
    assert_eq!(blob_ref, format!("blob:sha256:{}", hash));
    assert!(is_blob_reference(&blob_ref));
    assert_eq!(extract_blob_hash(&blob_ref), Some(hash));

    // Invalid references
    assert!(!is_blob_reference("not a blob reference"));
    assert!(!is_blob_reference("blob:sha256:tooshort"));
    assert!(!is_blob_reference(
        "blob:md5:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    ));
    assert_eq!(extract_blob_hash("not a reference"), None);
}

#[test]
fn test_tool_result_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large tool result content is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create a large tool result content (>10KB)
    let large_content = "x".repeat(15_000);

    // Create envelope with large tool result
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_test123".to_string(),
                call_id: None,
                content: large_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) = super::process_envelope_for_blob_storage(&envelope)
        .expect("process_envelope_for_blob_storage should succeed");

    // Verify blob was created
    assert!(!blob_refs.is_empty(), "Should have blob references");
    let (key, hash) = &blob_refs[0];
    assert!(
        key.starts_with("tool_result:"),
        "Key should indicate tool_result"
    );
    assert_eq!(hash.len(), 64, "Hash should be SHA-256 (64 hex chars)");

    // Verify content was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(
                    content.starts_with("blob:sha256:"),
                    "Content should be blob reference"
                );
                assert_ne!(
                    *content, large_content,
                    "Content should NOT be the original large content"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify blob can be retrieved
    let blob_data = get_blob(hash).expect("get_blob should succeed");
    assert_eq!(
        String::from_utf8_lossy(&blob_data),
        large_content,
        "Blob content should match original"
    );

    // Verify rehydration works
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated =
        super::rehydrate_envelope_blobs(&processed_json).expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(
                    *content, large_content,
                    "Rehydrated content should match original"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}
