use super::*;

#[test]
fn test_image_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large base64 image data is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large base64 image data (>10KB)
    let large_image_data = "A".repeat(20_000); // Simulates base64 encoded image

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Image {
                source: ImageSource::Base64 {
                    media_type: "image/png".to_string(),
                    data: large_image_data.clone(),
                },
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for image"
    );

    // Verify image data was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Image {
                source: ImageSource::Base64 { data, media_type },
            } => {
                assert!(
                    data.starts_with("blob:sha256:"),
                    "Image data should be blob reference"
                );
                assert_eq!(media_type, "image/png", "Media type should be preserved");
            }
            _ => panic!("Expected Base64 Image"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify rehydration restores original data
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated =
        super::rehydrate_envelope_blobs(&processed_json).expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Image {
                source: ImageSource::Base64 { data, .. },
            } => {
                assert_eq!(
                    *data, large_image_data,
                    "Rehydrated image data should match original"
                );
            }
            _ => panic!("Expected Base64 Image"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_document_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large base64 document data is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large base64 document data (>10KB)
    let large_doc_data = "B".repeat(25_000); // Simulates base64 encoded PDF

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Document {
                source: DocumentSource::Base64 {
                    media_type: "application/pdf".to_string(),
                    data: large_doc_data.clone(),
                },
                title: Some("report.pdf".to_string()),
                context: Some("Q4 Report".to_string()),
                cache_control: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::process_envelope_for_blob_storage(&envelope).expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for document"
    );

    // Verify document data was replaced with blob reference and metadata preserved
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Document {
                source: DocumentSource::Base64 { data, media_type },
                title,
                context,
                ..
            } => {
                assert!(
                    data.starts_with("blob:sha256:"),
                    "Document data should be blob reference"
                );
                assert_eq!(
                    media_type, "application/pdf",
                    "Media type should be preserved"
                );
                assert_eq!(
                    title,
                    &Some("report.pdf".to_string()),
                    "Title should be preserved"
                );
                assert_eq!(
                    context,
                    &Some("Q4 Report".to_string()),
                    "Context should be preserved"
                );
            }
            _ => panic!("Expected Base64 Document"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify rehydration restores original data
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated =
        super::rehydrate_envelope_blobs(&processed_json).expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Document {
                source: DocumentSource::Base64 { data, .. },
                ..
            } => {
                assert_eq!(
                    *data, large_doc_data,
                    "Rehydrated document data should match original"
                );
            }
            _ => panic!("Expected Base64 Document"),
        },
        _ => panic!("Expected User message"),
    }
}
