//! Feature: spec/features/agent-manager-context-resolution.feature
//!
//! AMGR-011 result serialization and backward compatibility tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::types::*;
use uuid::Uuid;

// ============================================================
// Additional: MessageDeliveredWithContext result serialization
// ============================================================
#[test]
fn test_result_serialization_message_delivered_with_context() {
    let result = AgentManagerResult::MessageDeliveredWithContext {
        delivered: true,
        session_id: "target-123".to_string(),
        context_resolved: 2,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"delivered\":true"));
    assert!(json.contains("\"context_resolved\":2"));
    assert!(json.contains("target-123"));
}

// ============================================================
// Additional: Message without context omits context_resolved in JSON
// ============================================================
#[test]
fn test_result_serialization_plain_message_no_context_field() {
    let result = AgentManagerResult::MessageDelivered {
        delivered: true,
        session_id: "target-123".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"delivered\":true"));
    assert!(!json.contains("context_resolved")); // No context_resolved in plain delivery
}

// ============================================================
// Additional: Message with empty context array behaves as no context
// ============================================================
#[test]
fn test_message_with_empty_context_array_deserializes() {
    let json = r#"{
        "action": "message",
        "session_id": "target-123",
        "message": "hello",
        "context": []
    }"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Message { context, .. } => {
            let ctx = context.unwrap();
            assert!(ctx.is_empty());
        }
        _ => panic!("Expected Message"),
    }
}

// ============================================================
// Additional: Message without context field deserializes correctly (backward compat)
// ============================================================
#[test]
fn test_message_without_context_field_backward_compat() {
    let json = r#"{"action": "message", "session_id": "target-123", "message": "hello"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Message {
            session_id,
            message,
            context,
        } => {
            assert_eq!(session_id, "target-123");
            assert_eq!(message, "hello");
            assert!(context.is_none()); // Default to None when omitted
        }
        _ => panic!("Expected Message action"),
    }
}

// ============================================================
// Additional: Mixed context array deserialization
// ============================================================
#[test]
fn test_mixed_context_array_deserialization() {
    let json = r#"{
        "action": "message",
        "session_id": "target",
        "message": "cross-ref",
        "context": [
            {"session_id": "A", "turns": [3]},
            {"session_id": "B", "query": "auth"},
            {"session_id": "C", "start_turn": 0, "end_turn": 5}
        ]
    }"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Message { context, .. } => {
            let ctx = context.unwrap();
            assert_eq!(ctx.len(), 3);
            assert!(matches!(&ctx[0], ContextReference::Turns { .. }));
            assert!(matches!(&ctx[1], ContextReference::Query { .. }));
            assert!(matches!(&ctx[2], ContextReference::TurnRange { .. }));
        }
        _ => panic!("Expected Message"),
    }
}

// ============================================================
// Additional: Tool definition includes context parameter
// ============================================================
#[tokio::test]
async fn test_tool_definition_includes_context_parameter() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    let definition = tool.definition(String::new()).await;
    let props = definition.parameters.get("properties").unwrap();
    assert!(
        props.get("context").is_some(),
        "Tool definition should include 'context' parameter for AMGR-011"
    );
}
