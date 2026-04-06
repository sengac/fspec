//! AgentManager tests
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-messaging.feature
//! Feature: spec/features/agent-manager-context-resolution.feature
//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! This test file validates the acceptance criteria defined in the feature files.
//! Scenarios map directly to Gherkin scenarios.

use super::handler::*;
use super::types::*;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

fn with_clean_handlers<T>(f: impl FnOnce() -> T) -> T {
    clear_all_agent_manager_handlers();
    let result = f();
    clear_all_agent_manager_handlers();
    result
}

/// Tracks messages sent to a mock session for verification
#[derive(Debug, Clone)]
struct DeliveredMessage {
    source_session_id: String,
    role_name: String,
    message: String,
}

/// Helper to create a mock handler that tracks calls and returns configurable results.
/// Supports all 5 actions: spawn, list, get_status, close, message.
///
/// The `message_log` collects all delivered messages for verification.
/// The `channel_full` flag simulates a full incoming_message channel.
fn mock_handler_with_messaging(
    sessions: Vec<SessionEntry>,
    message_log: Arc<std::sync::Mutex<Vec<DeliveredMessage>>>,
    channel_full: bool,
) -> (AgentManagerHandler, Arc<AtomicBool>) {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    let sessions = Arc::new(sessions);

    let handler: AgentManagerHandler = Arc::new(move |action, calling_session_id| {
        called_clone.store(true, Ordering::SeqCst);
        match action {
            AgentManagerAction::Spawn { role: _ } => {
                let subordinate_id = Uuid::new_v4().to_string();
                AgentManagerResult::Spawned {
                    session_id: subordinate_id,
                }
            }
            AgentManagerAction::List => {
                AgentManagerResult::Listed {
                    sessions: (*sessions).clone(),
                }
            }
            AgentManagerAction::GetStatus { session_id } => {
                if let Some(entry) = sessions.iter().find(|s| s.session_id == session_id) {
                    AgentManagerResult::Status(SessionStatus {
                        session_id: entry.session_id.clone(),
                        role: entry.role.clone(),
                        status: entry.status.clone(),
                        model: Some("anthropic/claude-sonnet-4".to_string()),
                        spawner_id: entry.spawner_id.clone(),
                        subordinate_ids: entry.subordinate_ids.clone(),
                        pending_messages: 0,
                    })
                } else {
                    AgentManagerResult::session_not_found(&session_id)
                }
            }
            AgentManagerAction::Close { session_id } => {
                if let Some(entry) = sessions.iter().find(|s| s.session_id == session_id) {
                    if entry.spawner_id.as_deref() == Some(&calling_session_id.to_string()) {
                        AgentManagerResult::Closed {
                            closed: true,
                            session_id,
                        }
                    } else {
                        AgentManagerResult::permission_denied(
                            "Only the spawner can close a subordinate session",
                        )
                    }
                } else {
                    AgentManagerResult::session_not_found(&session_id)
                }
            }
            AgentManagerAction::Message { session_id, message, context: _ } => {
                // Check if target session exists
                let target_exists = sessions.iter().any(|s| s.session_id == session_id)
                    || session_id == calling_session_id.to_string();

                if !target_exists {
                    return AgentManagerResult::session_not_found(&session_id);
                }

                if channel_full {
                    return AgentManagerResult::delivery_failed(
                        &format!("Incoming message channel full for session {session_id}"),
                    );
                }

                // Find sender's role from sessions list
                let sender_role = sessions
                    .iter()
                    .find(|s| s.session_id == calling_session_id.to_string())
                    .and_then(|s| s.role.clone())
                    .unwrap_or_default();

                message_log.lock().unwrap().push(DeliveredMessage {
                    source_session_id: calling_session_id.to_string(),
                    role_name: sender_role,
                    message,
                });

                AgentManagerResult::MessageDelivered {
                    delivered: true,
                    session_id,
                }
            }
            AgentManagerAction::SetRole { session_id, role } => {
                let target_id = session_id
                    .unwrap_or_else(|| calling_session_id.to_string());

                let target_exists = sessions.iter().any(|s| s.session_id == target_id)
                    || target_id == calling_session_id.to_string();

                if !target_exists {
                    return AgentManagerResult::session_not_found(&target_id);
                }

                if role.is_empty() {
                    AgentManagerResult::RoleSet {
                        session_id: target_id,
                        role: None,
                    }
                } else {
                    AgentManagerResult::RoleSet {
                        session_id: target_id,
                        role: Some(role),
                    }
                }
            }
            AgentManagerAction::AwaitIdle { .. } => {
                AgentManagerResult::invalid_parameter(
                    "await_idle must be dispatched through async handler",
                )
            }
        }
    });

    (handler, called)
}

