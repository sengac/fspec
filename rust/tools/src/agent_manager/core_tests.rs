//! Feature: spec/features/agent-manager-core.feature
//!
//! AMGR-009 spawn, list and get_status tests, moved out of tests.rs (COV-075: one test file per feature).
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
// Scenario: Spawn subordinate session without role
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step When I call AgentManager with action "spawn" and no role
// @step Then I should receive a JSON response with a valid session_id
// @step And the subordinate session should exist with idle status
// @step And the subordinate should inherit the spawner's model
// @step And the ChainOfCommand should record the spawner-subordinate relationship
#[test]
#[serial]
fn test_spawn_without_role() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();

        let (handler, called) = mock_handler(vec![]);
        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(session_id, AgentManagerAction::Spawn { role: None });

        assert!(called.load(Ordering::SeqCst));
        match result {
            AgentManagerResult::Spawned {
                session_id: spawned_id,
            } => {
                // Validate it returned a UUID-like string
                assert!(!spawned_id.is_empty());
                assert!(Uuid::parse_str(&spawned_id).is_ok());
            }
            other => panic!("Expected Spawned result, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Spawn subordinate session with role
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step When I call AgentManager with action "spawn" and role "You are a security reviewer"
// @step Then I should receive a JSON response with a valid session_id
// @step And the subordinate session should have role "You are a security reviewer"
#[test]
#[serial]
fn test_spawn_with_role() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();

        // Use a handler that verifies the role parameter is passed through
        let role_received = Arc::new(std::sync::Mutex::new(None::<String>));
        let role_received_clone = role_received.clone();

        let handler: AgentManagerHandler = Arc::new(move |action, _sid| match action {
            AgentManagerAction::Spawn { role } => {
                *role_received_clone.lock().unwrap() = role;
                AgentManagerResult::Spawned {
                    session_id: Uuid::new_v4().to_string(),
                }
            }
            _ => AgentManagerResult::invalid_parameter("unexpected action"),
        });

        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::Spawn {
                role: Some("You are a security reviewer".to_string()),
            },
        );

        match result {
            AgentManagerResult::Spawned {
                session_id: spawned_id,
            } => {
                assert!(Uuid::parse_str(&spawned_id).is_ok());
            }
            other => panic!("Expected Spawned result, got: {other:?}"),
        }

        // Verify role was received by the handler
        let received = role_received.lock().unwrap();
        assert_eq!(received.as_deref(), Some("You are a security reviewer"));
    });
}

// ============================================================
// Scenario: List all sessions with relationships
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step And I have spawned 2 subordinate sessions
// @step When I call AgentManager with action "list"
// @step Then I should receive a JSON response with a sessions array
// @step And each session entry should include session_id, name, role, status, spawner_id, and subordinate_ids
// @step And my session should show 2 subordinate_ids
// @step And each subordinate should show my session as spawner_id
#[test]
#[serial]
fn test_list_sessions_with_relationships() {
    with_clean_handlers(|| {
        let spawner_id = Uuid::new_v4();
        let sub1_id = Uuid::new_v4().to_string();
        let sub2_id = Uuid::new_v4().to_string();

        let sessions = vec![
            SessionEntry {
                session_id: spawner_id.to_string(),
                name: "Supervisor".to_string(),
                role: None,
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![sub1_id.clone(), sub2_id.clone()],
                pending_request: None,
            },
            SessionEntry {
                session_id: sub1_id.clone(),
                name: "Worker 1".to_string(),
                role: Some("security reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
            SessionEntry {
                session_id: sub2_id.clone(),
                name: "Worker 2".to_string(),
                role: Some("test writer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(spawner_id, AgentManagerAction::List);

        match result {
            AgentManagerResult::Listed { sessions } => {
                assert_eq!(sessions.len(), 3);

                // Find the spawner entry
                let spawner = sessions
                    .iter()
                    .find(|s| s.session_id == spawner_id.to_string())
                    .unwrap();
                assert_eq!(spawner.subordinate_ids.len(), 2);
                assert!(spawner.spawner_id.is_none());

                // Check subordinates point back to spawner
                let sub1 = sessions.iter().find(|s| s.session_id == sub1_id).unwrap();
                assert_eq!(
                    sub1.spawner_id.as_deref(),
                    Some(spawner_id.to_string().as_str())
                );
                assert_eq!(sub1.role.as_deref(), Some("security reviewer"));

                let sub2 = sessions.iter().find(|s| s.session_id == sub2_id).unwrap();
                assert_eq!(
                    sub2.spawner_id.as_deref(),
                    Some(spawner_id.to_string().as_str())
                );
            }
            other => panic!("Expected Listed result, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Get status of an existing session
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step And I have spawned a subordinate session
// @step When I call AgentManager with action "get_status" and the subordinate's session_id
// @step Then I should receive a JSON response with session_id, role, status, model, spawner_id, subordinate_ids, and pending_messages
#[test]
#[serial]
fn test_get_status_existing_session() {
    with_clean_handlers(|| {
        let spawner_id = Uuid::new_v4();
        let sub_id = Uuid::new_v4().to_string();

        let sessions = vec![SessionEntry {
            session_id: sub_id.clone(),
            name: "Worker".to_string(),
            role: Some("security reviewer".to_string()),
            status: "idle".to_string(),
            spawner_id: Some(spawner_id.to_string()),
            subordinate_ids: vec![],
            pending_request: None,
        }];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(
            spawner_id,
            AgentManagerAction::GetStatus {
                session_id: sub_id.clone(),
            },
        );

        match result {
            AgentManagerResult::Status(status) => {
                assert_eq!(status.session_id, sub_id);
                assert_eq!(status.role.as_deref(), Some("security reviewer"));
                assert_eq!(status.status, "idle");
                assert!(status.model.is_some());
                assert_eq!(
                    status.spawner_id.as_deref(),
                    Some(spawner_id.to_string().as_str())
                );
                assert!(status.subordinate_ids.is_empty());
                assert_eq!(status.pending_messages, 0);
            }
            other => panic!("Expected Status result, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Get status of a nonexistent session
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step When I call AgentManager with action "get_status" and session_id "nonexistent-uuid"
// @step Then I should receive an error response with code "session_not_found"
#[test]
#[serial]
fn test_get_status_nonexistent_session() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();
        let (handler, _) = mock_handler(vec![]);
        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::GetStatus {
                session_id: "nonexistent-uuid".to_string(),
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
            other => panic!("Expected Error result, got: {other:?}"),
        }
    });
}
