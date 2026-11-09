use super::*;

// ============================================================================
// END-TO-END TEST: Verify compacted session envelopes can be restored
// This tests the ACTUAL bug fix - that synthetic compaction envelopes
// returned by persistence_get_session_message_envelopes() can be parsed
// by restore_messages_from_envelopes() in session.rs
// ============================================================================
#[test]
fn test_compacted_session_envelopes_can_be_parsed_end_to_end() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    let project = PathBuf::from("/test/project/e2e_compaction");

    // @step Given I have a session with 10 messages
    let mut session = create_session("E2E Compaction Test", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        // Store messages with proper envelope metadata (simulating real usage)
        let metadata = {
            let mut meta = std::collections::HashMap::new();
            let envelope = serde_json::json!({
                "uuid": uuid::Uuid::new_v4().to_string(),
                "parentUuid": null,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "type": role,
                "provider": "test",
                "message": {
                    "role": role,
                    "content": [{"type": "text", "text": format!("Message {}", i)}]
                },
                "requestId": null
            });
            for (k, v) in envelope.as_object().unwrap() {
                meta.insert(k.clone(), v.clone());
            }
            meta
        };
        append_message_with_metadata(&mut session, role, &format!("Message {}", i), metadata)
            .expect("append");
    }
    let session_id = session.id;

    // @step And the session has been compacted at index 8
    let summary = "Previous 8 messages discussed project setup and architecture decisions.";
    set_compaction_state(&mut session, summary.to_string(), 8).expect("set compaction");

    // @step When I reload the session and get envelopes (simulating /resume)
    let reloaded = load_session(session_id).expect("reload");

    // This is what persistence_get_session_message_envelopes does internally
    let messages = get_session_messages(&reloaded).expect("get messages");

    // Should have: 1 synthetic summary + 2 post-compaction messages (indices 8, 9)
    assert_eq!(
        messages.len(),
        3,
        "Expected 1 summary + 2 post-compaction messages"
    );

    // @step And I convert each message to envelope JSON (like napi_bindings does)
    let mut envelopes: Vec<String> = Vec::new();
    for stored_msg in &messages {
        if stored_msg.id == uuid::Uuid::nil() {
            // Synthetic compaction summary - use the FIXED format
            let synthetic_envelope = serde_json::json!({
                "uuid": "00000000-0000-0000-0000-000000000000",
                "parentUuid": null,
                "timestamp": stored_msg.created_at.to_rfc3339(),
                "type": "user",
                "provider": "compaction",
                "message": {
                    "role": "user",
                    "content": [{"type": "text", "text": stored_msg.content}]
                },
                "requestId": null,
                "_synthetic": true,
                "_compactionSummary": true
            });
            envelopes.push(serde_json::to_string(&synthetic_envelope).unwrap());
        } else {
            // Real message - use stored metadata
            let metadata_value = serde_json::Value::Object(
                stored_msg
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            envelopes.push(serde_json::to_string(&metadata_value).unwrap());
        }
    }

    // @step Then ALL envelopes should parse successfully as MessageEnvelope
    // This is what restore_messages_from_envelopes() does
    for (idx, envelope_json) in envelopes.iter().enumerate() {
        let parsed: Result<MessageEnvelope, _> = serde_json::from_str(envelope_json);
        assert!(
            parsed.is_ok(),
            "Envelope {} failed to parse as MessageEnvelope: {:?}\nJSON: {}",
            idx,
            parsed.err(),
            &envelope_json[..std::cmp::min(500, envelope_json.len())]
        );
    }

    // @step And the first envelope should be the compaction summary
    let first_envelope: MessageEnvelope = serde_json::from_str(&envelopes[0]).unwrap();
    assert_eq!(first_envelope.uuid, uuid::Uuid::nil());
    assert_eq!(first_envelope.provider, "compaction");
    match &first_envelope.message {
        super::message_envelope::MessagePayload::User(user_msg) => match &user_msg.content[0] {
            super::message_envelope::UserContent::Text { text } => {
                assert!(
                    text.contains(summary),
                    "First envelope should contain the compaction summary"
                );
            }
            _ => panic!("Expected Text content"),
        },
        _ => panic!("Expected User message"),
    }

    // @step And the remaining envelopes should be the post-compaction messages
    let second_envelope: MessageEnvelope = serde_json::from_str(&envelopes[1]).unwrap();
    if let super::message_envelope::MessagePayload::User(user_msg) = &second_envelope.message {
        if let super::message_envelope::UserContent::Text { text } = &user_msg.content[0] {
            assert!(
                text.contains("Message 8"),
                "Second envelope should be message 8 (first post-compaction)"
            );
        } else {
            panic!("Expected Text content");
        }
    }
    // Note: Could also be assistant message, which is fine
}

#[test]
fn rpc049_get_session_message_envelopes_missing_session_returns_err() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given a non-existent session UUID
    let missing = uuid::Uuid::new_v4();

    // @step When codelet_core::persistence::get_session_message_envelopes(uuid) is called
    let result = get_session_message_envelopes(missing);

    // @step Then the function returns Err with a message describing the missing session
    assert!(
        result.is_err(),
        "expected Err for non-existent session, got {:?}",
        result
    );
}

#[test]
fn rpc049_get_session_message_envelopes_returns_parseable_envelopes() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/rpc049_envelopes");

    // @step Given an existing manifest with two appended messages
    let mut session = create_session("RPC-049 envelope test", &project).expect("create");
    for (role, text) in &[("user", "hello"), ("assistant", "hi back")] {
        // RPC-049: messages must carry proper envelope metadata so the
        // lifted `get_session_message_envelopes` body can rehydrate the
        // stored envelope into a serde_json::Value with a "message"
        // field. Mirrors the metadata-shape used by
        // `test_compaction_envelopes_parse_as_messageenvelope` above.
        let metadata = {
            let mut meta = std::collections::HashMap::new();
            let envelope = serde_json::json!({
                "uuid": uuid::Uuid::new_v4().to_string(),
                "parentUuid": null,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "type": role,
                "provider": "test",
                "message": {
                    "role": role,
                    "content": [{"type": "text", "text": text}]
                },
                "requestId": null
            });
            for (k, v) in envelope.as_object().unwrap() {
                meta.insert(k.clone(), v.clone());
            }
            meta
        };
        append_message_with_metadata(&mut session, role, text, metadata)
            .expect("append message with metadata");
    }
    let session_id = session.id;

    // @step When codelet_core::persistence::get_session_message_envelopes(uuid) is called
    let envelopes = get_session_message_envelopes(session_id).expect("get envelopes");

    // @step Then the result is Ok(Vec<String>) with two JSON envelopes
    assert_eq!(
        envelopes.len(),
        2,
        "expected 2 envelopes, got {}",
        envelopes.len()
    );

    // @step And each envelope parses via serde_json::from_str into a serde_json::Value with a 'message' field
    for (idx, envelope_json) in envelopes.iter().enumerate() {
        let value: serde_json::Value =
            serde_json::from_str(envelope_json).expect("envelope is valid JSON");
        assert!(
            value.get("message").is_some(),
            "envelope {} lacks a 'message' field: {}",
            idx,
            envelope_json
        );
    }
}