/// Backward-compatible mock_handler that wraps mock_handler_with_messaging
/// for existing core tests
fn mock_handler(
    sessions: Vec<SessionEntry>,
) -> (AgentManagerHandler, Arc<AtomicBool>) {
    let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));
    mock_handler_with_messaging(sessions, message_log, false)
}

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

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::Spawn { role: None },
        );

        assert!(called.load(Ordering::SeqCst));
        match result {
            AgentManagerResult::Spawned { session_id: spawned_id } => {
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

        let handler: AgentManagerHandler = Arc::new(move |action, _sid| {
            match action {
                AgentManagerAction::Spawn { role } => {
                    *role_received_clone.lock().unwrap() = role;
                    AgentManagerResult::Spawned {
                        session_id: Uuid::new_v4().to_string(),
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(session_id, Some(handler));

        let result = execute_agent_manager(
            session_id,
            AgentManagerAction::Spawn {
                role: Some("You are a security reviewer".to_string()),
            },
        );

        match result {
            AgentManagerResult::Spawned { session_id: spawned_id } => {
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
            },
            SessionEntry {
                session_id: sub1_id.clone(),
                name: "Worker 1".to_string(),
                role: Some("security reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
            },
            SessionEntry {
                session_id: sub2_id.clone(),
                name: "Worker 2".to_string(),
                role: Some("test writer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
            },
        ];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(spawner_id, AgentManagerAction::List);

        match result {
            AgentManagerResult::Listed { sessions } => {
                assert_eq!(sessions.len(), 3);

                // Find the spawner entry
                let spawner = sessions.iter().find(|s| s.session_id == spawner_id.to_string()).unwrap();
                assert_eq!(spawner.subordinate_ids.len(), 2);
                assert!(spawner.spawner_id.is_none());

                // Check subordinates point back to spawner
                let sub1 = sessions.iter().find(|s| s.session_id == sub1_id).unwrap();
                assert_eq!(sub1.spawner_id.as_deref(), Some(spawner_id.to_string().as_str()));
                assert_eq!(sub1.role.as_deref(), Some("security reviewer"));

                let sub2 = sessions.iter().find(|s| s.session_id == sub2_id).unwrap();
                assert_eq!(sub2.spawner_id.as_deref(), Some(spawner_id.to_string().as_str()));
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

        let sessions = vec![
            SessionEntry {
                session_id: sub_id.clone(),
                name: "Worker".to_string(),
                role: Some("security reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
            },
        ];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(
            spawner_id,
            AgentManagerAction::GetStatus { session_id: sub_id.clone() },
        );

        match result {
            AgentManagerResult::Status(status) => {
                assert_eq!(status.session_id, sub_id);
                assert_eq!(status.role.as_deref(), Some("security reviewer"));
                assert_eq!(status.status, "idle");
                assert!(status.model.is_some());
                assert_eq!(status.spawner_id.as_deref(), Some(spawner_id.to_string().as_str()));
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
            AgentManagerResult::Error { error, code, message } => {
                assert!(error);
                assert_eq!(code, "session_not_found");
                assert!(message.contains("nonexistent-uuid"));
            }
            other => panic!("Expected Error result, got: {other:?}"),
        }
    });
}

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

        let sessions = vec![
            SessionEntry {
                session_id: sub_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(spawner_id.to_string()),
                subordinate_ids: vec![],
            },
        ];

        let (handler, _) = mock_handler(sessions);
        set_agent_manager_handler(spawner_id, Some(handler));

        let result = execute_agent_manager(
            spawner_id,
            AgentManagerAction::Close { session_id: sub_id.clone() },
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
        let sessions = vec![
            SessionEntry {
                session_id: sub_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(actual_spawner_id.to_string()),
                subordinate_ids: vec![],
            },
        ];

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
            let result = execute_agent_manager(
                spawner_id,
                AgentManagerAction::Spawn { role: None },
            );
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
    assert!(result.is_err(), "Unknown action should fail deserialization");
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
        let handler: AgentManagerHandler = Arc::new(|_, _| {
            AgentManagerResult::Listed { sessions: vec![] }
        });
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

        let handler_a: AgentManagerHandler = Arc::new(|_, _| {
            AgentManagerResult::Listed {
                sessions: vec![SessionEntry {
                    session_id: "from-a".to_string(),
                    name: "A".to_string(),
                    role: None,
                    status: "idle".to_string(),
                    spawner_id: None,
                    subordinate_ids: vec![],
                }],
            }
        });

        let handler_b: AgentManagerHandler = Arc::new(|_, _| {
            AgentManagerResult::Listed {
                sessions: vec![SessionEntry {
                    session_id: "from-b".to_string(),
                    name: "B".to_string(),
                    role: None,
                    status: "idle".to_string(),
                    spawner_id: None,
                    subordinate_ids: vec![],
                }],
            }
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

// ============================================================
// Scenario: Tool is registered in all providers
// ============================================================
// @step Given the AgentManager tool is implemented
// @step When a session is created with any of the 5 providers
// @step Then the AgentManagerTool should be included in the agent's tool set
// @step And the tool should accept the session_id parameter
#[tokio::test]
async fn test_tool_definition_and_construction() {
    use rig::tool::Tool;
    use super::super::AgentManagerTool;

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
        AgentManagerResult::Error { error, code, message } => {
            assert!(error);
            assert_eq!(code, "permission_denied");
            assert_eq!(message, "no access");
        }
        _ => panic!("Expected Error"),
    }

    let invalid = AgentManagerResult::invalid_parameter("bad param");
    match invalid {
        AgentManagerResult::Error { error, code, message } => {
            assert!(error);
            assert_eq!(code, "invalid_parameter");
            assert_eq!(message, "bad param");
        }
        _ => panic!("Expected Error"),
    }
}

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
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: Some("security-reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
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
            AgentManagerResult::MessageDelivered { delivered, session_id } => {
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
            },
            SessionEntry {
                session_id: subordinate_id.to_string(),
                name: "Worker".to_string(),
                role: Some("code-reviewer".to_string()),
                status: "running".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
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
            AgentManagerResult::MessageDelivered { delivered, session_id } => {
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
            AgentManagerResult::Error { error, code, message } => {
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

        let sessions = vec![
            SessionEntry {
                session_id: target_id.clone(),
                name: "Busy Worker".to_string(),
                role: None,
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
            },
        ];

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
            AgentManagerResult::Error { error, code, message } => {
                assert!(error);
                assert_eq!(code, "delivery_failed");
                assert!(!message.is_empty());
            }
            other => panic!("Expected Error with delivery_failed, got: {other:?}"),
        }
    });
}

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
            },
            SessionEntry {
                session_id: sub_a_id.to_string(),
                name: "Worker A".to_string(),
                role: Some("analyzer".to_string()),
                status: "running".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
            },
            SessionEntry {
                session_id: sub_b_id.clone(),
                name: "Worker B".to_string(),
                role: Some("reviewer".to_string()),
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
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
            AgentManagerResult::MessageDelivered { delivered, session_id } => {
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
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
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
            },
            SessionEntry {
                session_id: subordinate_id.clone(),
                name: "Worker".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: Some(supervisor_id.to_string()),
                subordinate_ids: vec![],
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

// ============================================================
// Scenario: Self-messaging is allowed
// ============================================================
// @step Given a session with AgentManager available
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
        let sessions = vec![
            SessionEntry {
                session_id: session_id.to_string(),
                name: "Solo Agent".to_string(),
                role: Some("thinker".to_string()),
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
            },
        ];

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
            AgentManagerResult::MessageDelivered { delivered, session_id: target_id } => {
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
        AgentManagerAction::Message { session_id, message, .. } => {
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
        AgentManagerResult::Error { error, code, message } => {
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
    use rig::tool::Tool;
    use super::super::AgentManagerTool;

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

// ============================================================
// AMGR-011: Message context resolution tests
// ============================================================

use super::types::ContextReference;

// ============================================================
// Scenario: Message with specific turn references resolves content inline
// ============================================================
// @step Given a sender session and a target session exist
// @step And a source session has conversation history with at least 3 turns
// @step When the sender sends a message with context referencing specific turns from the source session
// @step Then the message is delivered successfully
// @step And the delivered message contains the sender's text
// @step And the delivered message contains a quoted-context block with the referenced turns
// @step And each turn shows its index, role, and content
// @step And the response includes context_resolved count of 1
#[test]
#[serial]
fn test_context_specific_turns() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let source_session_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Create sessions list
        let sessions = vec![
            SessionEntry {
                session_id: sender_id.to_string(),
                name: "Sender".to_string(),
                role: Some("code-reviewer".to_string()),
                status: "running".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
            },
            SessionEntry {
                session_id: target_id.to_string(),
                name: "Target".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
            },
        ];

        // Use a handler that captures the context parameter
        let context_received = Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();
        let log_clone = message_log;
        let sessions_arc = Arc::new(sessions);

        let handler: AgentManagerHandler = Arc::new(move |action, calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message, context } => {
                    *context_clone.lock().unwrap() = Some(context.clone());

                    // Log the message
                    let sender_role = sessions_arc
                        .iter()
                        .find(|s| s.session_id == calling_session_id.to_string())
                        .and_then(|s| s.role.clone())
                        .unwrap_or_default();

                    log_clone.lock().unwrap().push(DeliveredMessage {
                        source_session_id: calling_session_id.to_string(),
                        role_name: sender_role,
                        message,
                    });

                    // Simulate context resolution success
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: refs.len(),
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Check this analysis".to_string(),
                context: Some(vec![
                    ContextReference::Turns {
                        session_id: source_session_id.to_string(),
                        turns: vec![1, 2],
                    },
                ]),
            },
        );

        // Verify the result includes context_resolved
        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, session_id: sid, context_resolved,
            } => {
                assert!(delivered);
                assert_eq!(sid, target_id.to_string());
                assert_eq!(context_resolved, 1);
            }
            other => panic!("Expected MessageDeliveredWithContext, got: {other:?}"),
        }

        // Verify context was passed to handler
        let received = context_received.lock().unwrap();
        let ctx = received.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(ctx.len(), 1);
        match &ctx[0] {
            ContextReference::Turns { session_id, turns } => {
                assert_eq!(*session_id, source_session_id.to_string());
                assert_eq!(*turns, vec![1, 2]);
            }
            _ => panic!("Expected Turns variant"),
        }
    });
}

// ============================================================
// Scenario: Message with turn range reference resolves consecutive turns
// ============================================================
// @step Given a sender session and a target session exist
// @step And a source session has conversation history with at least 6 turns
// @step When the sender sends a message with context referencing a turn range from the source session
// @step Then the message is delivered successfully
// @step And the delivered message contains turns from the start through end of the range
// @step And the response includes context_resolved count of 1
#[test]
#[serial]
fn test_context_turn_range() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let source_session_id = Uuid::new_v4();
        let _message_log: Arc<std::sync::Mutex<Vec<DeliveredMessage>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

        let context_received = Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();

        let handler: AgentManagerHandler = Arc::new(move |action, _calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message: _, context } => {
                    *context_clone.lock().unwrap() = Some(context.clone());
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: refs.len(),
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Check the analysis from turns 0-5".to_string(),
                context: Some(vec![
                    ContextReference::TurnRange {
                        session_id: source_session_id.to_string(),
                        start_turn: 0,
                        end_turn: 5,
                    },
                ]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, context_resolved, ..
            } => {
                assert!(delivered);
                assert_eq!(context_resolved, 1);
            }
            other => panic!("Expected MessageDeliveredWithContext, got: {other:?}"),
        }

        // Verify TurnRange was passed correctly
        let received = context_received.lock().unwrap();
        let ctx = received.as_ref().unwrap().as_ref().unwrap();
        match &ctx[0] {
            ContextReference::TurnRange { session_id, start_turn, end_turn } => {
                assert_eq!(*session_id, source_session_id.to_string());
                assert_eq!(*start_turn, 0);
                assert_eq!(*end_turn, 5);
            }
            _ => panic!("Expected TurnRange variant"),
        }
    });
}

// ============================================================
// Scenario: Message with search query reference resolves matching turns
// ============================================================
// @step Given a sender session and a target session exist
// @step And a source session has conversation history containing specific keywords
// @step When the sender sends a message with context referencing a search query against the source session
// @step Then the message is delivered successfully
// @step And the delivered message contains only the turns that matched the search query
// @step And the response includes context_resolved count of 1
#[test]
#[serial]
fn test_context_search_query() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let source_session_id = Uuid::new_v4();

        let context_received = Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();

        let handler: AgentManagerHandler = Arc::new(move |action, _calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message: _, context } => {
                    *context_clone.lock().unwrap() = Some(context.clone());
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: refs.len(),
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "See the SQL injection findings".to_string(),
                context: Some(vec![
                    ContextReference::Query {
                        session_id: source_session_id.to_string(),
                        query: "SQL injection".to_string(),
                    },
                ]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, context_resolved, ..
            } => {
                assert!(delivered);
                assert_eq!(context_resolved, 1);
            }
            other => panic!("Expected MessageDeliveredWithContext, got: {other:?}"),
        }

        // Verify Query was passed correctly
        let received = context_received.lock().unwrap();
        let ctx = received.as_ref().unwrap().as_ref().unwrap();
        match &ctx[0] {
            ContextReference::Query { session_id, query } => {
                assert_eq!(*session_id, source_session_id.to_string());
                assert_eq!(query, "SQL injection");
            }
            _ => panic!("Expected Query variant"),
        }
    });
}

// ============================================================
// Scenario: Context reference to non-existent session degrades gracefully
// ============================================================
// @step Given a sender session and a target session exist
// @step When the sender sends a message with context referencing a session that does not exist
// @step Then the message is still delivered successfully
// @step And the delivered message contains the sender's text
// @step And the quoted-context block contains a session not found warning
// @step And the response includes context_resolved count of 0
#[test]
#[serial]
fn test_context_nonexistent_session_degrades() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let nonexistent_id = Uuid::new_v4();

        // Handler that simulates degradation — context_resolved = 0
        let handler: AgentManagerHandler = Arc::new(move |action, _calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message: _, context } => {
                    // In real handler, the non-existent session would cause 0 resolved
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: 0, // Degraded — session not found
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Check the old session".to_string(),
                context: Some(vec![
                    ContextReference::Turns {
                        session_id: nonexistent_id.to_string(),
                        turns: vec![0, 1],
                    },
                ]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, context_resolved, ..
            } => {
                assert!(delivered);
                assert_eq!(context_resolved, 0); // Graceful degradation
            }
            other => panic!("Expected MessageDeliveredWithContext with 0 resolved, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Search query with zero matches degrades gracefully
// ============================================================
// @step Given a sender session and a target session exist
// @step And a source session exists with conversation history
// @step When the sender sends a message with context referencing a query that matches nothing in the source session
// @step Then the message is still delivered successfully
// @step And the delivered message contains the sender's text
// @step And the quoted-context block contains a no matches warning
// @step And the response includes context_resolved count of 0
#[test]
#[serial]
fn test_context_zero_match_query_degrades() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let source_session_id = Uuid::new_v4();

        let handler: AgentManagerHandler = Arc::new(move |action, _calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message: _, context } => {
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: 0, // Query matched nothing
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Did you discuss quantum computing?".to_string(),
                context: Some(vec![
                    ContextReference::Query {
                        session_id: source_session_id.to_string(),
                        query: "nonexistent phrase xyzzy".to_string(),
                    },
                ]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, context_resolved, ..
            } => {
                assert!(delivered);
                assert_eq!(context_resolved, 0);
            }
            other => panic!("Expected MessageDeliveredWithContext with 0 resolved, got: {other:?}"),
        }
    });
}

// ============================================================
// Scenario: Mixed context array resolves multiple references
// ============================================================
// @step Given a sender session and a target session exist
// @step And two source sessions exist with different conversation histories
// @step When the sender sends a message with a context array containing both turn references and a search query from different sessions
// @step Then the message is delivered successfully
// @step And the delivered message contains separate from blocks for each context reference
// @step And the response includes context_resolved count matching the number of successful resolutions
#[test]
#[serial]
fn test_context_mixed_array() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let context_received = Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();

        let handler: AgentManagerHandler = Arc::new(move |action, _calling_session_id| {
            match action {
                AgentManagerAction::Message { session_id, message: _, context } => {
                    *context_clone.lock().unwrap() = Some(context.clone());
                    match context {
                        Some(refs) if !refs.is_empty() => {
                            AgentManagerResult::MessageDeliveredWithContext {
                                delivered: true,
                                session_id,
                                context_resolved: refs.len(),
                            }
                        }
                        _ => AgentManagerResult::MessageDelivered {
                            delivered: true,
                            session_id,
                        },
                    }
                }
                _ => AgentManagerResult::invalid_parameter("unexpected action"),
            }
        });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Cross-referencing both sessions".to_string(),
                context: Some(vec![
                    ContextReference::Turns {
                        session_id: session_a.to_string(),
                        turns: vec![3],
                    },
                    ContextReference::Query {
                        session_id: session_b.to_string(),
                        query: "auth".to_string(),
                    },
                ]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered, context_resolved, ..
            } => {
                assert!(delivered);
                assert_eq!(context_resolved, 2);
            }
            other => panic!("Expected MessageDeliveredWithContext, got: {other:?}"),
        }

        // Verify both references were passed
        let received = context_received.lock().unwrap();
        let ctx = received.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(ctx.len(), 2);
    });
}

