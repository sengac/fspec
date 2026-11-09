//! Feature: spec/features/agent-manager-messaging.feature
//!
//! AMGR-010 supervisor/subordinate delivery tests, moved out of tests.rs (COV-075: one test file per feature).
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
// AMGR-010: Agent Messaging Tests
// Feature: spec/features/agent-manager-messaging.feature
// ============================================================

// ============================================================
// Scenario: Supervisor sends task to subordinate
// ============================================================
// @step Given a supervisor session has spawned a subordinate session
// @step When the supervisor calls AgentManager with action "message", session_id of subordinate, and message "Analyze auth.rs for security issues"
// @step Then the response should contain "delivered" as true
// @step And the response should contain "session_id" matching the subordinate's ID
// @step And the subordinate's incoming message channel should contain the message
#[test]
#[serial]
fn test_message_supervisor_to_subordinate() {
    with_clean_handlers(|| {
        let supervisor_id = Uuid::new_v4();
        let subordinate_id = Uuid::new_v4().to_string();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: supervisor_id.to_string(),
                name: "Supervisor".to_string(),
                role: Some("project-lead".to_string()),
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![subordinate_id.clone()],
                pending_request: None,
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: Some("security-reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(supervisor_id, Some(handler));

        let result = execute_agent_manager(
            supervisor_id,
            AgentManagerAction::Message {
                session_id: subordinate_id.clone(),
                message: "Analyze auth.rs for security issues".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::MessageDelivered {
                delivered,
                session_id,
            } => {
                assert!(delivered);
                assert_eq!(session_id, subordinate_id);
            }
            other => panic!("Expected MessageDelivered, got: {other:?}"),
        }

        // Verify the message was logged (simulating channel delivery)
        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "Analyze auth.rs for security issues");
        assert_eq!(log[0].source_session_id, supervisor_id.to_string());
    });
}

// ============================================================
// Scenario: Subordinate reports results to supervisor
// ============================================================
// @step Given a supervisor session has spawned a subordinate session
// @step When the subordinate calls AgentManager with action "message", session_id of supervisor, and message "Found 2 SQL injection vulnerabilities"
// @step Then the response should contain "delivered" as true
// @step And the response should contain "session_id" matching the supervisor's ID
#[test]
#[serial]
fn test_message_subordinate_to_supervisor() {
    with_clean_handlers(|| {
        let supervisor_id = Uuid::new_v4();
        let subordinate_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: supervisor_id.to_string(),
                name: "Supervisor".to_string(),
                role: None,
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![subordinate_id.to_string()],
                pending_request: None,
            },
            SessionEntry {
                session_id: subordinate_id.to_string(),
                name: "Worker".to_string(),
                role: Some("code-reviewer".to_string()),
                status: "running".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        let (handler, _) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(subordinate_id, Some(handler));

        let result = execute_agent_manager(
            subordinate_id,
            AgentManagerAction::Message {
                session_id: supervisor_id.to_string(),
                message: "Found 2 SQL injection vulnerabilities".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::MessageDelivered {
                delivered,
                session_id,
            } => {
                assert!(delivered);
                assert_eq!(session_id, supervisor_id.to_string());
            }
            other => panic!("Expected MessageDelivered, got: {other:?}"),
        }

        // Verify sender identity in message log
        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].source_session_id, subordinate_id.to_string());
        assert_eq!(log[0].role_name, "code-reviewer");
    });
}

// ============================================================
// Scenario: Message to nonexistent session returns error
// ============================================================
// @step Given a session with AgentManager available
// @step When the agent calls AgentManager with action "message", session_id "nonexistent-uuid", and message "hello"
// @step Then the response should contain "error" as true
// @step And the response should contain "code" as "session_not_found"
// @step And the response should contain a "message" string
#[test]
#[serial]
fn test_message_nonexistent_session() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let (handler, _) = mock_handler_with_messaging(vec![], message_log, false);
        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::Message {
                session_id: "nonexistent-uuid".to_string(),
                message: "hello".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::Error {
                error,
                code,
                message,
            } => {
                assert!(error);
                assert_eq!(code, "session_not_found");
                assert!(message.contains("nonexistent-uuid"));
            }
            other => panic!("Expected Error with session_not_found, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Channel full returns delivery failed error
// ============================================================
// @step Given a target session with its incoming message channel full at capacity 16
// @step When the agent calls AgentManager with action "message" to the target session
// @step Then the response should contain "error" as true
// @step And the response should contain "code" as "delivery_failed"
// @step And the response should contain a "message" string
#[test]
#[serial]
fn test_message_channel_full() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4().to_string();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![SessionEntry {
            session_id: target_id.clone(),
            name: "Busy Worker".to_string(),
            role: None,
            status: "running".to_string(),
            spawner_id: None,
            subordinate_ids: vec![],
            pending_request: None,
        }];

        // channel_full = true simulates a full mpsc channel
        let (handler, _) = mock_handler_with_messaging(sessions, message_log, true);
        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id,
                message: "this should fail".to_string(),
                context: None,
            },
        );

        match result {
            AgentManagerResult::Error {
                error,
                code,
                message,
            } => {
                assert!(error);
                assert_eq!(code, "delivery_failed");
                assert!(!message.is_empty());
            }
            other => panic!("Expected Error with delivery_failed, got: {other:?}"),
        }
    });
}
