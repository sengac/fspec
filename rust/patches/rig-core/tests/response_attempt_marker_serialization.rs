//! Feature: spec/features/response-attempt-marker-serialization.feature

use rig::{agent::MultiTurnStreamItem, streaming::StreamedAssistantContent};
use serde_json::{Value, json};

type Content = StreamedAssistantContent<Value>;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn attempt_boundaries_retain_their_identity_through_json() -> TestResult {
    // @step Given response attempt start and abort markers
    let markers = [
        Content::ResponseAttemptStarted,
        Content::ResponseAttemptAborted,
    ];
    let mut encodings = Vec::new();
    // @step When each marker is serialized and deserialized
    for marker in markers {
        let encoded = serde_json::to_value(&marker)?;
        let restored: Content = serde_json::from_value(encoded.clone())?;
        assert_eq!(restored, marker);
        encodings.push(encoded);
    }
    // @step Then their encodings differ and both identities are preserved
    assert_ne!(encodings[0], encodings[1]);
    assert_eq!(encodings[0], json!({"responseAttempt": "started"}));
    assert_eq!(encodings[1], json!({"responseAttempt": "aborted"}));
    Ok(())
}

#[test]
fn multi_turn_envelopes_preserve_attempt_boundaries() -> TestResult {
    // @step Given multi-turn assistant items containing start and abort markers
    for marker in [
        Content::ResponseAttemptStarted,
        Content::ResponseAttemptAborted,
    ] {
        let envelope = MultiTurnStreamItem::StreamAssistantItem(marker.clone());
        // @step When each envelope is serialized and deserialized
        let encoded = serde_json::to_value(envelope)?;
        assert_eq!(encoded["type"], "streamAssistantItem");
        let restored: MultiTurnStreamItem<Value> = serde_json::from_value(encoded)?;
        // @step Then both boundary identities are preserved
        match restored {
            MultiTurnStreamItem::StreamAssistantItem(content) => assert_eq!(content, marker),
            other => return Err(format!("unexpected envelope: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn existing_content_encodings_remain_unchanged() -> TestResult {
    // @step Given text and final response content without attempt markers
    for (content, expected) in [
        (Content::text("hello"), json!({"text": "hello"})),
        (Content::Final(json!("finished")), json!("finished")),
    ] {
        // @step When the content is serialized and deserialized
        let encoded = serde_json::to_value(&content)?;
        // @step Then its existing JSON representation and identity are preserved
        assert_eq!(encoded, expected);
        assert_eq!(serde_json::from_value::<Content>(encoded)?, content);
    }
    Ok(())
}