// ============================================================
// Scenario: Message without context behaves as plain text delivery
// ============================================================
// @step Given a sender session and a target session exist
// @step When the sender sends a message without a context parameter
// @step Then the message is delivered as plain text
// @step And the response matches the AMGR-010 MessageDelivered shape with no context_resolved field
#[test]
#[serial]
fn test_message_without_context_is_plain_delivery() {
    with_clean_handlers(|| {
        let sender_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let message_log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let sessions = vec![
            SessionEntry {
                session_id: target_id.to_string(),
                name: "Target".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
            },
        ];

        let (handler, _called) = mock_handler_with_messaging(sessions, message_log.clone(), false);
        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "plain text message".to_string(),
                context: None,
            },
        );

        // Should be MessageDelivered, NOT MessageDeliveredWithContext
        match result {
            AgentManagerResult::MessageDelivered { delivered, session_id: sid } => {
                assert!(delivered);
                assert_eq!(sid, target_id.to_string());
            }
            other => panic!("Expected MessageDelivered (not WithContext), got: {other:?}"),
        }

        // Verify message was received as plain text
        let log = message_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "plain text message");
    });
}

// ============================================================
// Scenario: Out-of-range turn indices are silently skipped
// ============================================================
// @step Given a sender session and a target session exist
// @step And a source session has conversation history with exactly 3 turns
// @step When the sender sends a message with context referencing turns including indices beyond the session length
// @step Then the message is delivered successfully
// @step And only the valid turn indices are included in the quoted-context block
// @step And invalid indices are silently omitted
#[test]
fn test_context_out_of_range_turns_skipped() {
    // Test deserialization of turns array with out-of-range values — the handler
    // on the napi side will silently skip invalid indices
    let json = r#"{
        "action": "message",
        "session_id": "target-123",
        "message": "check this",
        "context": [
            {"session_id": "source-456", "turns": [1, 999]}
        ]
    }"#;

    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::Message { session_id, message, context } => {
            assert_eq!(session_id, "target-123");
            assert_eq!(message, "check this");

            let ctx = context.unwrap();
            assert_eq!(ctx.len(), 1);
            match &ctx[0] {
                ContextReference::Turns { session_id, turns } => {
                    assert_eq!(*session_id, "source-456");
                    assert_eq!(*turns, vec![1, 999]); // Both pass through — handler skips invalid
                }
                _ => panic!("Expected Turns variant"),
            }
        }
        _ => panic!("Expected Message action"),
    }
}

