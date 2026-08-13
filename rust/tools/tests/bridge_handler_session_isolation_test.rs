#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/bridge-handler-session-isolation.feature
//!
//! Tests for BUG-128: BRIDGE_HANDLER per-session isolation.
//!
//! These tests verify that bridge handlers are keyed by session_id
//! so that concurrent sessions never route bridge commands to the wrong handler.

use codelet_tools::bridge::{BridgeAction, BridgeResult};
use codelet_tools::bridge_handler::{
    execute_bridge_command, has_bridge_handler_for_session, remove_bridge_session_context,
    set_bridge_handler, set_bridge_session_context, BridgeHandler, BridgeRequest,
};
use codelet_tools::bridge_relay::InputInjector;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Scenario: Per-session handler isolation — execute dispatches only to the
//           registered session
// ============================================================================

#[test]
#[serial]
fn test_per_session_bridge_handler_isolation() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a bridge handler returning a success result
    let handler_a: BridgeHandler = Arc::new(|_req| BridgeResult {
        success: true,
        message: "handler_a".to_string(),
        connections: None,
    });
    set_bridge_handler(session_a, Some(handler_a));

    // @step And session B has registered a bridge handler returning a different result
    let handler_b: BridgeHandler = Arc::new(|_req| BridgeResult {
        success: true,
        message: "handler_b".to_string(),
        connections: None,
    });
    set_bridge_handler(session_b, Some(handler_b));

    // @step When execute_bridge_command is called with session A's ID
    let result = execute_bridge_command(BridgeRequest {
        session_id: session_a,
        action: BridgeAction::List,
    });

    // @step Then only session A's handler is invoked
    // @step And the result matches session A's handler response
    assert!(result.success);
    assert_eq!(result.message, "handler_a");

    // Verify B gets its own handler
    let result_b = execute_bridge_command(BridgeRequest {
        session_id: session_b,
        action: BridgeAction::List,
    });
    assert_eq!(result_b.message, "handler_b");

    // Cleanup
    set_bridge_handler(session_a, None);
    set_bridge_handler(session_b, None);
}

// ============================================================================
// Scenario: Clearing one session's handler does not affect another session
// ============================================================================

#[test]
#[serial]
fn test_clearing_one_session_bridge_handler_does_not_affect_another() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a bridge handler
    let handler_a: BridgeHandler = Arc::new(|_req| BridgeResult {
        success: true,
        message: "from_a".to_string(),
        connections: None,
    });
    set_bridge_handler(session_a, Some(handler_a));

    // @step And session B has registered a bridge handler
    let handler_b: BridgeHandler = Arc::new(|_req| BridgeResult {
        success: true,
        message: "from_b".to_string(),
        connections: None,
    });
    set_bridge_handler(session_b, Some(handler_b));

    // @step When session B's handler is cleared
    set_bridge_handler(session_b, None);

    // @step And execute_bridge_command is called with session A's ID
    let result = execute_bridge_command(BridgeRequest {
        session_id: session_a,
        action: BridgeAction::List,
    });

    // @step Then session A's handler is invoked normally
    assert!(result.success);
    assert_eq!(result.message, "from_a");

    // @step And execute_bridge_command with session B's ID returns not-configured error
    let result_b = execute_bridge_command(BridgeRequest {
        session_id: session_b,
        action: BridgeAction::List,
    });
    assert!(!result_b.success);
    assert!(result_b.message.contains("not configured"));

    // Cleanup
    set_bridge_handler(session_a, None);
}

// ============================================================================
// Scenario: execute_bridge_command for an unregistered session returns error
// ============================================================================

#[test]
#[serial]
fn test_execute_bridge_command_unregistered_session_returns_error() {
    // @step Given no bridge handler is registered for session C
    let session_c = Uuid::new_v4();

    // @step When execute_bridge_command is called with session C's ID
    let result = execute_bridge_command(BridgeRequest {
        session_id: session_c,
        action: BridgeAction::List,
    });

    // @step Then the result indicates handler not configured
    assert!(!result.success);
    assert!(result.message.contains("not configured"));

    // @step And no error or panic occurs
    // (reaching here is the assertion)
}

// ============================================================================
// Scenario: has_bridge_handler_for_session checks per-session handler and context
// ============================================================================

#[test]
#[serial]
fn test_has_bridge_handler_for_session_checks_per_session() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has a registered bridge handler and session context
    let handler_a: BridgeHandler = Arc::new(|_req| BridgeResult {
        success: true,
        message: String::new(),
        connections: None,
    });
    set_bridge_handler(session_a, Some(handler_a));

    let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
    let broadcast_factory: codelet_tools::bridge_handler::BroadcastReceiverFactory =
        Arc::new(move || tx.subscribe());
    let input_injector: InputInjector = Arc::new(|_| {});
    set_bridge_session_context(session_a, broadcast_factory, input_injector, None, None);

    // @step And session B has neither handler nor context

    // @step When has_bridge_handler_for_session is queried for session A
    // @step Then it returns true
    assert!(has_bridge_handler_for_session(session_a));

    // @step When has_bridge_handler_for_session is queried for session B
    // @step Then it returns false
    assert!(!has_bridge_handler_for_session(session_b));

    // Cleanup
    set_bridge_handler(session_a, None);
    remove_bridge_session_context(session_a);
}
