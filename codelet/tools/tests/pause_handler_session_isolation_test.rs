#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/pause-handler-session-isolation.feature
//!
//! Tests for BUG-127: PAUSE_HANDLER per-session isolation.
//!
//! These tests verify that pause handlers are keyed by session_id
//! so that concurrent sessions never route pause interactions to the wrong session.

use codelet_tools::tool_pause::{
    has_pause_handler, pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Scenario: Per-session handler isolation — pause dispatches only to the
//           registered session
// ============================================================================

#[test]
#[serial]
fn test_per_session_pause_handler_isolation() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a pause handler that returns Approved
    let handler_a: PauseHandler = Arc::new(|_| PauseResponse::Approved);
    set_pause_handler(session_a, Some(handler_a));

    // @step And session B has registered a pause handler that returns Denied
    let handler_b: PauseHandler = Arc::new(|_| PauseResponse::Denied);
    set_pause_handler(session_b, Some(handler_b));

    // @step When pause_for_user is called with session A's ID
    let response = pause_for_user(session_a, PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Test".to_string(),
        message: "test".to_string(),
        details: None,
    });

    // @step Then only session A's handler is invoked
    // @step And the response is Approved
    assert_eq!(response, PauseResponse::Approved);

    // Verify session B returns Denied (its own handler)
    let response_b = pause_for_user(session_b, PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Test".to_string(),
        message: "test".to_string(),
        details: None,
    });
    assert_eq!(response_b, PauseResponse::Denied);

    // Cleanup
    set_pause_handler(session_a, None);
    set_pause_handler(session_b, None);
}

// ============================================================================
// Scenario: Clearing one session's handler does not affect another session
// ============================================================================

#[test]
#[serial]
fn test_clearing_one_session_pause_handler_does_not_affect_another() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a pause handler
    let handler_a: PauseHandler = Arc::new(|_| PauseResponse::Approved);
    set_pause_handler(session_a, Some(handler_a));

    // @step And session B has registered a pause handler
    let handler_b: PauseHandler = Arc::new(|_| PauseResponse::Denied);
    set_pause_handler(session_b, Some(handler_b));

    // @step When session B's handler is cleared
    set_pause_handler(session_b, None);

    // @step And pause_for_user is called with session A's ID
    let response = pause_for_user(session_a, PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "test".to_string(),
        details: None,
    });

    // @step Then session A's handler is invoked normally
    assert_eq!(response, PauseResponse::Approved);

    // @step And pause_for_user with session B's ID returns Resumed
    let response_b = pause_for_user(session_b, PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "test".to_string(),
        details: None,
    });
    assert_eq!(response_b, PauseResponse::Resumed);

    // Cleanup
    set_pause_handler(session_a, None);
}

// ============================================================================
// Scenario: Pausing for an unregistered session returns Resumed without error
// ============================================================================

#[test]
#[serial]
fn test_pause_for_unregistered_session_returns_resumed() {
    // @step Given no pause handler is registered for session C
    let session_c = Uuid::new_v4();

    // @step When pause_for_user is called with session C's ID
    let response = pause_for_user(session_c, PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "test".to_string(),
        details: None,
    });

    // @step Then the response is Resumed
    assert_eq!(response, PauseResponse::Resumed);

    // @step And no error or panic occurs
    // (reaching here without panic is the assertion)
}

// ============================================================================
// Scenario: has_pause_handler checks only the specified session
// ============================================================================

#[test]
#[serial]
fn test_has_pause_handler_checks_only_specified_session() {
    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a pause handler
    let handler_a: PauseHandler = Arc::new(|_| PauseResponse::Resumed);
    set_pause_handler(session_a, Some(handler_a));

    // @step And session B has no registered pause handler
    // (deliberately do NOT register B)

    // @step When has_pause_handler is queried for session A
    // @step Then it returns true
    assert!(has_pause_handler(session_a));

    // @step When has_pause_handler is queried for session B
    // @step Then it returns false
    assert!(!has_pause_handler(session_b));

    // Cleanup
    set_pause_handler(session_a, None);
}