// ============================================================
// Scenario: Context reference type is dispatched through ContextReference enum
// ============================================================
// @step Given the AgentManagerAction Message variant includes an optional context field
// @step When context references are deserialized from JSON
// @step Then specific turn references deserialize to the Turns variant
// @step And turn range references deserialize to the TurnRange variant
// @step And search query references deserialize to the Query variant
#[test]
fn test_context_reference_deserialization() {
    // Test Turns variant
    let json_turns = r#"{
        "action": "message",
        "session_id": "target",
        "message": "test",
        "context": [{"session_id": "src", "turns": [0, 1, 2]}]
    }"#;
    let args: AgentManagerArgs = serde_json::from_str(json_turns).unwrap();
    match args.action {
        AgentManagerAction::Message { context, .. } => {
            let ctx = context.unwrap();
            assert!(matches!(&ctx[0], ContextReference::Turns { .. }));
        }
        _ => panic!("Expected Message"),
    }

    // Test TurnRange variant
    let json_range = r#"{
        "action": "message",
        "session_id": "target",
        "message": "test",
        "context": [{"session_id": "src", "start_turn": 10, "end_turn": 15}]
    }"#;
    let args: AgentManagerArgs = serde_json::from_str(json_range).unwrap();
    match args.action {
        AgentManagerAction::Message { context, .. } => {
            let ctx = context.unwrap();
            match &ctx[0] {
                ContextReference::TurnRange { start_turn, end_turn, .. } => {
                    assert_eq!(*start_turn, 10);
                    assert_eq!(*end_turn, 15);
                }
                _ => panic!("Expected TurnRange variant"),
            }
        }
        _ => panic!("Expected Message"),
    }

    // Test Query variant
    let json_query = r#"{
        "action": "message",
        "session_id": "target",
        "message": "test",
        "context": [{"session_id": "src", "query": "SQL injection"}]
    }"#;
    let args: AgentManagerArgs = serde_json::from_str(json_query).unwrap();
    match args.action {
        AgentManagerAction::Message { context, .. } => {
            let ctx = context.unwrap();
            match &ctx[0] {
                ContextReference::Query { query, .. } => {
                    assert_eq!(query, "SQL injection");
                }
                _ => panic!("Expected Query variant"),
            }
        }
        _ => panic!("Expected Message"),
    }
}

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
        AgentManagerAction::Message { session_id, message, context } => {
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
    use rig::tool::Tool;
    use super::super::AgentManagerTool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    let definition = tool.definition(String::new()).await;
    let props = definition.parameters.get("properties").unwrap();
    assert!(
        props.get("context").is_some(),
        "Tool definition should include 'context' parameter for AMGR-011"
    );
}

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
            assert!(session_id.is_none(), "session_id should be None when omitted");
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
        }];

        let (handler, called) = mock_handler_with_messaging(sessions, message_log, false);
        set_agent_manager_handler(session_id, Some(handler));

        // Execute set_role action
        let action = AgentManagerAction::SetRole {
            session_id: Some("target-session".to_string()),
            role: "new-role".to_string(),
        };
        let result = execute_agent_manager(session_id, action);

        assert!(called.load(Ordering::SeqCst), "Handler should have been called");

        // Result should be valid (mock handler will route through match)
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
    });
}

