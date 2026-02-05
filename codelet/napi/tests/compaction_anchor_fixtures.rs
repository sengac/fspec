//! Feature: Compaction and Anchor Point System
//!
//! Shared fixtures for testing the compaction, anchor detection, and persistence
//! subsystems. These fixtures provide reusable components for testing at all levels:
//! - Unit tests for individual components
//! - Integration tests for component interactions
//! - End-to-end tests for full workflows
//!
//! IMPORTANT: Tests using these fixtures share global state (persistence stores)
//! and must be run sequentially: cargo test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use codelet_napi::persistence::{
    set_data_directory, create_session, load_session, 
    append_message_with_metadata, set_compaction_state,
    add_anchor_point, get_anchor_points, PersistedAnchorPoint, PersistedAnchorToolCall,
    SessionManifest,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

// Global mutex for sequential test execution (shared global state)
lazy_static::lazy_static! {
    pub static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

// =============================================================================
// ENVIRONMENT SETUP
// =============================================================================

/// Setup an isolated temp directory for a test.
/// Uses unwrap_or_else to handle poisoned mutex gracefully.
///
/// Returns a guard (for sequential execution) and a TempDir that will be
/// cleaned up when dropped.
///
/// MUST be called at the start of every persistence test to ensure:
/// 1. Tests don't pollute ~/.fspec with test data
/// 2. Tests don't interfere with each other
pub fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory");
    (guard, temp_dir)
}

// =============================================================================
// ANCHOR POINT FIXTURES
// =============================================================================

/// Create an error resolution anchor point
pub fn create_error_resolution_anchor(turn_index: usize) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: "ErrorResolution".to_string(),
        weight: 0.9,
        confidence: 0.95,
        description: format!("Build error fixed at turn {}", turn_index),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        user_message: Some(format!("Fix the build error at turn {}", turn_index)),
        assistant_response: Some(format!("I fixed the build error at turn {}", turn_index)),
        tool_calls: vec![PersistedAnchorToolCall { tool: "Edit".to_string(), success: true }],
    }
}

/// Create a task completion anchor point
pub fn create_task_completion_anchor(turn_index: usize) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8,
        confidence: 0.92,
        description: format!("Task completed at turn {}", turn_index),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        user_message: Some(format!("Complete the task at turn {}", turn_index)),
        assistant_response: Some(format!("I completed the task at turn {}", turn_index)),
        tool_calls: vec![PersistedAnchorToolCall { tool: "Write".to_string(), success: true }],
    }
}

/// Create a user checkpoint anchor point
pub fn create_user_checkpoint_anchor(turn_index: usize) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: "UserCheckpoint".to_string(),
        weight: 0.7,
        confidence: 0.88,
        description: format!("User checkpoint at turn {}", turn_index),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        user_message: Some(format!("Create checkpoint at turn {}", turn_index)),
        assistant_response: Some(format!("Checkpoint created at turn {}", turn_index)),
        tool_calls: vec![],
    }
}

/// Create a feature milestone anchor point
pub fn create_feature_milestone_anchor(turn_index: usize) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: "FeatureMilestone".to_string(),
        weight: 0.75,
        confidence: 0.9,
        description: format!("Feature milestone at turn {}", turn_index),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        user_message: Some(format!("Reach milestone at turn {}", turn_index)),
        assistant_response: Some(format!("Milestone reached at turn {}", turn_index)),
        tool_calls: vec![],
    }
}

/// Create a synthetic anchor point (created when LLM detection fails)
pub fn create_synthetic_anchor(turn_index: usize, reason: &str) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: "UserCheckpoint".to_string(),
        weight: 1.0,
        confidence: 1.0,
        description: format!("Synthetic anchor - {}", reason),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        user_message: None,
        assistant_response: None,
        tool_calls: vec![],
    }
}

