//! Feature: spec/features/agent-manager-core.feature
//!
//! AMGR-009 tool definition and types serialization tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::types::*;
use uuid::Uuid;

// ============================================================
// Scenario: Tool is registered in all providers
// ============================================================
// @step Given the AgentManager tool is implemented
// @step When a session is created with any of the 5 providers
// @step Then the AgentManagerTool should be included in the agent's tool set
// @step And the tool should accept the session_id parameter
#[tokio::test]
async fn test_tool_definition_and_construction() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    // Verify the tool has the correct name
    assert_eq!(AgentManagerTool::NAME, "AgentManager");

    // Verify the definition includes all actions
    let definition = tool.definition(String::new()).await;
    assert_eq!(definition.name, "AgentManager");
    assert!(definition.description.contains("spawn"));
    assert!(definition.description.contains("list"));
    assert!(definition.description.contains("get_status"));
    assert!(definition.description.contains("close"));

    // Verify the schema has action as required
    let params = &definition.parameters;
    let required = params.get("required").unwrap().as_array().unwrap();
    assert!(required.iter().any(|v| v.as_str() == Some("action")));

    // Verify action enum values
    let action_prop = params.get("properties").unwrap().get("action").unwrap();
    let action_enum = action_prop.get("enum").unwrap().as_array().unwrap();
    let action_values: Vec<&str> = action_enum.iter().filter_map(|v| v.as_str()).collect();
    assert!(action_values.contains(&"spawn"));
    assert!(action_values.contains(&"list"));
    assert!(action_values.contains(&"get_status"));
    assert!(action_values.contains(&"close"));
}

// ============================================================
// Types serialization tests
// ============================================================
#[test]
fn test_spawn_action_deserializes() {
    let json = r#"{"action": "spawn"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Spawn { role } => {
            assert!(role.is_none());
        }
        _ => panic!("Expected Spawn action"),
    }
}

#[test]
fn test_spawn_action_with_role_deserializes() {
    let json = r#"{"action": "spawn", "role": "You are a security reviewer"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Spawn { role } => {
            assert_eq!(role.as_deref(), Some("You are a security reviewer"));
        }
        _ => panic!("Expected Spawn action"),
    }
}

#[test]
fn test_list_action_deserializes() {
    let json = r#"{"action": "list"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::List => {}
        _ => panic!("Expected List action"),
    }
}

#[test]
fn test_get_status_action_deserializes() {
    let json = r#"{"action": "get_status", "session_id": "abc-123"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::GetStatus { session_id } => {
            assert_eq!(session_id, "abc-123");
        }
        _ => panic!("Expected GetStatus action"),
    }
}

#[test]
fn test_close_action_deserializes() {
    let json = r#"{"action": "close", "session_id": "def-456"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Close { session_id } => {
            assert_eq!(session_id, "def-456");
        }
        _ => panic!("Expected Close action"),
    }
}

#[test]
fn test_result_serialization_spawned() {
    let result = AgentManagerResult::Spawned {
        session_id: "abc-123".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("abc-123"));
    assert!(json.contains("session_id"));
}

#[test]
fn test_result_serialization_error() {
    let result = AgentManagerResult::session_not_found("abc-123");
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("session_not_found"));
    assert!(json.contains("abc-123"));
    assert!(json.contains("\"error\":true"));
}

#[test]
fn test_result_serialization_closed() {
    let result = AgentManagerResult::Closed {
        closed: true,
        session_id: "abc-123".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"closed\":true"));
    assert!(json.contains("abc-123"));
}

#[test]
fn test_error_helpers() {
    let perm = AgentManagerResult::permission_denied("no access");
    match perm {
        AgentManagerResult::Error {
            error,
            code,
            message,
        } => {
            assert!(error);
            assert_eq!(code, "permission_denied");
            assert_eq!(message, "no access");
        }
        _ => panic!("Expected Error"),
    }

    let invalid = AgentManagerResult::invalid_parameter("bad param");
    match invalid {
        AgentManagerResult::Error {
            error,
            code,
            message,
        } => {
            assert!(error);
            assert_eq!(code, "invalid_parameter");
            assert_eq!(message, "bad param");
        }
        _ => panic!("Expected Error"),
    }
}