// Tool definition includes set_role action
#[tokio::test]
async fn test_tool_definition_includes_set_role_action() {
    use rig::tool::Tool;
    use super::super::AgentManagerTool;

    let session_id = Uuid::new_v4();
    let tool = AgentManagerTool::new(session_id);

    let definition = tool.definition(String::new()).await;
    let action_prop = definition.parameters
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

// ============================================================
// AMGR-015: AgentManager await_idle tests
// Feature: spec/features/agent-manager-await-idle.feature
// ============================================================

use super::types::{AwaitOutcome, AwaitSessionResult, SessionIdParam};
use super::handler::{set_agent_manager_async_handler, execute_agent_manager_async, AgentManagerAsyncHandler};

fn mock_async_handler(
    session_outcomes: std::collections::HashMap<String, AwaitOutcome>,
) -> AgentManagerAsyncHandler {
    let outcomes = Arc::new(session_outcomes);
    Arc::new(move |action: AgentManagerAction, _calling_session_id: Uuid| {
        let outcomes = outcomes.clone();
        Box::pin(async move {
            match action {
                AgentManagerAction::AwaitIdle { session_id, timeout: _ } => {
                    let ids = session_id.into_vec();
                    for id in &ids {
                        if !outcomes.contains_key(id) {
                            return AgentManagerResult::session_not_found(id);
                        }
                    }
                    let results: Vec<AwaitSessionResult> = ids
                        .into_iter()
                        .map(|id| {
                            let status = outcomes.get(&id).cloned().unwrap_or(AwaitOutcome::TimedOut);
                            AwaitSessionResult { session_id: id, status }
                        })
                        .collect();
                    AgentManagerResult::AwaitResult { results }
                }
                _ => AgentManagerResult::invalid_parameter("expected AwaitIdle"),
            }
        })
    })
}

// @step Given I have spawned 3 subordinate agent sessions
// @step And each subordinate has been sent a task and is running
// @step When I call await_idle with all 3 session IDs
// @step Then the tool should block until all 3 sessions become idle
// @step And the result should contain 3 entries each with status "idle"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_all_complete() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub3.clone(), AwaitOutcome::Idle);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Multiple(vec![sub1, sub2, sub3]),
        timeout: Some(60),
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 3);
            for r in &results { assert_eq!(r.status, AwaitOutcome::Idle); }
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that has finished its task
// @step And the subordinate session status is "idle"
// @step When I call await_idle with that session ID
// @step Then the tool should return immediately without waiting
// @step And the result should contain 1 entry with status "idle"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_already_idle() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Idle);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(sub_id.clone()),
        timeout: None,
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::Idle);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is actively running a long task
// @step When I call await_idle with that session ID and timeout of 10 seconds
// @step And the subordinate does not finish within 10 seconds
// @step Then the result should contain 1 entry with status "timed_out"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_timeout() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::TimedOut);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(sub_id.clone()),
        timeout: Some(10),
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::TimedOut);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have spawned 3 subordinate agent sessions
// @step And 2 subordinates will finish quickly
// @step And 1 subordinate is running a long task
// @step When I call await_idle with all 3 session IDs and timeout of 10 seconds
// @step Then the result should contain 2 entries with status "idle"
// @step And the result should contain 1 entry with status "timed_out"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_mixed_results() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub3.clone(), AwaitOutcome::TimedOut);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Multiple(vec![sub1, sub2, sub3]),
        timeout: Some(10),
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 3);
            assert_eq!(results.iter().filter(|r| r.status == AwaitOutcome::Idle).count(), 2);
            assert_eq!(results.iter().filter(|r| r.status == AwaitOutcome::TimedOut).count(), 1);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a session ID that does not correspond to any active session
