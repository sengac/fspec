//! Feature: spec/features/resume-session-overwrites-manifest-destroying-message-references.feature
//!
//! Data consistency tests for the save → restore cycle.
//! Traces actual message content through persistence to find corruption.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::collections::HashMap;

use codelet_core::persistence::{
    append_message_with_metadata, create_session_with_provider, get_session_message_envelopes,
    reset_stores_for_tests,
};
use codelet_common::set_data_directory;
use tokio::sync::Mutex;
use uuid::Uuid;

/// PROV-132: Serialize tests that swap the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Helper: create a temp data directory and return its path.
fn make_temp_data_dir() -> PathBuf {
    tempfile::tempdir().expect("tempdir").keep()
}

/// Helper: set the data directory and reset stores.
fn set_temp_data_dir(path: PathBuf) -> PathBuf {
    set_data_directory(path.clone()).expect("set_data_directory");
    reset_stores_for_tests();
    path
}

// ============================================================================
// Test: Trace data through save → restore for a simple text message
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_consistency_simple_text_message() {
    let _guard = DATA_DIR_GUARD.lock().await;

    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    // @step Given a session with a simple text message
    let manifest = create_session_with_provider(
        "Data Consistency Test",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    let mut session = manifest;
    let original_content = "Hello, this is a test message with multiple words.";
    append_message_with_metadata(
        &mut session,
        "user",
        original_content,
        HashMap::new(),
    )
    .expect("append message");

    // @step When I load the stored message from disk
    let stored_messages = codelet_core::persistence::get_session_messages(&session)
        .expect("get session messages");
    assert_eq!(stored_messages.len(), 1, "should have 1 stored message");

    let stored_msg = &stored_messages[0];
    println!(
        "[SAVE] StoredMessage content: '{}'",
        stored_msg.content
    );

    // @step Then the stored content should match the original
    assert_eq!(
        stored_msg.content, original_content,
        "stored content should match original"
    );

    // @step When I get the envelope for restore
    let envelopes = get_session_message_envelopes(session.id)
        .expect("get envelopes");
    assert_eq!(envelopes.len(), 1, "should have 1 envelope");

    let envelope: serde_json::Value = serde_json::from_str(&envelopes[0]).expect("parse envelope");
    println!(
        "[RESTORE] Envelope JSON: {}",
        serde_json::to_string_pretty(&envelope).unwrap()
    );

    // @step Then the envelope content should contain the original text
    let msg_content = envelope
        .get("message")
        .and_then(|m| m.get("content"))
        .expect("message.content");
    let content_array = msg_content.as_array().expect("content is array");
    assert_eq!(content_array.len(), 1, "should have 1 content block");
    let text_block = &content_array[0];
    let restored_text = text_block
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");

    assert_eq!(
        restored_text, original_content,
        "restored text should match original content"
    );
}

// ============================================================================
// Test: Trace data through save → restore for assistant message with metadata
// (simulating what persist_assistant_message_internal does)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_consistency_assistant_message_with_envelope_metadata() {
    let _guard = DATA_DIR_GUARD.lock().await;

    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    // @step Given a session with an assistant message that has envelope metadata
    // This simulates what persist_assistant_message_internal does:
    // 1. Creates a MessageEnvelope with structured content
    // 2. Serializes it to JSON
    // 3. Stores it as metadata on the StoredMessage
    // 4. Uses a FLATTENED text string as the content field
    let manifest = create_session_with_provider(
        "Assistant Data Test",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Simulate the save path from persist_assistant_message_internal
    let original_texts = [
        "First paragraph of the response.",
        "Second paragraph with more detail.",
    ];
    let flattened_content: String = original_texts.join("\n");

    // Create the envelope metadata (as persist_assistant_message_internal does)
    let envelope_json = serde_json::json!({
        "uuid": Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "assistant",
        "provider": "anthropic/claude-sonnet-4",
        "message": {
            "role": "assistant",
            "content": original_texts.iter().map(|t| {
                serde_json::json!({"type": "text", "text": t})
            }).collect::<Vec<_>>()
        },
        "requestId": null
    });
    let metadata_map: HashMap<String, serde_json::Value> =
        serde_json::from_str(&envelope_json.to_string()).expect("parse envelope as map");

    let mut session = manifest;
    append_message_with_metadata(
        &mut session,
        "assistant",
        &flattened_content,
        metadata_map,
    )
    .expect("append message");

    // @step When I inspect the stored message
    let stored_messages = codelet_core::persistence::get_session_messages(&session)
        .expect("get session messages");
    assert_eq!(stored_messages.len(), 1, "should have 1 stored message");

    let stored_msg = &stored_messages[0];
    println!(
        "[SAVE] StoredMessage.content = '{}'",
        stored_msg.content
    );
    println!(
        "[SAVE] StoredMessage.metadata keys: {:?}",
        stored_msg.metadata.keys().collect::<Vec<_>>()
    );

    // @step Then the content field should be the flattened version
    assert_eq!(
        stored_msg.content, flattened_content,
        "content should be the flattened text"
    );

    // @step When I get the envelope for restore
    let envelopes = get_session_message_envelopes(session.id)
        .expect("get envelopes");
    assert_eq!(envelopes.len(), 1, "should have 1 envelope");

    let envelope: serde_json::Value = serde_json::from_str(&envelopes[0]).expect("parse envelope");
    println!(
        "[RESTORE] Reconstructed envelope: {}",
        serde_json::to_string_pretty(&envelope).unwrap()
    );

    // @step Then the envelope is reconstructed from StoredMessage fields
    // NOT from the metadata envelope — this is the key question!
    let msg_content = envelope
        .get("message")
        .and_then(|m| m.get("content"))
        .expect("message.content");
    let content_array = msg_content.as_array().expect("content is array");

    println!(
        "[RESTORE] Content block count: {}",
        content_array.len()
    );
    for (i, block) in content_array.iter().enumerate() {
        println!(
            "[RESTORE] Block {}: {}",
            i,
            serde_json::to_string(block).unwrap()
        );
    }

    // FIXED: The envelope now uses the original structured content from metadata,
    // NOT the flattened string. So we get the original multiple blocks back.
    assert_eq!(content_array.len(), 2, "should have 2 text blocks (original structure preserved)");
    let restored_text_0 = content_array[0]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");
    let restored_text_1 = content_array[1]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");

    // The restored text blocks match the original structured content
    assert_eq!(restored_text_0, "First paragraph of the response.", "first block should match");
    assert_eq!(restored_text_1, "Second paragraph with more detail.", "second block should match");
}

// ============================================================================
// Test: Check what get_session_message_envelopes actually produces
// for a message with multi-line content
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_consistency_multiline_content_in_envelope() {
    let _guard = DATA_DIR_GUARD.lock().await;

    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    let manifest = create_session_with_provider(
        "Multiline Test",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Save a message with embedded newlines (simulating flattened assistant content)
    let content_with_newlines = "Line one\nLine two\nLine three\n\nParagraph two\n  Indented line";
    let mut session = manifest;
    append_message_with_metadata(
        &mut session,
        "assistant",
        content_with_newlines,
        HashMap::new(),
    )
    .expect("append message");

    // @step When I get the envelope for restore
    let envelopes = get_session_message_envelopes(session.id)
        .expect("get envelopes");

    let envelope: serde_json::Value = serde_json::from_str(&envelopes[0]).expect("parse envelope");
    println!(
        "[ENVELOPE] Full envelope: {}",
        serde_json::to_string_pretty(&envelope).unwrap()
    );

    // @step Then the envelope content should preserve the newlines
    let msg_content = envelope
        .get("message")
        .and_then(|m| m.get("content"))
        .expect("message.content");
    let content_array = msg_content.as_array().expect("content is array");
    let restored_text = content_array[0]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");

    println!("[CHECK] Original: '{content_with_newlines}'");
    println!("[CHECK] Restored: '{restored_text}'");

    assert_eq!(
        restored_text, content_with_newlines,
        "newlines should be preserved in the envelope"
    );
}

// ============================================================================
// Test: Inspect the raw JSONL file to see what's actually stored
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_consistency_raw_jsonl_inspection() {
    let _guard = DATA_DIR_GUARD.lock().await;

    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    let manifest = create_session_with_provider(
        "Raw Inspection Test",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Save a message with complex content
    let content = "Hello world\nThis is a second line\n\nAnd a third paragraph";
    let mut session = manifest;
    append_message_with_metadata(
        &mut session,
        "assistant",
        content,
        HashMap::new(),
    )
    .expect("append message");

    // @step When I read the raw JSONL file
    let jsonl_path = data_dir.join("messages/messages.jsonl");
    let raw_content = std::fs::read_to_string(&jsonl_path).expect("read jsonl");
    println!("[RAW JSONL] File content:\n{raw_content}");

    // Parse the JSONL line
    let jsonl_line = raw_content.lines().next().expect("at least one line");
    let jsonl_entry: serde_json::Value = serde_json::from_str(jsonl_line).expect("parse jsonl");

    println!(
        "[RAW JSONL] Parsed entry: {}",
        serde_json::to_string_pretty(&jsonl_entry).unwrap()
    );

    // @step Then the content field in the JSONL should match what we saved
    let stored_content = jsonl_entry
        .get("content")
        .and_then(|c| c.as_str())
        .expect("content field");

    assert_eq!(
        stored_content, content,
        "JSONL content should match saved content"
    );
}

// ============================================================================
// Test: Full round-trip — save with metadata envelope, restore via envelopes
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn data_consistency_full_round_trip_with_metadata() {
    let _guard = DATA_DIR_GUARD.lock().await;

    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    let manifest = create_session_with_provider(
        "Round Trip Test",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Simulate the EXACT save path from persist_assistant_message_internal
    let assistant_content_blocks = [
        "This is the first text block from the assistant.",
        "This is the second text block.",
        "And a third block with [Thinking: truncated...] in it.",
    ];

    // This is what persist_assistant_message_internal does:
    // Flatten to: "block1\nblock2\n[Thinking: truncated...]"
    let flattened: String = assistant_content_blocks.join("\n");

    // Create the envelope metadata
    let envelope_json = serde_json::json!({
        "uuid": Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "assistant",
        "provider": "anthropic/claude-sonnet-4",
        "message": {
            "role": "assistant",
            "content": assistant_content_blocks.iter().map(|t| {
                serde_json::json!({"type": "text", "text": t})
            }).collect::<Vec<_>>()
        },
        "requestId": null
    });
    let metadata_map: HashMap<String, serde_json::Value> =
        serde_json::from_str(&envelope_json.to_string()).expect("parse envelope as map");

    let mut session = manifest;
    append_message_with_metadata(
        &mut session,
        "assistant",
        &flattened,
        metadata_map,
    )
    .expect("append message");

    // @step When I restore via get_session_message_envelopes
    let envelopes = get_session_message_envelopes(session.id)
        .expect("get envelopes");

    let envelope: serde_json::Value = serde_json::from_str(&envelopes[0]).expect("parse envelope");

    // @step Then inspect what the restore path produces
    let msg_content = envelope
        .get("message")
        .and_then(|m| m.get("content"))
        .expect("message.content");
    let content_array = msg_content.as_array().expect("content is array");

    println!(
        "[ROUND TRIP] Original had {} blocks, restored has {} blocks",
        assistant_content_blocks.len(),
        content_array.len()
    );

    for (i, block) in content_array.iter().enumerate() {
        println!(
            "[ROUND TRIP] Restored block {}: {}",
            i,
            serde_json::to_string(block).unwrap()
        );
    }

    // FIXED: The restored envelope now uses the original structured content from metadata,
    // preserving the original block structure instead of flattening.
    assert_eq!(content_array.len(), 3, "restored has 3 blocks (original structure preserved)");
    let restored_text_0 = content_array[0]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");
    let restored_text_1 = content_array[1]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");
    let restored_text_2 = content_array[2]
        .get("text")
        .and_then(|t| t.as_str())
        .expect("text field");

    // The content blocks are preserved with original structure
    assert_eq!(restored_text_0, "This is the first text block from the assistant.", "first block matches");
    assert_eq!(restored_text_1, "This is the second text block.", "second block matches");
    assert_eq!(restored_text_2, "And a third block with [Thinking: truncated...] in it.", "third block matches");
}
