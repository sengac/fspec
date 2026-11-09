//! Feature: spec/features/agent-manager-context-resolution.feature
//!
//! AMGR-011 turn references tests, moved out of tests.rs (COV-075: one test file per feature).
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
                pending_request: None,
            },
            SessionEntry {
                session_id: target_id.to_string(),
                name: "Target".to_string(),
                role: None,
                status: "idle".to_string(),
                spawner_id: None,
                subordinate_ids: vec![],
                pending_request: None,
            },
        ];

        // Use a handler that captures the context parameter
        let context_received =
            Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();
        let log_clone = message_log;
        let sessions_arc = Arc::new(sessions);

        let handler: AgentManagerHandler = Arc::new(move |action, calling_session_id| {
            match action {
                AgentManagerAction::Message {
                    session_id,
                    message,
                    context,
                } => {
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
                context: Some(vec![ContextReference::Turns {
                    session_id: source_session_id.to_string(),
                    turns: vec![1, 2],
                }]),
            },
        );

        // Verify the result includes context_resolved
        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered,
                session_id: sid,
                context_resolved,
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
        let _message_log: Arc<std::sync::Mutex<Vec<DeliveredMessage>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        let context_received =
            Arc::new(std::sync::Mutex::new(None::<Option<Vec<ContextReference>>>));
        let context_clone = context_received.clone();

        let handler: AgentManagerHandler =
            Arc::new(move |action, _calling_session_id| match action {
                AgentManagerAction::Message {
                    session_id,
                    message: _,
                    context,
                } => {
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
            });

        set_agent_manager_handler(sender_id, Some(handler));

        let result = execute_agent_manager(
            sender_id,
            AgentManagerAction::Message {
                session_id: target_id.to_string(),
                message: "Check the analysis from turns 0-5".to_string(),
                context: Some(vec![ContextReference::TurnRange {
                    session_id: source_session_id.to_string(),
                    start_turn: 0,
                    end_turn: 5,
                }]),
            },
        );

        match result {
            AgentManagerResult::MessageDeliveredWithContext {
                delivered,
                context_resolved,
                ..
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
            ContextReference::TurnRange {
                session_id,
                start_turn,
                end_turn,
            } => {
                assert_eq!(*session_id, source_session_id.to_string());
                assert_eq!(*start_turn, 0);
                assert_eq!(*end_turn, 5);
            }
            _ => panic!("Expected TurnRange variant"),
        }
    });
}
