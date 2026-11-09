use super::*;

#[test]
fn test_message_envelope_serialization() {
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Text {
                text: "Hello".to_string(),
            }],
        }),
        request_id: None,
    };

    let json = serde_json::to_string(&envelope).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope, restored);
}

#[test]
fn test_tool_use_serialization() {
    let content = AssistantContent::ToolUse {
        id: "toolu_123".to_string(),
        call_id: None,
        name: "read_file".to_string(),
        input: serde_json::json!({"path": "/foo.ts"}),
    };

    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"tool_use\""));
    assert!(json.contains("\"id\":\"toolu_123\""));
}

#[test]
fn test_thinking_with_signature() {
    let content = AssistantContent::Thinking {
        thinking: "Let me think...".to_string(),
        signature: Some("sig_abc123".to_string()),
    };

    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"thinking\""));
    assert!(json.contains("\"signature\":\"sig_abc123\""));
}

#[test]
fn test_message_uuid_preserved_for_identity() {
    let uuid = Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831").unwrap();
    let envelope = MessageEnvelope {
        uuid,
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::Text {
                text: "Hello".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: None,
    };

    let json = serde_json::to_string(&envelope).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.uuid,
        Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831").unwrap()
    );
}

#[test]
fn test_parent_uuid_preserved_for_threading() {
    let msg1_uuid = Uuid::new_v4();
    let msg2 = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: Some(msg1_uuid),
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: None,
            content: vec![AssistantContent::Text {
                text: "Hi there!".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: None,
    };

    let json = serde_json::to_string(&msg2).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.parent_uuid, Some(msg1_uuid));
}

#[test]
fn test_tool_result_serialization() {
    let content = UserContent::ToolResult {
        tool_use_id: "toolu_01AyTnm7YLfybhnhEhwwZvAY".to_string(),
        call_id: None,
        content: r#"{"port": 8080}"#.to_string(),
        is_error: false,
        tool_use_result: None,
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::ToolResult {
            tool_use_id,
            content,
            is_error,
            ..
        } => {
            assert_eq!(tool_use_id, "toolu_01AyTnm7YLfybhnhEhwwZvAY");
            assert_eq!(content, r#"{"port": 8080}"#);
            assert!(!is_error);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

#[test]
fn test_tool_error_result_preserved() {
    let content = UserContent::ToolResult {
        tool_use_id: "toolu_xyz789".to_string(),
        call_id: None,
        content: "Permission denied".to_string(),
        is_error: true,
        tool_use_result: None,
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::ToolResult { is_error, .. } => {
            assert!(is_error);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

#[test]
fn test_tool_result_with_metadata() {
    let content = UserContent::ToolResult {
        tool_use_id: "toolu_abc123".to_string(),
        call_id: None,
        content: "formatted output".to_string(),
        is_error: false,
        tool_use_result: Some(ToolUseResultMetadata::with_output(
            "raw stdout content",
            "warning: deprecated",
        )),
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::ToolResult {
            tool_use_result, ..
        } => {
            let metadata = tool_use_result.expect("Expected metadata");
            assert_eq!(metadata.stdout, Some("raw stdout content".to_string()));
            assert_eq!(metadata.stderr, Some("warning: deprecated".to_string()));
            assert!(!metadata.interrupted);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

#[test]
fn test_thinking_without_signature() {
    let content = AssistantContent::Thinking {
        thinking: "Let me think about this problem...".to_string(),
        signature: None,
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: AssistantContent = serde_json::from_str(&json).unwrap();

    match restored {
        AssistantContent::Thinking {
            thinking,
            signature,
        } => {
            assert_eq!(thinking, "Let me think about this problem...");
            assert!(signature.is_none());
        }
        _ => panic!("Expected Thinking content"),
    }
}

#[test]
fn test_base64_image_content() {
    let content = UserContent::Image {
        source: ImageSource::Base64 {
            media_type: "image/png".to_string(),
            data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
        },
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::Image { source } => match source {
            ImageSource::Base64 { media_type, data } => {
                assert_eq!(media_type, "image/png");
                assert!(data.starts_with("iVBORw0KGgo"));
            }
            _ => panic!("Expected Base64 source"),
        },
        _ => panic!("Expected Image content"),
    }
}

#[test]
fn test_url_image_reference() {
    let content = UserContent::Image {
        source: ImageSource::Url {
            url: "https://example.com/screenshot.jpg".to_string(),
        },
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::Image { source } => match source {
            ImageSource::Url { url } => {
                assert_eq!(url, "https://example.com/screenshot.jpg");
            }
            _ => panic!("Expected Url source"),
        },
        _ => panic!("Expected Image content"),
    }
}
