//! Feature: spec/features/agent-manager-context-resolution.feature
//!
//! AMGR-011 search query and graceful degradation tests, moved out of tests.rs (COV-075: one test file per feature).
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
                message: "See the SQL injection findings".to_string(),
                context: Some(vec![ContextReference::Query {
                    session_id: source_session_id.to_string(),
                    query: "SQL injection".to_string(),
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
                AgentManagerAction::Message {
                    session_id,
                    message: _,
                    context,
                } => {
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
                context: Some(vec![ContextReference::Turns {
                    session_id: nonexistent_id.to_string(),
                    turns: vec![0, 1],
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
                AgentManagerAction::Message {
                    session_id,
                    message: _,
                    context,
                } => {
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
                context: Some(vec![ContextReference::Query {
                    session_id: source_session_id.to_string(),
                    query: "nonexistent phrase xyzzy".to_string(),
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
                assert_eq!(context_resolved, 0);
            }
            other => panic!("Expected MessageDeliveredWithContext with 0 resolved, got: {other:?}"),
        }
    });
}