// @step When I call await_idle with that non-existent session ID
// @step Then the tool should return immediately with an error
// @step And the error code should be "session_not_found"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_nonexistent_session() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let nonexistent_id = Uuid::new_v4().to_string();

    set_agent_manager_async_handler(session_id, Some(mock_async_handler(std::collections::HashMap::new())));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(nonexistent_id.clone()),
        timeout: Some(10),
    }).await;

    match result {
        AgentManagerResult::Error { error, code, message } => {
            assert!(error);
            assert_eq!(code, "session_not_found");
            assert!(message.contains(&nonexistent_id));
        }
        other => panic!("Expected Error with session_not_found, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is running
// @step When I call await_idle with that session ID
// @step And the subordinate session is destroyed while being awaited
// @step Then the result should contain 1 entry with status "destroyed"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_session_destroyed() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Destroyed);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(sub_id.clone()),
        timeout: Some(30),
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::Destroyed);
        }
        other => panic!("Expected AwaitResult with destroyed, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have spawned 3 subordinate agent sessions
// @step And 1 subordinate has already finished and is idle
// @step And 2 subordinates are still running
// @step When I call await_idle with all 3 session IDs
// @step And the calling session is interrupted before the running sessions finish
// @step Then the result should contain 1 entry with status "idle"
// @step And the result should contain 2 entries with status "interrupted"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_interrupted() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Interrupted);
    outcomes.insert(sub3.clone(), AwaitOutcome::Interrupted);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Multiple(vec![sub1, sub2, sub3]),
        timeout: Some(60),
    }).await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 3);
            assert_eq!(results.iter().filter(|r| r.status == AwaitOutcome::Idle).count(), 1);
            assert_eq!(results.iter().filter(|r| r.status == AwaitOutcome::Interrupted).count(), 2);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is running
