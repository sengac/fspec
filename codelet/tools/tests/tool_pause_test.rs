// Feature: spec/features/tool-pause-handler-mechanism.feature
// PAUSE-001: Interactive Tool Pause for Browser Debugging
//
// This test file tests the tool_pause module's handler mechanism.
// Uses with_pause_handler() for scoped handler registration.

use codelet_tools::{
    has_pause_handler, pause_for_user, with_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse, PauseState,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// =============================================================================
// Scenario: No handler returns Resumed
// =============================================================================

/// @step Given no pause handler is registered
/// @step When a tool calls pause_for_user
/// @step Then it should return Resumed immediately (no-op)
#[test]
fn test_no_handler_returns_resumed_immediately() {
    // Outside any scope - should return Resumed immediately
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page loaded".to_string(),
        details: None,
    });

    assert_eq!(
        response,
        PauseResponse::Resumed,
        "Should return Resumed when no handler is set"
    );
}

// =============================================================================
// Scenario: Handler is invoked with correct request
// =============================================================================

/// @step Given a pause handler is registered
/// @step When a tool calls pause_for_user
/// @step Then the handler should be invoked with the request
/// @step And the handler's response should be returned
#[test]
fn test_handler_is_invoked_with_request() {
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();

    let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
        handler_called_clone.store(true, Ordering::SeqCst);

        // Verify request contents
        assert_eq!(request.kind, PauseKind::Continue);
        assert_eq!(request.tool_name, "WebSearch");
        assert_eq!(request.message, "Page loaded");
        assert!(request.details.is_none());

        PauseResponse::Resumed
    });

    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        })
    });

    assert!(
        handler_called.load(Ordering::SeqCst),
        "Handler should have been called"
    );
    assert_eq!(response, PauseResponse::Resumed);
}

// =============================================================================
// Scenario: Handler returns different response types
// =============================================================================

/// @step Given a handler that returns Approved
/// @step When pause_for_user is called
/// @step Then it should return Approved
#[test]
fn test_handler_returns_approved() {
    let handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);

    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("rm -rf /".to_string()),
        })
    });

    assert_eq!(response, PauseResponse::Approved);
}

/// @step Given a handler that returns Denied
/// @step When pause_for_user is called
/// @step Then it should return Denied
#[test]
fn test_handler_returns_denied() {
    let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);

    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("rm -rf /".to_string()),
        })
    });

    assert_eq!(response, PauseResponse::Denied);
}

/// @step Given a handler that returns Interrupted
/// @step When pause_for_user is called
/// @step Then it should return Interrupted
#[test]
fn test_handler_returns_interrupted() {
    let handler: PauseHandler = Arc::new(|_| PauseResponse::Interrupted);

    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        })
    });

    assert_eq!(response, PauseResponse::Interrupted);
}

// =============================================================================
// Scenario: Handler can block and resume
// =============================================================================

/// @step Given a handler that blocks on a condvar
/// @step When pause_for_user is called
/// @step And a background thread signals the condvar
/// @step Then pause_for_user should unblock and return the response
#[test]
fn test_handler_can_block_and_resume() {
    // Simulate the session's condvar mechanism
    let response_signal: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let signal_clone = Arc::clone(&response_signal);

    // Handler that blocks until signaled (simulates BackgroundSession behavior)
    let handler: PauseHandler = Arc::new(move |_request: PauseRequest| {
        let (lock, cvar) = &*signal_clone;
        let mut response = lock.lock().unwrap();

        // Block until response is set
        while response.is_none() {
            response = cvar.wait(response).unwrap();
        }

        response.take().unwrap()
    });

    // Spawn thread that will signal resume after a delay
    let signal_for_thread = Arc::clone(&response_signal);
    let signaler_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        // Signal resume
        let (lock, cvar) = &*signal_for_thread;
        *lock.lock().unwrap() = Some(PauseResponse::Resumed);
        cvar.notify_one();
    });

    // Call pause_for_user within the handler scope
    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        })
    });

    signaler_thread.join().expect("Signaler thread should complete");

    assert_eq!(
        response,
        PauseResponse::Resumed,
        "Tool should have received Resumed response"
    );
}

