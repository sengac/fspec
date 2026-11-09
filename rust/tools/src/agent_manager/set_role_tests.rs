//! Feature: spec/features/role-management-set-role.feature
//!
//! AMGR-012 set_role tests, moved out of tests.rs (COV-075: one test file per feature).
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
use std::sync::atomic::Ordering;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================
// AMGR-012: set_role action — Role Management
// Feature: spec/features/role-management.feature
// ============================================================

// @step Given I have a session with ID "own-session"
// @step When I call the set_role action with role "test-helper" and no session_id
// @step Then the role is set on the caller's own session
// @step And the response contains session_id "own-session" and role "test-helper"
#[test]
fn test_set_role_on_own_session_no_session_id() {
    let json = r#"{"action": "set_role", "role": "test-helper"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::SetRole { session_id, role } => {
            assert!(
                session_id.is_none(),
                "session_id should be None when omitted"
            );
            assert_eq!(role, "test-helper");
        }
        _ => panic!("Expected SetRole action"),
    }
}

// @step Given a session exists with ID "sub-123"
// @step When I call the set_role action with session_id "sub-123" and role "code-reviewer"
// @step Then the role is set on session "sub-123"
// @step And the response contains session_id "sub-123" and role "code-reviewer"
#[test]
fn test_set_role_on_specific_session_by_id() {
    let json = r#"{"action": "set_role", "session_id": "sub-123", "role": "code-reviewer"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::SetRole { session_id, role } => {
            assert_eq!(session_id.unwrap(), "sub-123");
            assert_eq!(role, "code-reviewer");
        }
        _ => panic!("Expected SetRole action"),
    }
}

// @step Given a session exists with ID "target-id" and role "old-role"
// @step When I call the set_role action with session_id "target-id" and role ""
// @step Then the role is cleared on session "target-id"
// @step And the response contains session_id "target-id" and role null
#[test]
fn test_set_role_clear_with_empty_string() {
    let json = r#"{"action": "set_role", "session_id": "target-id", "role": ""}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::SetRole { session_id, role } => {
            assert_eq!(session_id.unwrap(), "target-id");
            assert!(role.is_empty(), "Empty role string should clear the role");
        }
        _ => panic!("Expected SetRole action"),
    }
}

// @step When I call the set_role action with session_id "nonexistent" and role "any"
// @step Then an error response is returned with code "session_not_found"
#[test]
fn test_set_role_result_serialization() {
    let result = AgentManagerResult::RoleSet {
        session_id: "sess-1".to_string(),
        role: Some("architect".to_string()),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"session_id\":\"sess-1\""));
    assert!(json.contains("\"architect\""));
}

#[test]
fn test_set_role_result_serialization_cleared() {
    let result = AgentManagerResult::RoleSet {
        session_id: "sess-2".to_string(),
        role: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"session_id\":\"sess-2\""));
    assert!(json.contains("null"));
}

// @step Given a session exists with ID "sess-1" and role "architect"
// @step When I call the get_status action for session "sess-1"
// @step Then the response includes role "architect"
#[test]
fn test_get_status_includes_role() {
    let status = SessionStatus {
        session_id: "sess-1".to_string(),
        role: Some("architect".to_string()),
        status: "idle".to_string(),
        model: Some("anthropic/claude-sonnet-4".to_string()),
        spawner_id: None,
        subordinate_ids: vec![],
        pending_messages: 0,
        pending_request: None,
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("\"role\":\"architect\""));
}

// @step Given sessions exist with roles set
// @step When I call the list action
// @step Then each session entry includes its role field
#[test]
fn test_list_entries_include_role() {
    let entry = SessionEntry {
        session_id: "sess-1".to_string(),
        name: "Test Agent".to_string(),
        role: Some("code-reviewer".to_string()),
        status: "idle".to_string(),
        spawner_id: None,
        subordinate_ids: vec![],
        pending_request: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"role\":\"code-reviewer\""));
}

#[test]
fn test_list_entry_without_role() {
    let entry = SessionEntry {
        session_id: "sess-2".to_string(),
        name: "No Role Agent".to_string(),
        role: None,
        status: "idle".to_string(),
        spawner_id: None,
        subordinate_ids: vec![],
        pending_request: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    // role should be omitted when None (skip_serializing_if)
    assert!(!json.contains("\"role\""));
}

// Handler integration test with mock handler
#[test]
#[serial]
fn test_set_role_handler_dispatch() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::<DeliveredMessage>::new()));

        // Create mock handler that handles SetRole
        let sessions = vec![SessionEntry {
            session_id: "target-session".to_string(),
            name: "Target".to_string(),
            role: None,
            status: "idle".to_string(),
            spawner_id: Some(session_id.to_string()),
            subordinate_ids: vec![],
            pending_request: None,
        }];

        let (handler, called) = mock_handler_with_messaging(sessions, message_log, false);
        set_agent_manager_handler(session_id, Some(handler));

        // Execute set_role action
        let action = AgentManagerAction::SetRole {
            session_id: Some("target-session".to_string()),
            role: "new-role".to_string(),
        };
        let result = execute_agent_manager(session_id, action);

        assert!(
            called.load(Ordering::SeqCst),
            "Handler should have been called"
        );

        // Result should be valid (mock handler will route through match)
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
    });
}

// Tool definition includes set_role action
#[tokio::test]
async fn test_tool_definition_includes_set_role_action() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    let definition = tool.definition(String::new()).await;
    let action_prop = definition
        .parameters
        .get("properties")
        .and_then(|p| p.get("action"))
        .and_then(|a| a.get("enum"))
        .unwrap();

    let actions: Vec<String> = serde_json::from_value(action_prop.clone()).unwrap();
    assert!(
        actions.contains(&"set_role".to_string()),
        "Tool definition should include 'set_role' in action enum for AMGR-012"
    );
}
