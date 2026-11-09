//! Message-envelope fixtures for `session_persistence_test`: builders for
//! the user / assistant / tool_result envelopes the agent loop persists via
//! `codelet_core::persistence`, plus content-block assertions.

use codelet_core::persistence::{append_message, append_message_with_metadata, SessionManifest};
use std::collections::HashMap;

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
    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(session, role, &format!("Message {i}")).expect("append should succeed");
    }
    count
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
        "Content block {block_index} should be type '{expected_type}', got '{actual_type}'"
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