/// @step Given a blocking handler is registered
/// @step When pause_for_user is called
/// @step And the user signals Interrupted
/// @step Then pause_for_user should return Interrupted
#[test]
fn test_handler_can_be_interrupted() {
    let response_signal: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let signal_clone = Arc::clone(&response_signal);

    let handler: PauseHandler = Arc::new(move |_request: PauseRequest| {
        let (lock, cvar) = &*signal_clone;
        let mut response = lock.lock().unwrap();
        while response.is_none() {
            response = cvar.wait(response).unwrap();
        }
        response.take().unwrap()
    });

    // Spawn thread that will signal interrupt after a delay
    let signal_for_thread = Arc::clone(&response_signal);
    let signaler_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        // Signal interrupt (simulates user pressing Esc)
        let (lock, cvar) = &*signal_for_thread;
        *lock.lock().unwrap() = Some(PauseResponse::Interrupted);
        cvar.notify_one();
    });

    let response = with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        })
    });

    signaler_thread.join().expect("Signaler thread should complete");

    assert_eq!(response, PauseResponse::Interrupted);
}

// =============================================================================
// Scenario: has_pause_handler correctly reports handler presence
// =============================================================================

#[test]
fn test_has_pause_handler() {
    // Outside scope - no handler
    assert!(
        !has_pause_handler(),
        "Should return false when outside scope"
    );

    // Inside scope with handler
    let handler: PauseHandler = Arc::new(|_| PauseResponse::Resumed);
    with_pause_handler(Some(handler), || {
        assert!(has_pause_handler(), "Should return true inside scope with handler");
    });

    // Inside scope with None handler
    with_pause_handler(None, || {
        assert!(!has_pause_handler(), "Should return false inside scope with None handler");
    });

    // After scope - no handler again
    assert!(
        !has_pause_handler(),
        "Should return false after scope ends"
    );
}

// =============================================================================
// Scenario: PauseState can be created from PauseRequest
// =============================================================================

#[test]
fn test_pause_state_from_request() {
    let request = PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Dangerous command".to_string(),
        details: Some("rm -rf /important".to_string()),
    };

    let state: PauseState = request.into();

    assert_eq!(state.kind, PauseKind::Confirm);
    assert_eq!(state.tool_name, "Bash");
    assert_eq!(state.message, "Dangerous command");
    assert_eq!(state.details, Some("rm -rf /important".to_string()));
}

// =============================================================================
// Scenario: Handler receives correct request details
// =============================================================================

#[test]
fn test_confirm_pause_with_details() {
    let captured_details = Arc::new(Mutex::new(None));
    let captured_clone = Arc::clone(&captured_details);

    let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
        *captured_clone.lock().unwrap() = request.details.clone();
        PauseResponse::Approved
    });

    with_pause_handler(Some(handler), || {
        pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("sudo rm -rf /*".to_string()),
        })
    });

    assert_eq!(
        *captured_details.lock().unwrap(),
        Some("sudo rm -rf /*".to_string())
    );
}

// =============================================================================
// Scenario: Multiple pause calls work correctly
// =============================================================================

#[test]
fn test_multiple_pause_calls() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count_clone = Arc::clone(&call_count);

    let handler: PauseHandler = Arc::new(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
        PauseResponse::Resumed
    });

    with_pause_handler(Some(handler), || {
        // Make multiple pause calls within the same scope
        for _ in 0..3 {
            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            });
            assert_eq!(response, PauseResponse::Resumed);
        }
    });

    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

// =============================================================================
// Scenario: Nested scopes work correctly
// =============================================================================

#[test]
fn test_nested_scopes() {
    let outer_handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
    let inner_handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);

    with_pause_handler(Some(outer_handler), || {
        // Outer scope should use outer handler
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "Outer".to_string(),
            details: None,
        });
        assert_eq!(response, PauseResponse::Approved);

        // Inner scope should use inner handler
        with_pause_handler(Some(inner_handler), || {
            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Inner".to_string(),
                details: None,
            });
            assert_eq!(response, PauseResponse::Denied);
        });

        // After inner scope, outer handler should be active again
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "Outer again".to_string(),
            details: None,
        });
        assert_eq!(response, PauseResponse::Approved);
    });
}
