//! Feature: spec/features/agent-manager-messaging.feature
//!
//! AMGR-010 self-messaging and action dispatch tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::handler::*;
use super::test_support::*;
use super::types::*;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================
// Scenario: Self-messaging to an idle session is allowed
// ============================================================
// @step Given an idle session with AgentManager available
// @step When the agent calls AgentManager with action "message" targeting its own session_id with message "note to self"
// @step Then the response should contain "delivered" as true
// @step And the session's incoming message channel should contain the self-addressed message
#[test]
#[serial]
fn test_message_self_messaging() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Session is both sender and target — self-messaging
        let sessions = vec![SessionEntry {
            session_id: session_id.to_string(),
            name: "Solo Agent".to_string(),
            role: Some("thinker".to_string()),
            status: "idle".to_string(),
            spawner_id: None,
            subordinate_ids: vec![],
            pending_request: None,
        }];

        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::Message {
                session_id: session_id.to_string(),
                message: "note to self".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::MessageDelivered {
                delivered,
                session_id: target_id,
            } => {
                assert!(delivered);
                assert_eq!(target_id, session_id.to_string());
            }
            other => panic!("Expected MessageDelivered for self-messaging, got: {other:?}"),
        }

        // Verify self-addressed message in log
        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].source_session_id, session_id.to_string());
        assert_eq!(log[0].message, "note to self");
    });
}

// ============================================================
// Scenario: Message action is dispatched through AgentManagerAction enum
// ============================================================
// @step Given the AgentManagerAction enum includes a Message variant with session_id and message fields
// @step When a message action is deserialized from JSON input
// @step Then it should produce a Message variant with the correct session_id and message values
#[test]
fn test_message_action_deserializes() {
    let json = r#"{"action": "message", "session_id": "target-123", "message": "hello world"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Message {
            session_id,
            message,
            ..
        } => {
            assert_eq!(session_id, "target-123");
            assert_eq!(message, "hello world");
        }
        _ => panic!("Expected Message action"),
    }
}

// ============================================================
// Additional: MessageDelivered result serialization
// ============================================================
#[test]
fn test_result_serialization_message_delivered() {
    let result = AgentManagerResult::MessageDelivered {
        delivered: true,
        session_id: "target-123".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"delivered\":true"));
    assert!(json.contains("target-123"));
}

// ============================================================
// Additional: delivery_failed error helper
// ============================================================
#[test]
fn test_delivery_failed_error_helper() {
    let result = AgentManagerResult::delivery_failed("channel full");
    match result {
        AgentManagerResult::Error {
            error,
            code,
            message,
        } => {
            assert!(error);
            assert_eq!(code, "delivery_failed");
            assert_eq!(message, "channel full");
        }
        _ => panic!("Expected Error"),
    }
}

// ============================================================
// Additional: Tool definition includes message action
// ============================================================
#[tokio::test]
async fn test_tool_definition_includes_message_action() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    let definition = tool.definition(String::new()).await;
    assert!(definition.description.contains("message"));

    // Verify action enum includes "message"
    let action_prop = definition
        .parameters
        .get("properties")
        .unwrap()
        .get("action")
        .unwrap();
    let action_enum = action_prop.get("enum").unwrap().as_array().unwrap();
    let action_values: Vec<&str> = action_enum.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        action_values.contains(&"message"),
        "Tool definition should include 'message' action, got: {action_values:?}"
    );

    // Verify message parameter exists
    let props = definition.parameters.get("properties").unwrap();
    assert!(
        props.get("message").is_some(),
        "Tool definition should include 'message' parameter"
    );
}