/// Create an anchor with a specific timestamp
pub fn create_anchor_with_timestamp(
    turn_index: usize, 
    anchor_type: &str, 
    timestamp_ms: i64
) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: anchor_type.to_string(),
        weight: 0.8,
        confidence: 0.9,
        description: format!("{} at turn {}", anchor_type, turn_index),
        timestamp_ms,
        user_message: Some(format!("User message for {} at turn {}", anchor_type, turn_index)),
        assistant_response: Some(format!("Response for {} at turn {}", anchor_type, turn_index)),
        tool_calls: vec![],
    }
}

// =============================================================================
// MESSAGE ENVELOPE FIXTURES
// =============================================================================

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

// =============================================================================
// SESSION FIXTURES
// =============================================================================

/// Create a session with a specified number of conversation turns
/// Each turn consists of user message + assistant response
pub fn create_session_with_turns(name: &str, num_turns: usize) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");
    
    for i in 0..num_turns {
        let user_meta = create_user_envelope(&format!("User message {}", i));
        append_message_with_metadata(
            &mut session, 
            "user", 
            &format!("User message {}", i), 
            user_meta
        ).expect("append user message");
        
        let assistant_meta = create_assistant_text_envelope(&format!("Assistant response {}", i));
        append_message_with_metadata(
            &mut session,
            "assistant",
            &format!("Assistant response {}", i),
            assistant_meta
        ).expect("append assistant message");
    }
    
    session
}

/// Create a session with turns and anchors at specified indices
pub fn create_session_with_anchors(
    name: &str, 
    num_turns: usize, 
    anchor_indices: &[usize]
) -> SessionManifest {
    let mut session = create_session_with_turns(name, num_turns);
    
    for &idx in anchor_indices {
        let anchor = create_task_completion_anchor(idx);
        add_anchor_point(&mut session, anchor).expect("add anchor");
    }
    
    session
}

/// Create a session that simulates having gone through compaction
pub fn create_compacted_session(
    name: &str,
    turns_before_compaction: usize,
    turns_kept: usize,
    summary: &str,
) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");
    
    // Add the kept turns
    for i in 0..turns_kept {
        let user_meta = create_user_envelope(&format!("Kept user message {}", i));
        append_message_with_metadata(
            &mut session,
            "user",
            &format!("Kept user message {}", i),
            user_meta
        ).expect("append user message");
        
        let assistant_meta = create_assistant_text_envelope(&format!("Kept assistant response {}", i));
        append_message_with_metadata(
            &mut session,
            "assistant", 
            &format!("Kept assistant response {}", i),
            assistant_meta
        ).expect("append assistant message");
    }
    
    // Set compaction state
    let compacted_before_index = (turns_before_compaction - turns_kept) * 2;
    set_compaction_state(&mut session, summary.to_string(), compacted_before_index)
        .expect("set compaction state");
    
    session
}

/// Create a session with tool calls and results (for anchor detection testing)
pub fn create_session_with_tool_calls(name: &str) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");
    
    // Turn 1: User asks to read a file
    let user1 = create_user_envelope("Read the config file");
    append_message_with_metadata(&mut session, "user", "Read the config file", user1)
        .expect("append");
    
    // Turn 1: Assistant uses Read tool
    let tool_id = "toolu_read_01";
    let assistant1 = create_assistant_tool_use_envelope(
        "I'll read that file for you.",
        tool_id,
        "Read",
        serde_json::json!({"file_path": "/config.json"})
    );
    append_message_with_metadata(&mut session, "assistant", "I'll read the file", assistant1)
        .expect("append");
    
    // Turn 1: Tool result
    let tool_result1 = create_tool_result_envelope(tool_id, r#"{"debug": true}"#, false);
    append_message_with_metadata(&mut session, "user", "tool result", tool_result1)
        .expect("append");
    
    // Turn 2: User asks to edit
    let user2 = create_user_envelope("Change debug to false");
    append_message_with_metadata(&mut session, "user", "Change debug to false", user2)
        .expect("append");
    
    // Turn 2: Assistant uses Edit tool
    let edit_tool_id = "toolu_edit_01";
    let assistant2 = create_assistant_tool_use_envelope(
        "I'll edit the config.",
        edit_tool_id,
        "Edit",
        serde_json::json!({
            "file_path": "/config.json",
            "old_string": "\"debug\": true",
            "new_string": "\"debug\": false"
        })
    );
    append_message_with_metadata(&mut session, "assistant", "I'll edit", assistant2)
        .expect("append");
    
    // Turn 2: Edit result (success)
    let tool_result2 = create_tool_result_envelope(edit_tool_id, "File edited successfully", false);
    append_message_with_metadata(&mut session, "user", "edit result", tool_result2)
        .expect("append");
    
    session
}

