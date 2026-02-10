//! Test Support Module
//!
//! Shared fixtures for persistence tests. Compiled only for tests.

use crate::persistence::{
    add_anchor_point, append_message_with_metadata, create_session, set_compaction_state,
    PersistedAnchorPoint, PersistedAnchorToolCall, SessionManifest,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

pub fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    crate::persistence::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    (guard, temp_dir)
}

fn json_object_to_hashmap(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    value
        .as_object()
        .expect("Expected JSON object")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

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

pub fn create_assistant_multi_tool_envelope(
    text: &str,
    tools: Vec<(&str, &str, serde_json::Value)>,
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

pub fn create_session_with_turns(name: &str, num_turns: usize) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");

    for i in 0..num_turns {
        let user_meta = create_user_envelope(&format!("User message {}", i));
        append_message_with_metadata(
            &mut session,
            "user",
            &format!("User message {}", i),
            user_meta,
        )
        .expect("append user message");

        let assistant_meta = create_assistant_text_envelope(&format!("Assistant response {}", i));
        append_message_with_metadata(
            &mut session,
            "assistant",
            &format!("Assistant response {}", i),
            assistant_meta,
        )
        .expect("append assistant message");
    }

    session
}

pub fn create_compacted_session(
    name: &str,
    turns_before_compaction: usize,
    turns_kept: usize,
    summary: &str,
) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");

    for i in 0..turns_kept {
        let user_meta = create_user_envelope(&format!("Kept user message {}", i));
        append_message_with_metadata(
            &mut session,
            "user",
            &format!("Kept user message {}", i),
            user_meta,
        )
        .expect("append user message");

        let assistant_meta =
            create_assistant_text_envelope(&format!("Kept assistant response {}", i));
        append_message_with_metadata(
            &mut session,
            "assistant",
            &format!("Kept assistant response {}", i),
            assistant_meta,
        )
        .expect("append assistant message");
    }

    let compacted_before_index = (turns_before_compaction - turns_kept) * 2;
    set_compaction_state(&mut session, summary.to_string(), compacted_before_index)
        .expect("set compaction state");

    session
}

pub fn create_session_with_tool_calls(name: &str) -> SessionManifest {
    let project = PathBuf::from("/test/project");
    let mut session = create_session(name, &project).expect("create session");

    let user1 = create_user_envelope("Read the config file");
    append_message_with_metadata(&mut session, "user", "Read the config file", user1)
        .expect("append");

    let tool_id = "toolu_read_01";
    let assistant1 = create_assistant_tool_use_envelope(
        "I'll read that file for you.",
        tool_id,
        "Read",
        serde_json::json!({"file_path": "/config.json"}),
    );
    append_message_with_metadata(&mut session, "assistant", "I'll read the file", assistant1)
        .expect("append");

    let tool_result1 = create_tool_result_envelope(tool_id, r#"{"debug": true}"#, false);
    append_message_with_metadata(&mut session, "user", "tool result", tool_result1)
        .expect("append");

    let user2 = create_user_envelope("Change debug to false");
    append_message_with_metadata(&mut session, "user", "Change debug to false", user2)
        .expect("append");

    let edit_tool_id = "toolu_edit_01";
    let assistant2 = create_assistant_tool_use_envelope(
        "I'll edit the config.",
        edit_tool_id,
        "Edit",
        serde_json::json!({
            "file_path": "/config.json",
            "old_string": "\"debug\": true",
            "new_string": "\"debug\": false"
        }),
    );
    append_message_with_metadata(&mut session, "assistant", "I'll edit", assistant2)
        .expect("append");

    let tool_result2 = create_tool_result_envelope(edit_tool_id, "File edited successfully", false);
    append_message_with_metadata(&mut session, "user", "edit result", tool_result2)
        .expect("append");

    session
}

pub fn add_alternating_messages(session: &mut SessionManifest, count: usize) -> usize {
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

pub fn add_simple_alternating_messages(session: &mut SessionManifest, count: usize) -> usize {
    use crate::persistence::append_message;

    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(session, role, &format!("Message {i}")).expect("append should succeed");
    }
    count
}

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
        tool_calls: vec![PersistedAnchorToolCall {
            tool: "Edit".to_string(),
            success: true,
        }],
    }
}

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
        tool_calls: vec![PersistedAnchorToolCall {
            tool: "Write".to_string(),
            success: true,
        }],
    }
}

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

pub fn create_anchor_with_timestamp(
    turn_index: usize,
    anchor_type: &str,
    timestamp_ms: i64,
) -> PersistedAnchorPoint {
    PersistedAnchorPoint {
        turn_index,
        anchor_type: anchor_type.to_string(),
        weight: 0.8,
        confidence: 0.9,
        description: format!("{} at turn {}", anchor_type, turn_index),
        timestamp_ms,
        user_message: Some(format!(
            "User message for {} at turn {}",
            anchor_type, turn_index
        )),
        assistant_response: Some(format!("Response for {} at turn {}", anchor_type, turn_index)),
        tool_calls: vec![],
    }
}

pub fn create_session_with_anchors(
    name: &str,
    num_turns: usize,
    anchor_indices: &[usize],
) -> SessionManifest {
    let mut session = create_session_with_turns(name, num_turns);

    for &idx in anchor_indices {
        let anchor = create_task_completion_anchor(idx);
        add_anchor_point(&mut session, anchor).expect("add anchor");
    }

    session
}

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
