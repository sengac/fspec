use super::*;

#[test]
fn test_one_byte_over_threshold_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content at 10KB + 1 byte IS stored in blob
    use chrono::Utc;
    use uuid::Uuid;

    // 10KB + 1 = 10241 bytes
    let just_over_10kb = "x".repeat(10_241);
    assert_eq!(just_over_10kb.len(), 10_241);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_over".to_string(),
                call_id: None,
                content: just_over_10kb,
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify blob WAS created
    assert!(!blob_refs.is_empty(), "10KB+1 should create blob");

    // Verify content was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(
                    content.starts_with("blob:sha256:"),
                    "Should be blob reference"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_tool_use_storage_and_retrieval() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that a simple ToolUse (no blob needed) is stored and retrieved correctly
    use chrono::Utc;
    use uuid::Uuid;

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::ToolUse {
                id: "toolu_bash123".to_string(),
                call_id: None,
                name: "Bash".to_string(),
                input: serde_json::json!({
                    "command": "ls -la",
                    "description": "List files"
                }),
            }],
            stop_reason: Some("tool_use".to_string()),
            usage: None,
        }),
        request_id: Some("req_test123".to_string()),
    };

    // Process for blob storage (should NOT create blobs for small content)
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify NO blob was created (content is small)
    assert!(
        blob_refs.is_empty(),
        "Small ToolUse should NOT create blobs"
    );

    // Verify ToolUse content is preserved
    match &processed.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::ToolUse {
                id, name, input, ..
            } => {
                assert_eq!(id, "toolu_bash123");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls -la");
                assert_eq!(input["description"], "List files");
            }
            _ => panic!("Expected ToolUse"),
        },
        _ => panic!("Expected Assistant message"),
    }

    // Verify round-trip through JSON serialization works
    let json = serde_json::to_string(&processed).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();

    match &restored.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::ToolUse {
                id, name, input, ..
            } => {
                assert_eq!(id, "toolu_bash123");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls -la");
                assert_eq!(input["description"], "List files");
            }
            _ => panic!("Expected ToolUse after restore"),
        },
        _ => panic!("Expected Assistant message after restore"),
    }
}

#[test]
fn test_set_session_tokens() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/token_set");

    // @step Given I have a session
    let mut session = create_session("Token Test", &project).expect("create");
    let session_id = session.id;

    // @step And the session has some initial token usage from messages
    append_message(&mut session, "user", "Hello").expect("append");
    append_message(&mut session, "assistant", "Hi there!").expect("append");

    // @step When I set the cumulative token usage to specific values
    // Pass cumulative values separately (here same as current for fresh session)
    set_session_tokens(
        &mut session,
        &TokenUsage {
            current_context_tokens: 1000,
            current_context_output_tokens: 0,
            cumulative_billed_input: 1000,
            cumulative_billed_output: 500,
            cache_read_tokens: 100,
            cache_creation_tokens: 50,
            reasoning_tokens: 0,
        },
    )
    .expect("set tokens");

    // @step Then the session should have exactly those token values (not added)
    // CTX-003: Now uses dual-metric fields
    assert_eq!(session.token_usage.current_context_tokens, 1000);
    assert_eq!(session.token_usage.cumulative_billed_input, 1000);
    assert_eq!(session.token_usage.cumulative_billed_output, 500);
    assert_eq!(session.token_usage.cache_read_tokens, 100);
    assert_eq!(session.token_usage.cache_creation_tokens, 50);
    assert_eq!(session.token_usage.reasoning_tokens, 0);

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the token values should be persisted
    // CTX-003: Now uses dual-metric fields
    assert_eq!(reloaded.token_usage.current_context_tokens, 1000);
    assert_eq!(reloaded.token_usage.cumulative_billed_input, 1000);
    assert_eq!(reloaded.token_usage.cumulative_billed_output, 500);
    assert_eq!(reloaded.token_usage.cache_read_tokens, 100);
    assert_eq!(reloaded.token_usage.cache_creation_tokens, 50);
}

#[test]
fn test_set_compaction_state() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_set");

    // @step Given I have a session with messages
    let mut session = create_session("Compaction Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step And the session has no compaction state initially
    assert!(session.compaction.is_none());

    // @step When I set the compaction state after context compaction
    let summary = "Compacted 8 turns (5000→1500 tokens, 70% compression)".to_string();
    set_compaction_state(&mut session, summary.clone(), 8).expect("set compaction");

    // @step Then the session should have the compaction state
    assert!(session.compaction.is_some());
    let compaction = session.compaction.as_ref().unwrap();
    assert_eq!(compaction.summary, summary);
    assert_eq!(compaction.compacted_before_index, 8);

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the compaction state should be persisted
    assert!(reloaded.compaction.is_some());
    let reloaded_compaction = reloaded.compaction.as_ref().unwrap();
    assert_eq!(reloaded_compaction.summary, summary);
    assert_eq!(reloaded_compaction.compacted_before_index, 8);
}

#[test]
fn test_clear_compaction_state() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_clear");

    // @step Given I have a session with compaction state
    let mut session = create_session("Compaction Clear Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }
    set_compaction_state(&mut session, "Test summary".to_string(), 5).expect("set compaction");
    assert!(session.compaction.is_some());

    // @step When I clear the compaction state
    clear_compaction_state(&mut session).expect("clear compaction");

    // @step Then the session should have no compaction state
    assert!(session.compaction.is_none());

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the compaction should still be cleared
    assert!(reloaded.compaction.is_none());
}
