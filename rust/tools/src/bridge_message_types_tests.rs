//! Tests for the bridge message types that carry fspec command traffic
//!
//! Feature: spec/features/bridge-command-message-types.feature
//!
//! Inbound traffic is parsed as a multiplexed `Envelope` (the flat
//! InboundMessage struct was removed); `OutboundMessage` in bridge.rs keeps
//! its optional `request_id` for commandResponse correlation.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod message_types_tests {
    use crate::bridge::OutboundMessage;
    use crate::bridge_multiplexed::{route_inbound, Envelope, InboundAction};
    use serde_json::json;

    #[test]
    fn test_deserialize_fspec_command_envelope_with_all_fields() {
        // @step Given a JSON envelope with service "fspec", type "command", request_id, and data command and args
        let text = r#"{"service":"fspec","type":"command","session_id":"s1","request_id":"r1","data":{"command":"board","args":{"status":"backlog"}}}"#;

        // @step When the JSON is deserialized into an Envelope and routed with route_inbound
        let envelope: Envelope = serde_json::from_str(text).unwrap();
        let action = route_inbound(&envelope);

        // @step Then route_inbound returns FspecCommand with request_id, command, and args_json populated from the JSON values
        match action {
            InboundAction::FspecCommand {
                request_id,
                command,
                args_json,
            } => {
                assert_eq!(request_id, "r1");
                assert_eq!(command, "board");
                let args: serde_json::Value = serde_json::from_str(&args_json).unwrap();
                assert_eq!(args, json!({"status": "backlog"}));
            }
            other => panic!("Expected FspecCommand, got {other:?}"),
        }
    }

    #[test]
    fn test_deserialize_session_input_envelope_without_command_fields() {
        // @step Given a JSON envelope with service "session", type "input" and session_id but no request_id
        let text = r#"{"service":"session","type":"input","session_id":"test-123","data":{"message":"Hello"}}"#;

        // @step When the JSON is deserialized into an Envelope and routed with route_inbound
        let envelope: Envelope = serde_json::from_str(text).unwrap();
        let action = route_inbound(&envelope);

        // @step Then the envelope request_id should be None and route_inbound returns SessionInput
        assert!(envelope.request_id.is_none());
        match action {
            InboundAction::SessionInput {
                session_id,
                message,
                images,
            } => {
                assert_eq!(session_id, "test-123");
                assert_eq!(message, "Hello");
                assert!(images.is_none());
            }
            other => panic!("Expected SessionInput, got {other:?}"),
        }
    }

    #[test]
    fn test_serialize_outbound_message_with_request_id() {
        // @step Given an OutboundMessage with type commandResponse and request_id set to a value
        let message = OutboundMessage {
            msg_type: "commandResponse".to_string(),
            session_id: "s1".to_string(),
            data: json!({"command": "board", "success": true}),
            request_id: Some("req-001".to_string()),
        };

        // @step When the OutboundMessage is serialized to JSON
        let value = serde_json::to_value(&message).unwrap();

        // @step Then the JSON output should contain both the type and request_id fields
        assert_eq!(value["type"], "commandResponse");
        assert_eq!(value["request_id"], "req-001");
    }

    #[test]
    fn test_serialize_outbound_message_without_request_id() {
        // @step Given an OutboundMessage with type chunk and request_id set to None
        let message = OutboundMessage {
            msg_type: "chunk".to_string(),
            session_id: "s1".to_string(),
            data: json!({"type": "text", "text": "hi"}),
            request_id: None,
        };

        // @step When the OutboundMessage is serialized to JSON
        let value = serde_json::to_value(&message).unwrap();

        // @step Then the JSON output should NOT contain a request_id field
        assert_eq!(value["type"], "chunk");
        assert!(
            value.get("request_id").is_none(),
            "request_id must be omitted, got {value}"
        );
    }
}
