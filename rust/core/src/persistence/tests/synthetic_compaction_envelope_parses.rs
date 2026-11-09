use super::*;

// ============================================================================
// CRITICAL: Test that synthetic compaction envelope can be parsed by session restore
// This tests the actual flow: persistence → envelope JSON → MessageEnvelope struct
// ============================================================================
#[test]
fn test_synthetic_compaction_envelope_parses_as_message_envelope() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    let project = PathBuf::from("/test/project/envelope_parse_test");

    // @step Given I have a session with messages that has been compacted
    let mut session = create_session("Envelope Parse Test", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has compaction state
    let summary = "Previous conversation covered authentication flow and database design.";
    set_compaction_state(&mut session, summary.to_string(), 8).expect("set compaction");

    // @step When I get the message envelopes (which includes the synthetic compaction summary)
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // The first message should be synthetic (nil UUID)
    assert_eq!(
        messages[0].id,
        uuid::Uuid::nil(),
        "First message should be synthetic"
    );

    // @step And I construct the synthetic envelope JSON (same logic as napi_bindings.rs)
    let stored_msg = &messages[0];
    let synthetic_envelope_json = serde_json::json!({
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
    let envelope_str = serde_json::to_string(&synthetic_envelope_json).unwrap();

    // @step Then the envelope should successfully parse as MessageEnvelope
    let parsed: Result<MessageEnvelope, _> = serde_json::from_str(&envelope_str);
    assert!(
        parsed.is_ok(),
        "Synthetic compaction envelope must parse as MessageEnvelope. Error: {:?}",
        parsed.err()
    );

    // @step And the parsed envelope should have correct fields
    let envelope = parsed.unwrap();
    assert_eq!(envelope.uuid, uuid::Uuid::nil());
    assert_eq!(envelope.message_type, "user");
    assert_eq!(envelope.provider, "compaction");

    // @step And the message payload should contain the summary
    match &envelope.message {
        super::message_envelope::MessagePayload::User(user_msg) => {
            assert_eq!(user_msg.role, "user");
            assert!(!user_msg.content.is_empty());
            match &user_msg.content[0] {
                super::message_envelope::UserContent::Text { text } => {
                    assert!(
                        text.contains(summary),
                        "Envelope text should contain the compaction summary"
                    );
                }
                _ => panic!("Expected Text content in synthetic envelope"),
            }
        }
        _ => panic!("Expected User message payload in synthetic envelope"),
    }
}

// ============================================================================
// Test the OLD broken format to document what was wrong
// This test ensures the bug is understood and doesn't regress
// ============================================================================
#[test]
fn test_old_broken_synthetic_envelope_fails_to_parse() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    // @step Given the OLD broken synthetic envelope format (missing required fields, wrong key name)
    let broken_envelope_json = serde_json::json!({
        "message_type": "user",  // WRONG: should be "type" due to #[serde(rename = "type")]
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "test summary"}]
        },
        "_synthetic": true,
        "_compactionSummary": true
        // MISSING: uuid, timestamp, provider (all required!)
    });
    let envelope_str = serde_json::to_string(&broken_envelope_json).unwrap();

    // @step Then the broken envelope should FAIL to parse as MessageEnvelope
    let parsed: Result<MessageEnvelope, _> = serde_json::from_str(&envelope_str);
    assert!(
        parsed.is_err(),
        "Old broken envelope format should NOT parse - if this passes, the bug isn't fixed!"
    );
}
