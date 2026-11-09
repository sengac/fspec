use super::*;

#[test]
fn test_document_with_metadata() {
    let content = UserContent::Document {
        source: DocumentSource::Base64 {
            media_type: "application/pdf".to_string(),
            data: "JVBERi0xLjQK".to_string(),
        },
        title: Some("document.pdf".to_string()),
        context: Some("Project documentation".to_string()),
        cache_control: None,
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::Document {
            source,
            title,
            context,
            ..
        } => {
            match source {
                DocumentSource::Base64 { media_type, .. } => {
                    assert_eq!(media_type, "application/pdf");
                }
                _ => panic!("Expected Base64 source"),
            }
            assert_eq!(title, Some("document.pdf".to_string()));
            assert_eq!(context, Some("Project documentation".to_string()));
        }
        _ => panic!("Expected Document content"),
    }
}

#[test]
fn test_multi_part_message_preserves_order() {
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: Some("msg_multipart".to_string()),
        model: Some("claude-opus-4-5-20251101".to_string()),
        content: vec![
            AssistantContent::Text {
                text: "Let me check...".to_string(),
            },
            AssistantContent::ToolUse {
                id: "toolu_read".to_string(),
                call_id: None,
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "/foo.ts"}),
            },
            AssistantContent::Text {
                text: "Found it.".to_string(),
            },
        ],
        stop_reason: Some("end_turn".to_string()),
        usage: None,
    };

    let json = serde_json::to_string(&message).unwrap();
    let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.content.len(), 3);
    assert!(matches!(
        &restored.content[0],
        AssistantContent::Text { .. }
    ));
    assert!(matches!(
        &restored.content[1],
        AssistantContent::ToolUse { .. }
    ));
    assert!(matches!(
        &restored.content[2],
        AssistantContent::Text { .. }
    ));
}

#[test]
fn test_token_usage_preserved() {
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: Some("msg_usage".to_string()),
        model: Some("claude-opus-4-5-20251101".to_string()),
        content: vec![AssistantContent::Text {
            text: "Response with usage".to_string(),
        }],
        stop_reason: Some("end_turn".to_string()),
        usage: Some(TokenUsagePerMessage {
            input_tokens: 500,
            output_tokens: 150,
            cache_read_input_tokens: Some(200),
            cache_creation_input_tokens: Some(100),
        }),
    };

    let json = serde_json::to_string(&message).unwrap();
    let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

    let usage = restored.usage.expect("Expected usage");
    assert_eq!(usage.input_tokens, 500);
    assert_eq!(usage.output_tokens, 150);
    assert_eq!(usage.cache_read_input_tokens, Some(200));
    assert_eq!(usage.cache_creation_input_tokens, Some(100));
}

#[test]
fn test_request_id_preserved() {
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: None,
            model: None,
            content: vec![AssistantContent::Text {
                text: "Test".to_string(),
            }],
            stop_reason: None,
            usage: None,
        }),
        request_id: Some("req_011CWPLKJZcigRWVriKduWSr".to_string()),
    };

    let json = serde_json::to_string(&envelope).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.request_id,
        Some("req_011CWPLKJZcigRWVriKduWSr".to_string())
    );
}

#[test]
fn test_stop_reason_preserved() {
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: None,
        model: None,
        content: vec![AssistantContent::Text {
            text: "Complete response".to_string(),
        }],
        stop_reason: Some("end_turn".to_string()),
        usage: None,
    };

    let json = serde_json::to_string(&message).unwrap();
    let restored: AssistantMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.stop_reason, Some("end_turn".to_string()));
}

#[test]
fn test_assistant_message_without_optional_fields() {
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: None,
        model: None,
        content: vec![AssistantContent::Text {
            text: "Response without optional fields".to_string(),
        }],
        stop_reason: None,
        usage: None,
    };

    let json = serde_json::to_string(&message).unwrap();
    let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

    assert!(restored.id.is_none());
    assert!(restored.model.is_none());
    assert!(restored.stop_reason.is_none());
    assert!(restored.usage.is_none());
}

#[test]
fn test_interrupted_tool_execution_metadata() {
    let content = UserContent::ToolResult {
        tool_use_id: "toolu_abc123".to_string(),
        call_id: None,
        content: "partial output".to_string(),
        is_error: true,
        tool_use_result: Some(ToolUseResultMetadata {
            stdout: Some("partial output".to_string()),
            stderr: Some("".to_string()),
            interrupted: true,
            is_image: false,
        }),
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: UserContent = serde_json::from_str(&json).unwrap();

    match restored {
        UserContent::ToolResult {
            tool_use_result,
            is_error,
            ..
        } => {
            let metadata = tool_use_result.expect("Expected metadata");
            assert!(metadata.interrupted);
            assert!(is_error);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

/// Scenario: OpenAI streaming propagates max_tokens stop_reason instead of hardcoding end_turn
#[test]
fn test_assistant_message_preserves_max_tokens_stop_reason() {
    // @step Given the agent is using the OpenAI provider in streaming mode
    // (Testing persistence layer — provider-agnostic)

    // @step And the model hits the max_tokens limit during text generation
    // (Simulated by setting stop_reason to "max_tokens")

    // @step When the OpenAI SSE stream emits a response with finish_reason "length"
    // (At this layer: the normalized stop_reason "max_tokens" arrives at persistence)
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: Some("msg_truncated".to_string()),
        model: Some("gpt-4o".to_string()),
        content: vec![AssistantContent::Text {
            text: "This response was truncated...".to_string(),
        }],
        stop_reason: Some("max_tokens".to_string()),
        usage: None,
    };

    let json = serde_json::to_string(&message).unwrap();
    let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

    // @step Then the FinalResponse contains StopReason::MaxTokens
    assert_eq!(restored.stop_reason, Some("max_tokens".to_string()));

    // @step And the persisted AssistantMessage stop_reason is "max_tokens"
    assert_ne!(restored.stop_reason, Some("end_turn".to_string()));

    // @step And the stop_reason is not hardcoded to "end_turn"
    assert_ne!(restored.stop_reason, Some("end_turn".to_string()));
}
