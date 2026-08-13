#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/bash-abort-session-isolation.feature
//!
//! Tests for BUG-129: BASH_ABORT_FLAG per-session isolation.
//!
//! These tests verify that bash abort flags are keyed by session_id
//! so that pressing ESC in one session doesn't abort bash in other sessions.

use codelet_tools::bash::{clear_bash_abort, is_bash_abort_requested, request_bash_abort};
use std::sync::Mutex;
use uuid::Uuid;

/// Test lock — abort flag tests must be sequential since they share global state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

// ============================================================================
// Scenario: Per-session abort isolation — abort affects only the targeted session
// ============================================================================

#[test]
fn test_per_session_abort_isolation() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A and session B both have bash abort flags registered
    clear_bash_abort(session_a);
    clear_bash_abort(session_b);

    // @step When abort is requested for session A
    request_bash_abort(session_a);

    // @step Then session A's abort flag is true
    assert!(is_bash_abort_requested(session_a));

    // @step And session B's abort flag is false
    assert!(!is_bash_abort_requested(session_b));

    // Cleanup
    clear_bash_abort(session_a);
}

// ============================================================================
// Scenario: Clearing abort for one session does not affect another session
// ============================================================================

#[test]
fn test_clearing_abort_does_not_affect_other_session() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has abort requested
    clear_bash_abort(session_a);
    request_bash_abort(session_a);

    // @step And session B has abort requested
    clear_bash_abort(session_b);
    request_bash_abort(session_b);

    // @step When abort is cleared for session A
    clear_bash_abort(session_a);

    // @step Then session A's abort flag is false
    assert!(!is_bash_abort_requested(session_a));

    // @step And session B's abort flag is still true
    assert!(is_bash_abort_requested(session_b));

    // Cleanup
    clear_bash_abort(session_b);
}

// ============================================================================
// Scenario: Checking abort for an unknown session returns false without error
// ============================================================================

#[test]
fn test_abort_unknown_session_returns_false() {
    let _guard = TEST_LOCK.lock().unwrap();

    // @step Given no abort flag is registered for session C
    let session_c = Uuid::new_v4();

    // @step When abort status is checked for session C
    let result = is_bash_abort_requested(session_c);

    // @step Then the result is false
    assert!(!result);

    // @step And no error or panic occurs
    // (reaching here is the assertion)
}
