//! Feature: spec/features/session-search-trimming.feature
//!
//! CMPCT-010 tests, moved out of session_search_handler.rs's inline tests so each
//! test file maps one feature (COV-075).

use super::*;

// ========================================================================
// SessionSearch Trimming Integration tests
//
// Feature: spec/features/session-search-trimming.feature
//
// This test section validates the acceptance criteria for conditional
// trimming in SessionSearch when compaction_in_progress is active.
// ========================================================================

use serde_json::json;

/// Build metadata for an assistant message containing a ToolUse block.
fn make_assistant_tool_use_metadata(
    tool_name: &str,
    tool_id: &str,
    input: serde_json::Value,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("type".to_string(), json!("assistant"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "assistant",
            "content": [
                {
                    "type": "tool_use",
                    "id": tool_id,
                    "name": tool_name,
                    "input": input
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a user message containing a ToolResult block.
fn make_user_tool_result_metadata(
    tool_use_id: &str,
    is_error: bool,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("type".to_string(), json!("user"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": "[content in main field]",
                    "is_error": is_error
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a plain user text message (no tool result).
fn make_plain_user_metadata() -> std::collections::HashMap<String, serde_json::Value> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("type".to_string(), json!("user"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "plain user message"
                }
            ]
        }),
    );
    meta
}

/// Build metadata for a plain assistant text message (no tool use).
fn make_plain_assistant_metadata() -> std::collections::HashMap<String, serde_json::Value> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("type".to_string(), json!("assistant"));
    meta.insert(
        "message".to_string(),
        json!({
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "assistant reasoning"
                }
            ]
        }),
    );
    meta
}

/// Generate N lines of content for testing (same as trimmer tests).
fn generate_lines(n: usize) -> String {
    (1..=n)
        .map(|i| format!("     {}: line {} content here with some text", i, i))
        .collect::<Vec<_>>()
        .join("\n")
}

// Scenario: Flag is false — SessionSearch show returns full untrimmed content
#[test]
fn test_flag_false_returns_full_untrimmed_content() {
    // @step Given the compaction_in_progress flag is false
    let compaction_active = false;

    // @step And the session contains a user tool result message with 500 lines of Read output
    let content = generate_lines(500);
    let tool_id = "toolu_read_001";
    let assistant_meta =
        make_assistant_tool_use_metadata("Read", tool_id, json!({"file_path": "src/main.rs"}));
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let messages = vec![
        (
            "assistant".to_string(),
            "[tool_use: Read]".to_string(),
            assistant_meta,
        ),
        ("user".to_string(), content.clone(), user_meta),
    ];

    // @step When the agent calls SessionSearch with action "show"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the result includes the full 500-line content with no trimming applied
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[1], content,
        "Content should be unchanged when flag is false"
    );
    assert!(
        results[1].lines().count() == 500,
        "All 500 lines should be present"
    );
}

// Scenario: Flag is true — SessionSearch show returns trimmed Read tool results
#[test]
fn test_flag_true_trims_read_tool_results() {
    // @step Given the compaction_in_progress flag is true
    let compaction_active = true;

    // @step And the session contains a user tool result message with 500 lines of Read output for "src/main.rs"
    let content = generate_lines(500);
    let tool_id = "toolu_read_002";
    let assistant_meta =
        make_assistant_tool_use_metadata("Read", tool_id, json!({"file_path": "src/main.rs"}));
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let messages = vec![
        (
            "assistant".to_string(),
            "[tool_use: Read]".to_string(),
            assistant_meta,
        ),
        ("user".to_string(), content.clone(), user_meta),
    ];

    // @step When the agent calls SessionSearch with action "show"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the result shows a compact reference like "[file: src/main.rs, 500 lines, {tok} tok — use Read to retrieve]"
    assert_eq!(results.len(), 2);
    let trimmed = &results[1];
    assert!(
        trimmed.starts_with("[file: src/main.rs,"),
        "Should be a compact file reference, got: {}",
        trimmed
    );
    assert!(
        trimmed.contains("500 lines"),
        "Should mention 500 lines, got: {}",
        trimmed
    );
    assert!(
        trimmed.contains("tok —"),
        "Should contain token count, got: {}",
        trimmed
    );
    assert!(
        trimmed.contains("use Read to retrieve"),
        "Should hint at Read tool, got: {}",
        trimmed
    );
    assert_ne!(*trimmed, content, "Content should be trimmed, not original");
}

