use super::*;

#[test]
fn test_blob_deduplication_across_envelopes() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that identical content in different envelopes uses same blob hash
    use chrono::Utc;
    use uuid::Uuid;

    let identical_content = "x".repeat(15_000);

    let envelope1 = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_1".to_string(),
                call_id: None,
                content: identical_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    let envelope2 = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: Some(envelope1.uuid),
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_2".to_string(),
                call_id: None,
                content: identical_content,
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process both envelopes
    let (_, blob_refs1) =
        super::process_envelope_for_blob_storage(&envelope1).expect("process should succeed");
    let (_, blob_refs2) =
        super::process_envelope_for_blob_storage(&envelope2).expect("process should succeed");

    // Verify both have blob refs with SAME hash (deduplication)
    assert!(!blob_refs1.is_empty() && !blob_refs2.is_empty());
    let hash1 = &blob_refs1[0].1;
    let hash2 = &blob_refs2[0].1;
    assert_eq!(
        hash1, hash2,
        "Identical content should produce same blob hash"
    );
}

#[test]
fn test_multi_part_message_blob_storage() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that a message with multiple content parts handles blobs correctly
    use chrono::Utc;
    use uuid::Uuid;

    let large_content1 = "A".repeat(15_000);
    let large_content2 = "B".repeat(20_000);
    let small_content = "small text";

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![
                UserContent::Text {
                    text: small_content.to_string(),
                },
                UserContent::ToolResult {
                    tool_use_id: "toolu_large1".to_string(),
                    call_id: None,
                    content: large_content1.clone(),
                    is_error: false,
                    tool_use_result: None,
                },
                UserContent::ToolResult {
                    tool_use_id: "toolu_large2".to_string(),
                    call_id: None,
                    content: large_content2.clone(),
                    is_error: false,
                    tool_use_result: None,
                },
            ],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify we have 2 blob refs (for the 2 large tool results)
    assert_eq!(blob_refs.len(), 2, "Should have 2 blob references");

    // Verify text content unchanged, large contents replaced
    match &processed.message {
        MessagePayload::User(user_msg) => {
            // Text should be unchanged
            match &user_msg.content[0] {
                UserContent::Text { text } => {
                    assert_eq!(text, small_content);
                }
                _ => panic!("Expected Text"),
            }
            // Large tool results should be blob references
            match &user_msg.content[1] {
                UserContent::ToolResult { content, .. } => {
                    assert!(content.starts_with("blob:sha256:"));
                }
                _ => panic!("Expected ToolResult"),
            }
            match &user_msg.content[2] {
                UserContent::ToolResult { content, .. } => {
                    assert!(content.starts_with("blob:sha256:"));
                }
                _ => panic!("Expected ToolResult"),
            }
        }
        _ => panic!("Expected User message"),
    }

    // Verify full rehydration works
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated =
        super::rehydrate_envelope_blobs(&processed_json).expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => {
            // Text unchanged
            match &user_msg.content[0] {
                UserContent::Text { text } => assert_eq!(text, small_content),
                _ => panic!("Expected Text"),
            }
            // Large contents restored
            match &user_msg.content[1] {
                UserContent::ToolResult { content, .. } => assert_eq!(*content, large_content1),
                _ => panic!("Expected ToolResult"),
            }
            match &user_msg.content[2] {
                UserContent::ToolResult { content, .. } => assert_eq!(*content, large_content2),
                _ => panic!("Expected ToolResult"),
            }
        }
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_exact_10kb_threshold_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content at exactly 10KB (10240 bytes) stays inline
    use chrono::Utc;
    use uuid::Uuid;

    // Exactly 10KB = 10 * 1024 = 10240 bytes
    let exactly_10kb = "x".repeat(10_240);
    assert_eq!(exactly_10kb.len(), 10_240);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_exact".to_string(),
                call_id: None,
                content: exactly_10kb.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify NO blob was created (threshold is >10KB, not >=10KB)
    assert!(
        blob_refs.is_empty(),
        "Exactly 10KB should NOT create blob (threshold is >10KB)"
    );

    // Verify content remains inline
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(*content, exactly_10kb, "10KB content should remain inline");
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}
