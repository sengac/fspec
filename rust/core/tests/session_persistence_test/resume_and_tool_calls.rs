//! Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
use super::*;

// ============================================================================
// Scenario: Resumed session contains all messages including final responses
// @integration @napi
// ============================================================================
#[test]
fn test_session_resume_contains_all_messages() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/resume_all");

    // @step Given a completed session exists with messages: user, assistant+tool_use, tool_result, final_assistant
    let mut session =
        create_session("Resume All Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("List files");
    append_message_with_metadata(&mut session, "user", "List files", user_meta).unwrap();

    let tool_id = "toolu_04Resume";
    let assistant_meta = create_assistant_tool_use_envelope(
        "I'll list the files.",
        tool_id,
        "Ls",
        serde_json::json!({"path": "."}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll list the files.",
        assistant_meta,
    )
    .unwrap();

    let tool_result_meta = create_tool_result_envelope(tool_id, "file1.txt\nfile2.txt", false);
    append_message_with_metadata(
        &mut session,
        "user",
        "file1.txt\nfile2.txt",
        tool_result_meta,
    )
    .unwrap();

    let final_meta =
        create_assistant_text_envelope("The directory contains file1.txt and file2.txt.");
    append_message_with_metadata(
        &mut session,
        "assistant",
        "The directory contains file1.txt and file2.txt.",
        final_meta,
    )
    .unwrap();

    let session_id = session.id;
    drop(session);

    // @step When the user runs /resume and selects the session
    let resumed = load_session(session_id).expect("load should succeed");
    let messages = get_session_messages(&resumed).expect("get messages should succeed");

    // @step Then the MessagesRestored event should fire
    // @step And all four messages should be restored in order
    assert_eq!(messages.len(), 4, "All four messages should be restored");

    // @step And no messages should be truncated or missing
    assert!(messages[0].content.contains("List files"));
    assert!(messages[1].content.contains("I'll list the files"));
    assert!(messages[2].content.contains("file1.txt"));
    assert!(messages[3].content.contains("The directory contains"));

    // @step And the conversation should be fully visible
    for msg in &messages {
        assert!(
            !msg.content.is_empty(),
            "No message should have empty content"
        );
    }
}
// ============================================================================
// Scenario: Multiple tool uses in single assistant response are all persisted
// @integration @napi
// ============================================================================
#[test]
fn test_multiple_tool_uses_in_single_response() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/multi_tool");

    // @step Given a session with user prompt "Read file A and file B"
    let mut session =
        create_session("Multi Tool Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Read file A and file B");
    append_message_with_metadata(&mut session, "user", "Read file A and file B", user_meta)
        .unwrap();

    // @step When the assistant streams text with two tool_use blocks (read file A, read file B)
    let assistant_meta = create_assistant_multi_tool_envelope(
        "I'll read both files.",
        vec![
            ("tool_a", "Read", serde_json::json!({"file_path": "a.txt"})),
            ("tool_b", "Read", serde_json::json!({"file_path": "b.txt"})),
        ],
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll read both files.",
        assistant_meta,
    )
    .unwrap();

    // @step Then the assistant message should be persisted with both tool_use blocks
    // (Verified after full flow below)

    // @step And both tool results should be persisted after execution
    let tool_result_a = create_tool_result_envelope("tool_a", "Contents of A", false);
    append_message_with_metadata(&mut session, "user", "Contents of A", tool_result_a).unwrap();

    let tool_result_b = create_tool_result_envelope("tool_b", "Contents of B", false);
    append_message_with_metadata(&mut session, "user", "Contents of B", tool_result_b).unwrap();

    // @step And the final assistant response should be persisted
    let final_meta = create_assistant_text_envelope(
        "File A contains 'Contents of A' and file B contains 'Contents of B'.",
    );
    append_message_with_metadata(&mut session, "assistant", "Final response", final_meta).unwrap();

    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // @step And storage should contain: user, assistant(2 tools), tool_result, tool_result, assistant
    assert_eq!(
        messages.len(),
        5,
        "Should have 5 messages: user, assistant, tool_result x2, final assistant"
    );
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[2].role, "user"); // tool_result A
    assert_eq!(messages[3].role, "user"); // tool_result B
    assert_eq!(messages[4].role, "assistant"); // final response

    // Verify the assistant message has 2 tool_use blocks
    let tool_use_count = count_content_blocks_of_type(&messages[1].metadata, "tool_use");
    assert_eq!(tool_use_count, 2, "Should have 2 tool_use blocks persisted");
}
// ============================================================================
// Scenario: Sequential tool calls across multiple turns are persisted
// @integration @napi
// ============================================================================
#[test]
fn test_sequential_tool_calls_all_persisted() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/sequential_tools");

    // @step Given a session with an ongoing conversation
    let mut session =
        create_session("Sequential Tools Test", &project).expect("create_session should succeed");

    let user_meta = create_user_envelope("Read file A and file B");
    append_message_with_metadata(&mut session, "user", "Read file A and file B", user_meta)
        .unwrap();

    // @step When the assistant makes a tool call, gets result, makes another tool call
    // First tool call
    let assistant_meta1 = create_assistant_tool_use_envelope(
        "I'll read file A first.",
        "tool_a",
        "Read",
        serde_json::json!({"file_path": "a.txt"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "I'll read file A first.",
        assistant_meta1,
    )
    .unwrap();

    // First tool result
    let tool_result_a = create_tool_result_envelope("tool_a", "Contents of A", false);
    append_message_with_metadata(&mut session, "user", "Contents of A", tool_result_a).unwrap();

    // Second tool call (another assistant turn)
    let assistant_meta2 = create_assistant_tool_use_envelope(
        "Now I'll read file B.",
        "tool_b",
        "Read",
        serde_json::json!({"file_path": "b.txt"}),
    );
    append_message_with_metadata(
        &mut session,
        "assistant",
        "Now I'll read file B.",
        assistant_meta2,
    )
    .unwrap();

    // Second tool result
    let tool_result_b = create_tool_result_envelope("tool_b", "Contents of B", false);
    append_message_with_metadata(&mut session, "user", "Contents of B", tool_result_b).unwrap();

    // @step Then each intermediate assistant response should be persisted before the next tool
    // Final assistant response
    let final_meta = create_assistant_text_envelope(
        "File A contains 'Contents of A' and file B contains 'Contents of B'.",
    );
    append_message_with_metadata(&mut session, "assistant", "File A contains...", final_meta)
        .unwrap();

    let session_id = session.id;
    drop(session);

    let reloaded = load_session(session_id).expect("load");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // Verify: user, assistant1, tool_result1, assistant2, tool_result2, final_assistant
    assert_eq!(
        messages.len(),
        6,
        "Should have 6 messages for sequential tool calls"
    );
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "assistant"); // first tool call
    assert_eq!(messages[2].role, "user"); // tool_result A
    assert_eq!(messages[3].role, "assistant"); // second tool call
    assert_eq!(messages[4].role, "user"); // tool_result B
    assert_eq!(messages[5].role, "assistant"); // final response

    // @step And no "orphaned" tool_results should exist without following assistant responses
    // Each tool_result (messages[2] and [4]) is followed by an assistant message

    // @step And the session should never end with tool_result as the last message
    assert_eq!(
        messages.last().unwrap().role,
        "assistant",
        "Session should end with assistant response, not tool_result"
    );
}
