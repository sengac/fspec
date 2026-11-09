use super::*;

/// Scenario: Normal end_turn completion has correct persistence
#[test]
fn test_assistant_message_preserves_end_turn_stop_reason() {
    let message = AssistantMessage {
        role: "assistant".to_string(),
        id: Some("msg_normal".to_string()),
        model: Some("claude-opus-4-5-20251101".to_string()),
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
