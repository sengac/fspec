#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: End-to-End Pause Integration Tests
//!
//! These tests verify the pause mechanism integration:
//! 1. Tool pause handler mechanism works correctly with global RwLock
//! 2. WebSearchAction has pause field
//! 3. Pause types are properly exported
//!
//! NOTE: These tests don't use BackgroundSession directly as it requires NAPI runtime.
//! Instead, we test the pattern used (AtomicU8 status + RwLock<Option<PauseState>>).

use codelet_tools::tool_pause::{
    has_pause_handler, pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse, PauseState,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};

// =============================================================================
// Session Status Simulation (mirrors SessionStatus in session_manager.rs)
// =============================================================================

const STATUS_IDLE: u8 = 0;
const STATUS_RUNNING: u8 = 1;
const STATUS_INTERRUPTED: u8 = 2;
const STATUS_PAUSED: u8 = 3;

/// Simulated BackgroundSession for testing the pause pattern
/// This mirrors the exact pattern that BackgroundSession should use
struct SimulatedSession {
    status: AtomicU8,
    pause_state: RwLock<Option<PauseState>>,
}

impl SimulatedSession {
    fn new() -> Self {
        Self {
            status: AtomicU8::new(STATUS_IDLE),
            pause_state: RwLock::new(None),
        }
    }

    fn get_status(&self) -> u8 {
        self.status.load(Ordering::Acquire)
    }

    fn set_status(&self, status: u8) {
        self.status.store(status, Ordering::Release);
    }

    fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.read().unwrap().clone()
    }

    fn set_pause_state(&self, state: Option<PauseState>) {
        *self.pause_state.write().unwrap() = state;
    }

    fn clear_pause_state(&self) {
        *self.pause_state.write().unwrap() = None;
    }
}

// Helper to ensure tests don't interfere with each other
fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
    set_pause_handler(None);
    let result = f();
    set_pause_handler(None);
    result
}

// =============================================================================
// Scenario: Pause state isolation per session
// =============================================================================

/// @scenario: Pause state is stored per-session not globally
/// @step: Given two sessions running concurrently
/// @step: When session A requests a pause
/// @step: Then session B should NOT see session A's pause state
#[test]
fn test_pause_state_is_per_session() {
    with_clean_handler(|| {
        let session_a = Arc::new(SimulatedSession::new());
        let session_b = Arc::new(SimulatedSession::new());

        // Simulate session A setting pause state (what the handler does)
        let pause_state_a = PauseState {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Session A pause".to_string(),
            details: None,
        };
        session_a.set_pause_state(Some(pause_state_a));
        session_a.set_status(STATUS_PAUSED);

        // Session B should have no pause state
        assert!(session_b.get_pause_state().is_none());
        assert_eq!(session_b.get_status(), STATUS_IDLE);

        // Session A should have its pause state
        let state = session_a.get_pause_state().unwrap();
        assert_eq!(state.tool_name, "WebSearch");
        assert_eq!(state.message, "Session A pause");
        assert_eq!(session_a.get_status(), STATUS_PAUSED);
    });
}

/// @scenario: Handler sets pause state on session
/// @step: Given a handler that captures session context
/// @step: When pause_for_user is called
/// @step: Then the handler sets pause state on the session
#[test]
fn test_handler_sets_session_state() {
    with_clean_handler(|| {
        let session = Arc::new(SimulatedSession::new());
        let session_clone = session.clone();

        // Handler that mimics BackgroundSession handler (without blocking)
        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            let state = PauseState {
                kind: request.kind,
                tool_name: request.tool_name,
                message: request.message,
                details: request.details,
            };
            session_clone.set_pause_state(Some(state));
            session_clone.set_status(STATUS_PAUSED);
            
            // In real impl, would block here; for test, immediate return
            PauseResponse::Resumed
        });

        set_pause_handler(Some(handler));

        // Call pause_for_user
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Resumed);

        // Verify pause state was set on session
        let state = session.get_pause_state().unwrap();
        assert_eq!(state.kind, PauseKind::Continue);
        assert_eq!(state.tool_name, "WebSearch");
        assert_eq!(state.message, "Page loaded");
        assert_eq!(session.get_status(), STATUS_PAUSED);
    });
}

