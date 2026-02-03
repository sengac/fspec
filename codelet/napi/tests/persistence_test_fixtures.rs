// Feature: spec/features/migrate-session-message-persistence-from-typescript-to-rust.feature
//
// Shared Test Fixtures for Persistence Tests
//
// This module provides DRY fixtures for persistence tests:
// - setup_test_env() - isolated temp directory with sequential execution
// - Envelope builders for user, assistant, tool_use, tool_result messages
// - Common imports and test patterns
//
// IMPORTANT: Tests using these fixtures share global state (persistence stores)
// and must be run sequentially: cargo test -- --test-threads=1

// Suppress dead_code warnings for shared fixtures.
// These functions ARE used by session_persistence_test.rs but Rust's dead code
// analysis doesn't see cross-module usage in test binaries.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use codelet_napi::persistence::set_data_directory;
use std::collections::HashMap;
use std::sync::Mutex;

// Global mutex for sequential test execution (shared global state)
lazy_static::lazy_static! {
    pub static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup an isolated temp directory for a test.
/// Uses unwrap_or_else to handle poisoned mutex gracefully.
///
/// Returns a guard (for sequential execution) and a TempDir that will be
/// cleaned up when dropped.
///
/// MUST be called at the start of every persistence test to ensure:
/// 1. Tests don't pollute ~/.fspec with test data
/// 2. Tests don't interfere with each other
pub fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    // Handle potentially poisoned mutex from a previous test panic
    let guard = TEST_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory");
    (guard, temp_dir)
}

/// Create a user message envelope with proper structure
pub fn create_user_envelope(text: &str) -> HashMap<String, serde_json::Value> {
    let envelope = serde_json::json!({
        "uuid": uuid::Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "user",
        "provider": "test",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": text}]
        },
        "requestId": null
    });
    json_object_to_hashmap(&envelope)
}

/// Create an assistant message envelope with text content only
pub fn create_assistant_text_envelope(text: &str) -> HashMap<String, serde_json::Value> {
    let envelope = serde_json::json!({
        "uuid": uuid::Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "assistant",
        "provider": "claude",
        "message": {
            "role": "assistant",
            "content": [{"type": "text", "text": text}]
        },
        "requestId": format!("req_{}", uuid::Uuid::new_v4())
    });
    json_object_to_hashmap(&envelope)
}

/// Create an assistant message envelope with text and a single tool_use
pub fn create_assistant_tool_use_envelope(
    text: &str,
    tool_id: &str,
    tool_name: &str,
    input: serde_json::Value,
) -> HashMap<String, serde_json::Value> {
    let envelope = serde_json::json!({
        "uuid": uuid::Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "assistant",
        "provider": "claude",
        "message": {
            "role": "assistant",
            "content": [
                {"type": "text", "text": text},
                {"type": "tool_use", "id": tool_id, "name": tool_name, "input": input}
            ]
        },
        "requestId": format!("req_{}", uuid::Uuid::new_v4())
    });
    json_object_to_hashmap(&envelope)
}

/// Create an assistant message envelope with text and multiple tool_use blocks
pub fn create_assistant_multi_tool_envelope(
    text: &str,
    tools: Vec<(&str, &str, serde_json::Value)>, // (id, name, input)
) -> HashMap<String, serde_json::Value> {
    let mut content = vec![serde_json::json!({"type": "text", "text": text})];
    for (id, name, input) in tools {
        content.push(serde_json::json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": input
        }));
    }
    
    let envelope = serde_json::json!({
        "uuid": uuid::Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "assistant",
        "provider": "claude",
        "message": {
            "role": "assistant",
            "content": content
        },
        "requestId": format!("req_{}", uuid::Uuid::new_v4())
    });
    json_object_to_hashmap(&envelope)
}

/// Create a tool_result message envelope
pub fn create_tool_result_envelope(
    tool_use_id: &str,
    content: &str,
    is_error: bool,
) -> HashMap<String, serde_json::Value> {
    let envelope = serde_json::json!({
        "uuid": uuid::Uuid::new_v4().to_string(),
        "parentUuid": null,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": "user",
        "provider": "tool",
        "message": {
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
                "is_error": is_error
            }]
        },
        "requestId": null
    });
    json_object_to_hashmap(&envelope)
}

/// Helper: Convert a JSON object to HashMap<String, Value>
fn json_object_to_hashmap(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    value
        .as_object()
        .expect("Expected JSON object")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Assert that a message has a specific content block type at a given index
pub fn assert_content_block_type(
    metadata: &HashMap<String, serde_json::Value>,
    block_index: usize,
    expected_type: &str,
) {
    let content = metadata
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .expect("Should have content array");
    
    let actual_type = content
        .get(block_index)
        .and_then(|b| b.get("type"))
        .and_then(|t| t.as_str())
        .expect("Should have type field");
    
    assert_eq!(
        actual_type, expected_type,
        "Content block {} should be type '{}', got '{}'",
        block_index, expected_type, actual_type
    );
}

/// Assert that a message envelope has the expected number of content blocks
pub fn assert_content_block_count(
    metadata: &HashMap<String, serde_json::Value>,
    expected_count: usize,
) {
    let content = metadata
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .expect("Should have content array");
    
    assert_eq!(
        content.len(),
        expected_count,
        "Expected {} content blocks, got {}",
        expected_count,
        content.len()
    );
}

/// Count how many content blocks of a specific type exist in a message
pub fn count_content_blocks_of_type(
    metadata: &HashMap<String, serde_json::Value>,
    block_type: &str,
) -> usize {
    metadata
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some(block_type))
                .count()
        })
        .unwrap_or(0)
}

/// Add N alternating user/assistant messages to a session with metadata
/// 
/// This is a DRY helper for tests that need to populate a session with messages.
/// Returns the number of messages added.
pub fn add_alternating_messages(
    session: &mut codelet_napi::persistence::SessionManifest,
    count: usize,
) -> usize {
    use codelet_napi::persistence::append_message_with_metadata;
    
    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let meta = if role == "user" {
            create_user_envelope(&format!("Message {i}"))
        } else {
            create_assistant_text_envelope(&format!("Response {i}"))
        };
        append_message_with_metadata(session, role, &format!("Message {i}"), meta)
            .expect("append should succeed");
    }
    count
}

/// Add N alternating user/assistant messages without metadata (simpler version)
pub fn add_simple_alternating_messages(
    session: &mut codelet_napi::persistence::SessionManifest,
    count: usize,
) -> usize {
    use codelet_napi::persistence::append_message;
    
    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(session, role, &format!("Message {i}"))
            .expect("append should succeed");
    }
    count
}
