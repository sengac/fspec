use super::*;

#[test]
fn test_thinking_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large thinking content is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large thinking content (>10KB)
    let large_thinking = "Let me think about this problem step by step... ".repeat(500);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::Thinking {
                thinking: large_thinking.clone(),
                signature: Some("sig_test123".to_string()),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for thinking"
    );

    // Verify thinking content was replaced with blob reference and signature preserved
    match &processed.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::Thinking {
                thinking,
                signature,
            } => {
                assert!(
                    thinking.starts_with("blob:sha256:"),
                    "Thinking should be blob reference"
                );
                assert_eq!(
                    signature,
                    &Some("sig_test123".to_string()),
                    "Signature should be preserved"
                );
            }
            _ => panic!("Expected Thinking"),
        },
        _ => panic!("Expected Assistant message"),
    }

    // Verify rehydration restores original thinking
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated =
        super::rehydrate_envelope_blobs(&processed_json).expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::Thinking { thinking, .. } => {
                assert_eq!(
                    *thinking, large_thinking,
                    "Rehydrated thinking should match original"
                );
            }
            _ => panic!("Expected Thinking"),
        },
        _ => panic!("Expected Assistant message"),
    }
}

#[test]
fn test_small_content_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content under 10KB threshold stays inline (not stored in blob)
    use chrono::Utc;
    use uuid::Uuid;

    let small_content = "This is small content under 10KB threshold";

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_small".to_string(),
                call_id: None,
                content: small_content.to_string(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify NO blob was created (content too small)
    assert!(
        blob_refs.is_empty(),
        "Should NOT have blob references for small content"
    );

    // Verify content remains inline
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(
                    *content, small_content,
                    "Small content should remain inline"
                );
                assert!(
                    !content.starts_with("blob:sha256:"),
                    "Should NOT be blob reference"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_url_sources_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that URL image/document sources are NOT stored in blob (only base64 is)
    use chrono::Utc;
    use uuid::Uuid;

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![
                UserContent::Image {
                    source: ImageSource::Url {
                        url: "https://example.com/image.png".to_string(),
                    },
                },
                UserContent::Document {
                    source: DocumentSource::Url {
                        url: "https://example.com/doc.pdf".to_string(),
                    },
                    title: None,
                    context: None,
                    cache_control: None,
                },
            ],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify NO blob was created (URL sources stay inline)
    assert!(blob_refs.is_empty(), "URL sources should NOT create blobs");

    // Verify URLs remain unchanged
    match &processed.message {
        MessagePayload::User(user_msg) => {
            match &user_msg.content[0] {
                UserContent::Image {
                    source: ImageSource::Url { url },
                } => {
                    assert_eq!(
                        url, "https://example.com/image.png",
                        "Image URL should remain unchanged"
                    );
                }
                _ => panic!("Expected URL Image"),
            }
            match &user_msg.content[1] {
                UserContent::Document {
                    source: DocumentSource::Url { url },
                    ..
                } => {
                    assert_eq!(
                        url, "https://example.com/doc.pdf",
                        "Document URL should remain unchanged"
                    );
                }
                _ => panic!("Expected URL Document"),
            }
        }
        _ => panic!("Expected User message"),
    }
}