/// @scenario: Handler is set with set_pause_handler
/// @step: When set_pause_handler is used with a handler
/// @step: Then has_pause_handler returns true
/// @step: And pause_for_user calls the handler
#[test]
fn test_handler_is_scoped() {
    with_clean_handler(|| {
        let handler_called = Arc::new(AtomicBool::new(false));
        let handler_called_clone = handler_called.clone();

        let handler: PauseHandler = Arc::new(move |_| {
            handler_called_clone.store(true, Ordering::SeqCst);
            PauseResponse::Resumed
        });

        // Before setting handler
        assert!(!has_pause_handler());

        // Set handler
        set_pause_handler(Some(handler));
        assert!(has_pause_handler());

        // Call pause_for_user
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });

        assert!(handler_called.load(Ordering::SeqCst));

        // Clear handler
        set_pause_handler(None);
        assert!(!has_pause_handler());
    });
}

/// @scenario: Full pause flow with blocking handler
/// @step: Given a handler that blocks on condvar
/// @step: When pause_for_user is called
/// @step: And another thread signals resume
/// @step: Then the tool receives the response
#[test]
fn test_full_pause_flow_with_blocking() {
    with_clean_handler(|| {
        let session = Arc::new(SimulatedSession::new());
        let response_signal: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));

        let session_for_handler = session.clone();
        let signal_for_handler = response_signal.clone();

        // Create handler that mimics BackgroundSession handler behavior
        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            // Set pause state
            let state = PauseState {
                kind: request.kind,
                tool_name: request.tool_name,
                message: request.message,
                details: request.details,
            };
            session_for_handler.set_pause_state(Some(state));
            session_for_handler.set_status(STATUS_PAUSED);

            // Block until signaled (like wait_for_pause_response)
            let (lock, cvar) = &*signal_for_handler;
            let mut response = lock.lock().unwrap();
            while response.is_none() {
                response = cvar.wait(response).unwrap();
            }
            let result = response.take().unwrap();

            // Clean up
            session_for_handler.clear_pause_state();
            session_for_handler.set_status(STATUS_RUNNING);

            result
        });

        // Spawn thread that simulates UI sending response
        let signal_for_ui = response_signal.clone();
        let ui_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Simulate UI calling session_pause_resume
            let (lock, cvar) = &*signal_for_ui;
            *lock.lock().unwrap() = Some(PauseResponse::Resumed);
            cvar.notify_one();
        });

        set_pause_handler(Some(handler));

        // Tool calls pause_for_user (will block)
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        ui_thread.join().unwrap();

        assert_eq!(response, PauseResponse::Resumed);
        assert!(session.get_pause_state().is_none()); // Cleared after resume
        assert_eq!(session.get_status(), STATUS_RUNNING);
    });
}

// =============================================================================
// Scenario: Handler returns different response types
// =============================================================================

#[test]
fn test_handler_returns_approved() {
    with_clean_handler(|| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
        set_pause_handler(Some(handler));

        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Approved);
    });
}

#[test]
fn test_handler_returns_denied() {
    with_clean_handler(|| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
        set_pause_handler(Some(handler));

        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Denied);
    });
}

#[test]
fn test_handler_returns_interrupted() {
    with_clean_handler(|| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Interrupted);
        set_pause_handler(Some(handler));

        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Interrupted);
    });
}

// =============================================================================
// Scenario: WebSearchAction has pause field
// =============================================================================