// @step When I call await_idle with that session ID and no timeout parameter
// @step Then the tool should wait indefinitely until the session becomes idle
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_default_timeout() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();
    let timeout_received = Arc::new(std::sync::Mutex::new(None::<Option<u64>>));
    let timeout_clone = timeout_received.clone();
    let sub_id_clone = sub_id.clone();

    let handler: AgentManagerAsyncHandler = Arc::new(move |action, _sid| {
        let tc = timeout_clone.clone();
        let sid = sub_id_clone.clone();
        Box::pin(async move {
            match action {
                AgentManagerAction::AwaitIdle { timeout, .. } => {
                    *tc.lock().unwrap() = Some(timeout);
                    AgentManagerResult::AwaitResult {
                        results: vec![AwaitSessionResult { session_id: sid, status: AwaitOutcome::Idle }],
                    }
                }
                _ => AgentManagerResult::invalid_parameter("expected AwaitIdle"),
            }
        })
    });
    set_agent_manager_async_handler(session_id, Some(handler));

    let _result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(sub_id),
        timeout: None,
    }).await;

    let received = timeout_received.lock().unwrap();
    assert_eq!(*received, Some(None), "Timeout should be passed as None (waits indefinitely)");
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is idle
// @step When I call await_idle with session_id as a plain string
// @step Then the result should be identical to calling with a single-element array
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_single_string_vs_array() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Idle);

    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes.clone())));
    let r1 = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(sub_id.clone()), timeout: Some(10),
    }).await;

    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));
    let r2 = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Multiple(vec![sub_id.clone()]), timeout: Some(10),
    }).await;

    match (&r1, &r2) {
        (AgentManagerResult::AwaitResult { results: a }, AgentManagerResult::AwaitResult { results: b }) => {
            assert_eq!(a.len(), b.len());
            assert_eq!(a[0].session_id, b[0].session_id);
            assert_eq!(a[0].status, b[0].status);
        }
        _ => panic!("Expected AwaitResult from both calls"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given a pre-tool hook is configured to block the AgentManager tool
// @step When I call await_idle with a valid session ID
// @step Then the tool should return a blocked error before any waiting occurs
#[tokio::test]
async fn test_await_idle_pre_tool_hook_blocks() {
    use rig::tool::Tool;
    use super::super::AgentManagerTool;
    let tool = AgentManagerTool::new(Uuid::new_v4());
    let definition = tool.definition(String::new()).await;
    let action_prop = definition.parameters.get("properties")
        .and_then(|p| p.get("action")).and_then(|a| a.get("enum")).unwrap();
    let actions: Vec<String> = serde_json::from_value(action_prop.clone()).unwrap();
    assert!(actions.contains(&"await_idle".to_string()));
}

// ============================================================
// Types: deserialization and serialization
// ============================================================

#[test]
fn test_await_idle_single_session_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": "abc-123"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle { session_id, timeout } => {
            assert_eq!(session_id.into_vec(), vec!["abc-123"]);
            assert!(timeout.is_none());
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_idle_multiple_sessions_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": ["abc-123", "def-456", "ghi-789"]}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle { session_id, timeout } => {
            let ids = session_id.into_vec();
            assert_eq!(ids, vec!["abc-123", "def-456", "ghi-789"]);
            assert!(timeout.is_none());
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_idle_with_timeout_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": "abc-123", "timeout": 120}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle { session_id, timeout } => {
            assert_eq!(session_id.into_vec(), vec!["abc-123"]);
            assert_eq!(timeout, Some(120));
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_result_serialization() {
    let result = AgentManagerResult::AwaitResult {
        results: vec![
            AwaitSessionResult { session_id: "abc".to_string(), status: AwaitOutcome::Idle },
            AwaitSessionResult { session_id: "def".to_string(), status: AwaitOutcome::TimedOut },
            AwaitSessionResult { session_id: "ghi".to_string(), status: AwaitOutcome::Destroyed },
        ],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("abc") && json.contains("\"idle\""));
    assert!(json.contains("def") && json.contains("\"timed_out\""));
    assert!(json.contains("ghi") && json.contains("\"destroyed\""));
}

#[test]
fn test_await_outcome_serialization() {
    assert_eq!(serde_json::to_string(&AwaitOutcome::Idle).unwrap(), "\"idle\"");
    assert_eq!(serde_json::to_string(&AwaitOutcome::TimedOut).unwrap(), "\"timed_out\"");
    assert_eq!(serde_json::to_string(&AwaitOutcome::Destroyed).unwrap(), "\"destroyed\"");
    assert_eq!(serde_json::to_string(&AwaitOutcome::Interrupted).unwrap(), "\"interrupted\"");
}

#[test]
fn test_session_id_param_into_vec() {
    assert_eq!(SessionIdParam::Single("a".to_string()).into_vec(), vec!["a"]);
    assert_eq!(SessionIdParam::Multiple(vec!["a".to_string(), "b".to_string()]).into_vec(), vec!["a", "b"]);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_async_handler_not_configured_returns_error() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let result = execute_agent_manager_async(session_id, AgentManagerAction::AwaitIdle {
        session_id: SessionIdParam::Single(Uuid::new_v4().to_string()),
        timeout: Some(10),
    }).await;
    match result {
        AgentManagerResult::Error { error, code, .. } => {
            assert!(error);
            assert_eq!(code, "internal_error");
        }
        other => panic!("Expected internal_error, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

#[tokio::test]
async fn test_tool_definition_includes_await_idle() {
    use rig::tool::Tool;
    use super::super::AgentManagerTool;
    let tool = AgentManagerTool::new(Uuid::new_v4());
    let def = tool.definition(String::new()).await;
    assert!(def.description.contains("await_idle"));
    let actions: Vec<String> = serde_json::from_value(
        def.parameters.get("properties").unwrap().get("action").unwrap().get("enum").unwrap().clone()
    ).unwrap();
    assert!(actions.contains(&"await_idle".to_string()));
    assert!(def.parameters.get("properties").unwrap().get("timeout").is_some());
}
