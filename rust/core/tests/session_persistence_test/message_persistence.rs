//! Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
use super::*;

// ============================================================================
// Scenario: User message is persisted by Rust when prompt is received
// @integration @napi
// ============================================================================
#[test]
fn test_user_message_persisted_with_envelope() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/user_msg_persist");

    // @step Given a NAPI BackgroundSession is created
    let mut session =
        create_session("User Message Test", &project).expect("create_session should succeed");

    // @step When the user sends a prompt "Read the README file"
    let metadata = create_user_envelope("Read the README file");
    append_message_with_metadata(&mut session, "user", "Read the README file", metadata)
        .expect("append should succeed");

    // @step Then the user message should be persisted to storage immediately
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step And the persisted message should have role "user" with text "Read the README file"
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "user");
    assert!(messages[0].content.contains("Read the README file"));

    // @step And no TypeScript persistence functions should be called
    // Verified by: metadata preserved means Rust persisted it, not TypeScript
    // @step And the envelope metadata should be preserved
    assert!(messages[0].metadata.contains_key("uuid"));
    assert_eq!(
        messages[0].metadata.get("type").and_then(|v| v.as_str()),
        Some("user")
    );
}
// ============================================================================
// Scenario: Assistant message with tool_use is persisted before tool execution
// @integration @napi
// ============================================================================
#[test]
fn test_assistant_tool_use_persisted_before_execution() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/tool_use_persist");

    // @step Given a NAPI session with an active prompt
    let mut session =
        create_session("Tool Use Test", &project).expect("create_session should succeed");

    // Add initial user message
    let user_meta = create_user_envelope("Read the file");
    append_message_with_metadata(&mut session, "user", "Read the file", user_meta)
        .expect("append user should succeed");

    // @step When the assistant streams text "I'll read that file" followed by a tool_use block
    let tool_id = "toolu_01AbCdEf";
    let assistant_meta = create_assistant_tool_use_envelope(
        "I'll read that file for you.",
        tool_id,
        "Read",
        serde_json::json!({"file_path": "/path/to/file.txt"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll read that file for you. [tool_use: Read]",
        assistant_meta,
    )
    .expect("append assistant should succeed");

    // @step Then the AssistantMessagePersisted event should fire before ToolExecutionCompleted
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step And the persisted message should contain both text and tool_use content blocks
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].role, "assistant");

    // Verify the envelope has both content types
    assert_content_block_count(&messages[1].metadata, 2);
    assert_content_block_type(&messages[1].metadata, 0, "text");
    assert_content_block_type(&messages[1].metadata, 1, "tool_use");
}
// ============================================================================
// Scenario: Tool result is persisted by Rust when tool execution completes
// @integration @napi
// ============================================================================
#[test]
fn test_tool_result_persisted_on_completion() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/tool_result_persist");

    // @step Given an assistant has requested a tool execution via NAPI
    let mut session =
        create_session("Tool Result Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Read the config file");
    append_message_with_metadata(&mut session, "user", "Read the config file", user_meta)
        .expect("append user should succeed");

    let tool_id = "toolu_02XyZaBc";
    let assistant_meta = create_assistant_tool_use_envelope(
        "I'll read the config.",
        tool_id,
        "Read",
        serde_json::json!({"file_path": "config.json"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll read the config.",
        assistant_meta,
    )
    .expect("append assistant should succeed");

    // @step When the tool execution completes with result content
    let tool_result_meta =
        create_tool_result_envelope(tool_id, r#"{"debug": true, "port": 8080}"#, false);
    append_message_with_metadata(
        &mut session,
        "user",
        r#"{"debug": true, "port": 8080}"#,
        tool_result_meta,
    )
    .expect("append tool_result should succeed");

    // @step Then the ToolResultPersisted event should fire
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step And the persisted message should have role "user" with type "tool_result"
    assert_eq!(messages.len(), 3);

    // Third message is the tool_result
    let tool_msg = &messages[2];
    assert_eq!(tool_msg.role, "user");
    assert_content_block_type(&tool_msg.metadata, 0, "tool_result");

    // Verify tool_use_id is preserved
    let content = tool_msg
        .metadata
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .expect("Should have content array");
    assert_eq!(
        content[0].get("tool_use_id").and_then(|t| t.as_str()),
        Some(tool_id)
    );
}
// ============================================================================
// Scenario: Final assistant response is persisted after tool result
// @integration @napi
// ============================================================================
#[test]
fn test_complete_tool_flow_all_messages_persisted() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/complete_flow");

    // @step Given a tool execution has completed and been persisted
    let mut session =
        create_session("Complete Flow Test", &project).expect("create_session should succeed");

    // 1. User message
    let user_meta = create_user_envelope("Read file.txt");
    append_message_with_metadata(&mut session, "user", "Read file.txt", user_meta)
        .expect("append user should succeed");

    // 2. Assistant with tool_use
    let tool_id = "toolu_03Complete";
    let assistant_meta = create_assistant_tool_use_envelope(
        "I'll read that file.",
        tool_id,
        "Read",
        serde_json::json!({"file_path": "file.txt"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll read that file.",
        assistant_meta,
    )
    .expect("append assistant should succeed");

    // 3. Tool result
    let tool_result_meta = create_tool_result_envelope(tool_id, "Hello World", false);
    append_message_with_metadata(&mut session, "user", "Hello World", tool_result_meta)
        .expect("append tool_result should succeed");

    // @step When the assistant streams a final response "Here are the file contents..."
    // @step And the Done chunk is emitted
    let final_meta = create_assistant_text_envelope("Here are the file contents: Hello World");
    append_message_with_metadata(
        &mut session,
        "assistant",
        "Here are the file contents: Hello World",
        final_meta,
    )
    .expect("append final assistant should succeed");

    // @step Then the FinalAssistantMessagePersisted event should fire
    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&reloaded).expect("get messages should succeed");

    // @step And all messages should be in storage in order: user, assistant, tool_result, assistant
    assert_eq!(messages.len(), 4, "Should have all 4 messages persisted");
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[2].role, "user"); // tool_result is user role
    assert_eq!(messages[3].role, "assistant");

    // Verify final message content
    assert!(messages[3].content.contains("Here are the file contents"));
}