#[test]
fn test_web_search_action_has_pause_field() {
    use codelet_common::web_search::WebSearchAction;

    // Test OpenPage has pause
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: true,
        pause: true,
    };
    match action {
        WebSearchAction::OpenPage { pause, .. } => {
            assert!(pause, "OpenPage should have pause field set to true");
        }
        _ => panic!("Expected OpenPage variant"),
    }

    // Test CaptureScreenshot has pause
    let action = WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".to_string()),
        output_path: None,
        full_page: None,
        headless: true,
        pause: true,
    };
    match action {
        WebSearchAction::CaptureScreenshot { pause, .. } => {
            assert!(pause, "CaptureScreenshot should have pause field set to true");
        }
        _ => panic!("Expected CaptureScreenshot variant"),
    }

    // Test FindInPage has pause
    let action = WebSearchAction::FindInPage {
        url: Some("https://example.com".to_string()),
        pattern: Some("test".to_string()),
        headless: true,
        pause: true,
    };
    match action {
        WebSearchAction::FindInPage { pause, .. } => {
            assert!(pause, "FindInPage should have pause field set to true");
        }
        _ => panic!("Expected FindInPage variant"),
    }
}

// =============================================================================
// Scenario: Pause types are exportable
// =============================================================================

#[test]
fn test_pause_types_are_exported() {
    // Verify all pause types are exported from codelet_tools
    use codelet_tools::{
        PauseHandler, PauseKind, PauseRequest, PauseResponse, PauseState,
    };

    // Verify enums have expected variants
    assert_eq!(PauseKind::Continue, PauseKind::Continue);
    assert_eq!(PauseKind::Confirm, PauseKind::Confirm);

    assert_eq!(PauseResponse::Resumed, PauseResponse::Resumed);
    assert_eq!(PauseResponse::Approved, PauseResponse::Approved);
    assert_eq!(PauseResponse::Denied, PauseResponse::Denied);
    assert_eq!(PauseResponse::Interrupted, PauseResponse::Interrupted);

    // Verify structs can be constructed
    let _request = PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test".to_string(),
        details: None,
    };

    let _state = PauseState {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test".to_string(),
        details: None,
    };

    // Verify handler type compiles
    let _handler: PauseHandler = Arc::new(|_| PauseResponse::Resumed);
}

// =============================================================================
// Scenario: Full session handler like real implementation
// =============================================================================

/// This test verifies that the handler pattern matches what session_manager.rs does
#[test]
fn test_session_handler_pattern() {
    with_clean_handler(|| {
        let session = Arc::new(SimulatedSession::new());
        let response_condvar: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));

        let session_for_handler = session.clone();
        let condvar_for_handler = response_condvar.clone();

        // This handler matches the pattern in session_manager.rs agent_loop
        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            // Convert PauseRequest to PauseState and set on session
            let state = PauseState {
                kind: request.kind,
                tool_name: request.tool_name.clone(),
                message: request.message.clone(),
                details: request.details.clone(),
            };

            // Set pause state and status (order matters for UI)
            session_for_handler.set_pause_state(Some(state));
            session_for_handler.set_status(STATUS_PAUSED);

            // Block until TypeScript sends response
            let (lock, cvar) = &*condvar_for_handler;
            let mut response = lock.lock().unwrap();
            while response.is_none() {
                response = cvar.wait(response).unwrap();
            }
            let result = response.take().unwrap();

            // Restore status to Running after user responds
            session_for_handler.set_status(STATUS_RUNNING);

            result
        });

        // Set the handler
        set_pause_handler(Some(handler));

        // Spawn thread to send response (simulates TypeScript UI)
        let condvar_for_ui = response_condvar.clone();
        let ui_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let (lock, cvar) = &*condvar_for_ui;
            *lock.lock().unwrap() = Some(PauseResponse::Resumed);
            cvar.notify_one();
        });

        // Call pause_for_user within the scope
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded: https://example.com".to_string(),
            details: None,
        });

        ui_thread.join().unwrap();

        assert_eq!(response, PauseResponse::Resumed);
        assert_eq!(session.get_status(), STATUS_RUNNING);
    });
}
