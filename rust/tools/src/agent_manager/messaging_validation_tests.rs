//! Feature: spec/features/agent-manager-messaging.feature
//!
//! AMGR-010 peer-to-peer, parameter validation, sender identity tests, moved out of tests.rs (COV-075: one test file per feature).
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
// Scenario: Peer-to-peer messaging between subordinates
// ============================================================
// @step Given a supervisor has spawned two subordinate sessions A and B
// @step When subordinate A calls AgentManager with action "message" to subordinate B with message "coordinate on task X"
// @step Then the response should contain "delivered" as true
// @step And subordinate B's incoming message channel should contain the message from A
#[test]
#[serial]
fn test_message_peer_to_peer() {
    with_clean_handlers(|| {
        let supervisor_id = Uuid::new_v4();
        let sub_a_id = Uuid::new_v4();
        let sub_b_id = Uuid::new_v4().to_string();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: supervisor_id.to_string(),
                name: "Supervisor".to_string(),
                role: None,
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![sub_a_id.to_string(), sub_b_id.clone()],
                pending_request: None,
            },
            SessionEntry {
                session_id: sub_a_id.to_string(),
                name: "Worker A".to_string(),
                role: Some("analyzer".to_string()),
                status: "running".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
            SessionEntry {
                session_id: sub_b_id.clone(),
                name: "Worker B".to_string(),
                role: Some("reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        // Register handler for sub_a (the sender)
        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(sub_a_id, Some(handler));

        let result = execute_agent_manager(
            sub_a_id,
            AgentManagerAction::Message {
                session_id: sub_b_id.clone(),
                message: "coordinate on task X".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::MessageDelivered {
                delivered,
                session_id,
            } => {
                assert!(delivered);
                assert_eq!(session_id, sub_b_id);
            }
            other => panic!("Expected MessageDelivered, got: {other:?}"),
        }

        // Verify message from A is in the log
        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].source_session_id, sub_a_id.to_string());
        assert_eq!(log[0].role_name, "analyzer");
        assert_eq!(log[0].message, "coordinate on task X");
    });
}

// ============================================================
// Scenario: Missing session_id returns invalid parameter error
// ============================================================
// @step Given a session with AgentManager available
// @step When the agent calls AgentManager with action "message" without a session_id
// @step Then the response should contain "error" as true
// @step And the response should contain "code" as "invalid_parameter"
#[test]
fn test_message_missing_session_id() {
    // Missing session_id should fail at serde deserialization level
    let json = r#"{"action": "message", "message": "hello"}"#;
    let result: Result<AgentManagerArgs, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Message action without session_id should fail deserialization"
    );
}

// ============================================================
// Scenario: Missing message text returns invalid parameter error
// ============================================================
// @step Given a session with AgentManager available
// @step When the agent calls AgentManager with action "message" with session_id but without message text
// @step Then the response should contain "error" as true
// @step And the response should contain "code" as "invalid_parameter"
#[test]
fn test_message_missing_message_text() {
    // Missing message should fail at serde deserialization level
    let json = r#"{"action": "message", "session_id": "abc-123"}"#;
    let result: Result<AgentManagerArgs, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Message action without message text should fail deserialization"
    );
}

// ============================================================
// Scenario: Delivered message includes sender identity
// ============================================================
// @step Given a supervisor session with role "security-reviewer" has spawned a subordinate
// @step When the supervisor sends a message "Check for XSS" to the subordinate
// @step Then the IncomingMessage should have source_session_id matching the supervisor's ID
// @step And the IncomingMessage should have role_name "security-reviewer"
// @step And the IncomingMessage should have the message text "Check for XSS"
#[test]
#[serial]
fn test_message_includes_sender_identity() {
    with_clean_handlers(|| {
        let supervisor_id = Uuid::new_v4();
        let subordinate_id = Uuid::new_v4().to_string();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: supervisor_id.to_string(),
                name: "Supervisor".to_string(),
                role: Some("security-reviewer".to_string()),
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![subordinate_id.clone()],
                pending_request: None,
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(supervisor_id, Some(handler));

        execute_agent_manager(
            supervisor_id,
            AgentManagerAction::Message {
                session_id: subordinate_id,
                message: "Check for XSS".to_string(),
                context: None,
            },
        );

        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].source_session_id, supervisor_id.to_string());
        assert_eq!(log[0].role_name, "security-reviewer");
        assert_eq!(log[0].message, "Check for XSS");
    });
}

// ============================================================
// Scenario: Sender without role delivers empty role name
// ============================================================
// @step Given a supervisor session with no role has spawned a subordinate
// @step When the supervisor sends a message "Do analysis" to the subordinate
// @step Then the IncomingMessage should have role_name as empty string
#[test]
#[serial]
fn test_message_sender_without_role() {
    with_clean_handlers(|| {
        let supervisor_id = Uuid::new_v4();
        let subordinate_id = Uuid::new_v4().to_string();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: supervisor_id.to_string(),
                name: "Supervisor".to_string(),
                role: None, // No role
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![subordinate_id.clone()],
                pending_request: None,
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(supervisor_id, Some(handler));

        execute_agent_manager(
            supervisor_id,
            AgentManagerAction::Message {
                session_id: subordinate_id,
                message: "Do analysis".to_string(),
                context: None,
            },
        );

        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].role_name, ""); // Empty string when no role
    });
}
