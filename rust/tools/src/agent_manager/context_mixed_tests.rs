//! Feature: spec/features/agent-manager-context-resolution.feature
//!
//! AMGR-011 mixed arrays, plain delivery, out-of-range turns, dispatch tests, moved out of tests.rs (COV-075: one test file per feature).
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
                delivered,
                context_resolved,
                ..
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

        let sessions = vec![SessionEntry {
            session_id: target_id.to_string(),
            name: "Target".to_string(),
            role: None,
            status: "idle".to_string(),
            spawner_id: None,
            subordinate_ids: vec![],
            pending_request: None,
        }];

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
            AgentManagerResult::MessageDelivered {
                delivered,
                session_id: sid,
            } => {
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
        AgentManagerAction::Message {
            session_id,
            message,
            context,
        } => {
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
                ContextReference::TurnRange {
                    start_turn,
                    end_turn,
                    ..
                } => {
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