// =============================================================================
// ASSERTION HELPERS
// =============================================================================

/// Assert that a session has exactly N anchor points
pub fn assert_anchor_count(session: &SessionManifest, expected_count: usize) {
    let anchors = get_anchor_points(session);
    assert_eq!(
        anchors.len(),
        expected_count,
        "Expected {} anchors, got {}",
        expected_count,
        anchors.len()
    );
}

/// Assert that an anchor exists at a specific turn index
pub fn assert_anchor_at_turn(session: &SessionManifest, turn_index: usize) {
    let anchors = get_anchor_points(session);
    let found = anchors.iter().any(|a| a.turn_index == turn_index);
    assert!(
        found,
        "Expected anchor at turn {}, but no anchor found at that index. Anchors: {:?}",
        turn_index,
        anchors.iter().map(|a| a.turn_index).collect::<Vec<_>>()
    );
}

/// Assert that an anchor has specific properties
pub fn assert_anchor_properties(
    anchor: &PersistedAnchorPoint,
    expected_type: &str,
    expected_turn: usize,
) {
    assert_eq!(
        anchor.anchor_type, expected_type,
        "Anchor type mismatch: expected {}, got {}",
        expected_type, anchor.anchor_type
    );
    assert_eq!(
        anchor.turn_index, expected_turn,
        "Anchor turn mismatch: expected {}, got {}",
        expected_turn, anchor.turn_index
    );
}

/// Assert that anchors survived a save/load cycle
pub fn assert_anchors_persist(session_id: uuid::Uuid, expected_count: usize) {
    let reloaded = load_session(session_id).expect("load session");
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(
        anchors.len(),
        expected_count,
        "Anchors did not survive reload: expected {}, got {}",
        expected_count,
        anchors.len()
    );
}

/// Assert that compaction state exists and matches expectations
pub fn assert_compaction_state(
    session: &SessionManifest,
    expected_summary_contains: &str,
    expected_boundary: usize,
) {
    let compaction = session.compaction.as_ref()
        .expect("Session should have compaction state");
    
    assert!(
        compaction.summary.contains(expected_summary_contains),
        "Compaction summary should contain '{}', got: {}",
        expected_summary_contains,
        compaction.summary
    );
    
    assert_eq!(
        compaction.compacted_before_index, expected_boundary,
        "Compaction boundary mismatch: expected {}, got {}",
        expected_boundary,
        compaction.compacted_before_index
    );
}

// =============================================================================
// TIMESTAMP UTILITIES
// =============================================================================

/// Get a timestamp for N minutes ago
pub fn timestamp_minutes_ago(minutes: i64) -> i64 {
    chrono::Utc::now().timestamp_millis() - (minutes * 60 * 1000)
}

/// Get a timestamp for N hours ago  
pub fn timestamp_hours_ago(hours: i64) -> i64 {
    chrono::Utc::now().timestamp_millis() - (hours * 60 * 60 * 1000)
}

/// Create a sequence of anchors with timestamps spaced apart
pub fn create_anchor_sequence(
    start_turn: usize,
    count: usize,
    minutes_apart: i64,
) -> Vec<PersistedAnchorPoint> {
    (0..count)
        .map(|i| {
            let turn_index = start_turn + (i * 10); // Every 10 turns
            let timestamp = timestamp_minutes_ago(minutes_apart * (count - i) as i64);
            create_anchor_with_timestamp(turn_index, "TaskCompletion", timestamp)
        })
        .collect()
}