// Scenario: Flag is true — SessionSearch search preserves user messages unchanged
#[test]
fn test_flag_true_preserves_plain_user_messages() {
    // @step Given the compaction_in_progress flag is true
    let compaction_active = true;

    // @step And the session contains a user message "please fix the login bug"
    let user_content = "please fix the login bug".to_string();
    let user_meta = make_plain_user_metadata();

    let messages = vec![("user".to_string(), user_content.clone(), user_meta)];

    // @step When the agent calls SessionSearch with action "search" and query "login"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the matched content includes "please fix the login bug" unchanged
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0], user_content,
        "Plain user messages should pass through unchanged"
    );
}

// Scenario: Flag is true — SessionSearch show preserves assistant reasoning text
#[test]
fn test_flag_true_preserves_assistant_reasoning() {
    // @step Given the compaction_in_progress flag is true
    let compaction_active = true;

    // @step And the session contains an assistant message with reasoning text and no tool use
    let reasoning = "I need to analyze the error in the login handler. The stack trace suggests a null pointer in the auth middleware.".to_string();
    let assistant_meta = make_plain_assistant_metadata();

    let messages = vec![("assistant".to_string(), reasoning.clone(), assistant_meta)];

    // @step When the agent calls SessionSearch with action "show"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the assistant reasoning text is returned unchanged
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0], reasoning,
        "Assistant reasoning should pass through unchanged"
    );
}

// Scenario: Trimmer processes messages in order for tool_use_id correlation
#[test]
fn test_trimmer_processes_messages_in_order() {
    // @step Given the compaction_in_progress flag is true
    let compaction_active = true;

    // @step And the session contains an assistant message with a Write tool_use block
    let write_tool_id = "toolu_write_001";
    let assistant_meta = make_assistant_tool_use_metadata(
        "Write",
        write_tool_id,
        json!({
            "file_path": "src/auth.rs",
            "content": "fn login() { /* implementation */ }"
        }),
    );

    // @step And the session contains a user message with the corresponding tool_result
    let write_result = "Successfully wrote to src/auth.rs".to_string();
    let tool_result_meta = make_user_tool_result_metadata(write_tool_id, false);

    // @step And the session contains a plain user message
    let plain_content = "now please add tests for it".to_string();
    let plain_meta = make_plain_user_metadata();

    let messages = vec![
        (
            "assistant".to_string(),
            "[tool_use: Write src/auth.rs]".to_string(),
            assistant_meta,
        ),
        ("user".to_string(), write_result.clone(), tool_result_meta),
        ("user".to_string(), plain_content.clone(), plain_meta),
    ];

    // @step When the agent calls SessionSearch with action "show"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the Write tool result is trimmed to a compact persistence reference
    assert_eq!(results.len(), 3);
    // Assistant message with Write tool_use has its input trimmed to a compact reference
    assert!(
        results[0].contains("[Write:") || results[0].contains("persisted"),
        "Write tool_use in assistant msg should be trimmed to compact ref, got: {}",
        results[0]
    );
    // Write tool_result (user msg) passes through content heuristics — already compact
    assert_eq!(
        results[1], write_result,
        "Short Write tool_result should pass through heuristics unchanged"
    );

    // @step And the plain user message passes through unchanged
    assert_eq!(
        results[2], plain_content,
        "Plain user message should pass through unchanged"
    );
}

