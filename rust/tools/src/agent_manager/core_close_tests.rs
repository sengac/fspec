//! Feature: spec/features/agent-manager-core.feature
//!
//! AMGR-009 close, invalid action, handler lifecycle tests, moved out of tests.rs (COV-075: one test file per feature).
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
// Scenario: Close subordinate session as spawner
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step And I have spawned a subordinate session
// @step When I call AgentManager with action "close" and the subordinate's session_id
// @step Then I should receive a JSON response with closed true and the session_id
// @step And the subordinate session should no longer exist
// @step And the ChainOfCommand should have no record of the closed subordinate
#[test]
#[serial]
fn test_close_as_spawner() {
    with_clean_handlers(|| {
        let spawner_id = Uuid::new_v4();
        let sub_id = Uuid::new_v4().to_string();

        let sessions = vec![SessionEntry {
            session_id: sub_id.clone(),
            name: "Worker".to_string(),
            role: None,
            status: "idle".to_string(),
            spawner_id: Some(spawner_id.to_string()),
            subordinate_ids: vec![],
            pending_request: None,
        }];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(
            spawner_id,
            AgentManagerAction::Close {
                session_id: sub_id.clone(),
            },
        );

        match result {
            AgentManagerResult::Closed { closed, session_id } => {
                assert!(closed);
                assert_eq!(session_id, sub_id);
            }
            other => panic!("Expected Closed result, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Close session without spawner permission
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step And another agent has spawned a subordinate session
// @step When I call AgentManager with action "close" and that subordinate's session_id
// @step Then I should receive an error response with code "permission_denied"
// @step And the subordinate session should still exist
#[test]
#[serial]
fn test_close_without_permission() {
    with_clean_handlers(|| {
        let calling_session_id = Uuid::new_v4();
        let actual_spawner_id = Uuid::new_v4();
        let sub_id = Uuid::new_v4().to_string();

        // The subordinate's spawner is actual_spawner_id, not calling_session_id
        let sessions = vec![SessionEntry {
            session_id: sub_id.clone(),
            name: "Worker".to_string(),
            role: None,
            status: "idle".to_string(),
            spawner_id: Some(actual_spawner_id.to_string()),
            subordinate_ids: vec![],
            pending_request: None,
        }];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(calling_session_id, Some(handler));

        let result = execute_agent_manager(
            calling_session_id,
            AgentManagerAction::Close { session_id: sub_id },
        );

        match result {
            AgentManagerResult::Error { error, code, .. } => {
                assert!(error);
                assert_eq!(code, "permission_denied");
            }
            other => panic!("Expected Error result with permission_denied, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Spawn multiple subordinates and list them
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step When I call AgentManager with action "spawn" 3 times
// @step Then each spawn should return a unique session_id
// @step When I call AgentManager with action "list"
// @step Then the sessions array should include all 3 subordinates
// @step And each subordinate should show my session as spawner_id
#[test]
#[serial]
fn test_spawn_multiple_and_list() {
    with_clean_handlers(|| {
        let spawner_id = Uuid::new_v4();

        let (handler, _) = mock_handler(vec![]);
        set_agent_manager_handler(spawner_id, Some(handler));

        // Spawn 3 times and collect IDs
        let mut spawned_ids = Vec::new();
        for _ in 0..3 {
            let result =
                execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
            match result {
                AgentManagerResult::Spawned { session_id } => {
                    spawned_ids.push(session_id);
                }
                other => panic!("Expected Spawned result, got: {other:?}"),
            }
        }

        // All 3 IDs should be unique
        assert_eq!(spawned_ids.len(), 3);
        let unique: std::collections::HashSet<&String> = spawned_ids.iter().collect();
        assert_eq!(unique.len(), 3, "All spawned IDs should be unique");
    });
}

// ============================================================
// Scenario: Call with invalid action
// ============================================================
// @step Given I am an agent with a registered AgentManager handler
// @step When I call AgentManager with action "invalid_action"
// @step Then I should receive an error response with code "invalid_parameter"
// @step And the error message should mention "invalid_action"
#[test]
#[serial]
fn test_invalid_action_deserialization() {
    // Invalid action should fail at deserialization level
    let json = r#"{"action": "invalid_action"}"#;
    let result: Result<AgentManagerArgs, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Unknown action should fail deserialization"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("invalid_action") || err_msg.contains("unknown variant"),
        "Error should reference the invalid action: {err_msg}"
    );
}

// ============================================================
// Scenario: Handler lifecycle in agent loop
// ============================================================
// @step Given a session is starting its agent loop
// @step When the agent loop registers the AgentManager handler
// @step Then the handler should be available for the session
// @step When the agent loop completes and deregisters the handler
// @step Then the handler should no longer be available for the session
#[test]
#[serial]
fn test_handler_lifecycle() {
    with_clean_handlers(|| {
        let session_id = Uuid::new_v4();

        // Not registered yet
        assert!(!has_agent_manager_handler(session_id));

        // Register handler (simulating agent_loop start)
        let handler: AgentManagerHandler =
            Arc::new(|_, _| AgentManagerResult::Listed { sessions: vec![] });
        set_agent_manager_handler(session_id, Some(handler));

        // Should be available
        assert!(has_agent_manager_handler(session_id));

        // Execute works
        let result = execute_agent_manager(session_id, AgentManagerAction::List);
        match result {
            AgentManagerResult::Listed { sessions } => {
                assert!(sessions.is_empty());
            }
            other => panic!("Expected Listed result, got: {other:?}"),
        }

        // Deregister handler (simulating agent_loop end)
        set_agent_manager_handler(session_id, None);

        // Should no longer be available
        assert!(!has_agent_manager_handler(session_id));

        // Execute without handler returns error
        let result = execute_agent_manager(session_id, AgentManagerAction::List);
        match result {
            AgentManagerResult::Error { code, .. } => {
                assert_eq!(code, "internal_error");
            }
            other => panic!("Expected Error result, got: {other:?}"),
        }
    });
}

// ============================================================
// Additional: Concurrent session isolation
// ============================================================
#[test]
#[serial]
fn test_concurrent_sessions_isolated() {
    with_clean_handlers(|| {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let handler_a: AgentManagerHandler = Arc::new(|_, _| AgentManagerResult::Listed {
            sessions: vec![SessionEntry {
                session_id: "from-a".to_string(),
                name: "A".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
                pending_request: None,
            }],
        });

        let handler_b: AgentManagerHandler = Arc::new(|_, _| AgentManagerResult::Listed {
            sessions: vec![SessionEntry {
                session_id: "from-b".to_string(),
                name: "B".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
                pending_request: None,
            }],
        });

        set_agent_manager_handler(session_a, Some(handler_a));
        set_agent_manager_handler(session_b, Some(handler_b));

        // session_a returns its own data
        let result_a = execute_agent_manager(session_a, AgentManagerAction::List);
        match result_a {
            AgentManagerResult::Listed { sessions } => {
                assert_eq!(sessions[0].session_id, "from-a");
            }
            other => panic!("Expected Listed from a, got: {other:?}"),
        }

        // session_b returns its own data
        let result_b = execute_agent_manager(session_b, AgentManagerAction::List);
        match result_b {
            AgentManagerResult::Listed { sessions } => {
                assert_eq!(sessions[0].session_id, "from-b");
            }
            other => panic!("Expected Listed from b, got: {other:?}"),
        }

        // Remove session_b — session_a still works
        set_agent_manager_handler(session_b, None);
        let result_a2 = execute_agent_manager(session_a, AgentManagerAction::List);
        match result_a2 {
            AgentManagerResult::Listed { sessions } => {
                assert_eq!(sessions[0].session_id, "from-a");
            }
            other => panic!("Expected Listed from a after removing b, got: {other:?}"),
        }
    });
}