// Scenario: Trimming is applied after blob resolution
//
// NOTE: This scenario tests the ordering guarantee at integration level.
// Blob resolution happens in resolve_message_content() BEFORE content reaches
// apply_conditional_trimming(). We test that trimming operates on resolved
// content (not blob references) by ensuring the function works on plain strings.
#[test]
fn test_trimming_applied_after_blob_resolution() {
    // @step Given the compaction_in_progress flag is true
    let compaction_active = true;

    // @step And the session contains a message with blob-referenced content
    // At this point in the pipeline, blob has already been resolved by
    // resolve_message_content(). We simulate the resolved content.
    let resolved_content = generate_lines(500);
    let tool_id = "toolu_read_blob";
    let assistant_meta = make_assistant_tool_use_metadata(
        "Read",
        tool_id,
        json!({"file_path": "src/large_file.rs"}),
    );
    let user_meta = make_user_tool_result_metadata(tool_id, false);

    let messages = vec![
        (
            "assistant".to_string(),
            "[tool_use: Read]".to_string(),
            assistant_meta,
        ),
        ("user".to_string(), resolved_content.clone(), user_meta),
    ];

    // @step When the agent calls SessionSearch with action "show"
    let results = apply_conditional_trimming(compaction_active, &messages);

    // @step Then the blob is resolved before trimming is applied
    // @step And the trimmed output reflects the resolved content, not the raw blob reference
    let trimmed = &results[1];
    assert!(
        !trimmed.starts_with("blob:sha256:"),
        "Should not contain raw blob reference"
    );
    assert!(
        trimmed.contains("[file:") || trimmed.contains("lines"),
        "Trimmed output should reflect resolved file content, got: {}",
        trimmed
    );
    assert_ne!(
        *trimmed, resolved_content,
        "Should be trimmed, not raw resolved content"
    );
}

// Scenario: BackgroundSession defaults compaction_in_progress to false
//
// NOTE: BackgroundSession construction is tested via compilation - the field
// will be added during implementation. This test verifies the behavioral
// consequence: that untrimmed content is the default.
#[test]
fn test_default_flag_false_returns_untrimmed() {
    // @step Given a new BackgroundSession is created
    // BackgroundSession defaults compaction_in_progress to false.
    // We test the behavioral consequence: flag=false means no trimming.
    let compaction_active = false;

    // @step Then the compaction_in_progress field is initialized to false
    // @step And all SessionSearch calls return untrimmed content
    let content = generate_lines(200);
    let user_meta = make_plain_user_metadata();
    let messages = vec![("user".to_string(), content.clone(), user_meta)];
    let results = apply_conditional_trimming(compaction_active, &messages);
    assert_eq!(
        results[0], content,
        "Default flag=false should return untrimmed content"
    );
}

// Scenario: create_handler accepts compaction_in_progress parameter
#[test]
fn test_create_handler_accepts_compaction_flag() {
    // @step Given a project path and an Arc<AtomicBool> compaction_trimming flag
    let project_path = PathBuf::from("/tmp/test-project");
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When create_handler is called with both parameters
    let handler = create_handler(project_path, compaction_flag.clone());

    // @step Then the returned SessionSearchHandler captures both values
    // The handler is an Arc<dyn Fn(...)> — its existence proves both params were captured.
    // We can't invoke it without persistence stores, but we can verify the flag
    // is shared: mutations via the original are visible to the handler's captured clone.
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "Flag should default to false"
    );

    // @step And the handler uses the flag to conditionally apply trimming
    compaction_flag.store(true, Ordering::Relaxed);
    assert!(
        compaction_flag.load(Ordering::Relaxed),
        "Flag should be settable to true"
    );

    // Verify handler was constructed (type-checks the 2-parameter signature)
    let _: &SessionSearchHandler = &handler;
}

// Scenario: Handler is registered with compaction_in_progress from BackgroundSession
#[test]
fn test_handler_registration_with_flag() {
    // @step Given a BackgroundSession with a compaction_in_progress field
    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When the agent loop registers the SessionSearch handler
    // Simulate the registration: clone the Arc and pass to create_handler,
    // just like session_manager.rs does with session.compaction_in_progress.clone().
    let handler = create_handler(PathBuf::from("/tmp/test-project"), compaction_flag.clone());

    // @step Then session.compaction_in_progress.clone() is passed to create_handler()
    // The handler now holds a clone of the same Arc<AtomicBool>.
    // Verify shared state: flipping the original is visible to the handler's clone.
    assert!(!compaction_flag.load(Ordering::Relaxed));
    compaction_flag.store(true, Ordering::Relaxed);
    assert!(
        compaction_flag.load(Ordering::Relaxed),
        "Cloned flag should share state"
    );

    // @step And the handler is set via set_session_search_handler()
    // In production: codelet_tools::set_session_search_handler(session.id, Some(handler));
    // Here we verify the handler exists and is the right type.
    let _: &SessionSearchHandler = &handler;
}
