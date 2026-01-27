#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: Stream Loop Pause Handler Integration Tests
//!
//! These tests verify that run_agent_stream correctly integrates with the
//! pause handler mechanism using global RwLock via set_pause_handler().
//!
//! Key behaviors tested:
//! 1. set_pause_handler() sets the global handler
//! 2. The handler captures session context for per-session isolation
//! 3. The handler correctly sets session pause state and waits for response

use codelet_tools::tool_pause::{
    has_pause_handler, pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse, PauseState,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};

// Session status constants
const STATUS_IDLE: u8 = 0;
const STATUS_RUNNING: u8 = 1;
const STATUS_PAUSED: u8 = 3;

/// Mock session for testing pause handler integration
struct MockSession {
    status: AtomicU8,
    pause_state: RwLock<Option<PauseState>>,
    pause_response: Arc<(Mutex<Option<PauseResponse>>, Condvar)>,
}

impl MockSession {
    fn new() -> Self {
        Self {
            status: AtomicU8::new(STATUS_IDLE),
            pause_state: RwLock::new(None),
            pause_response: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    fn get_status(&self) -> u8 {
        self.status.load(Ordering::Acquire)
    }

    fn set_status(&self, status: u8) {
        self.status.store(status, Ordering::Release);
    }

    fn set_pause_state(&self, state: Option<PauseState>) {
        *self.pause_state.write().unwrap() = state;
    }

    fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.read().unwrap().clone()
    }

    fn wait_for_pause_response(&self) -> PauseResponse {
        let (lock, cvar) = &*self.pause_response;
        let mut response = lock.lock().unwrap();
        while response.is_none() {
            response = cvar.wait(response).unwrap();
        }
        response.take().unwrap()
    }

    fn send_pause_response(&self, resp: PauseResponse) {
        let (lock, cvar) = &*self.pause_response;
        *lock.lock().unwrap() = Some(resp);
        cvar.notify_one();
    }
}

// Helper to ensure tests don't interfere with each other
fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
    set_pause_handler(None);
    let result = f();
    set_pause_handler(None);
    result
}

/// @scenario: Handler is invoked with request and response is returned
/// @step: Given a pause handler is set
/// @step: When a tool calls pause_for_user
/// @step: Then the handler is invoked with the request
/// @step: And the response is returned to the tool
#[test]
fn test_handler_invoked_with_request_returns_response() {
    with_clean_handler(|| {
        let handler_invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = handler_invoked.clone();

        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(request.kind, PauseKind::Continue);
            assert_eq!(request.tool_name, "WebSearch");
            PauseResponse::Resumed
        });

        set_pause_handler(Some(handler));
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        assert!(handler_invoked.load(Ordering::SeqCst), "Handler should be invoked");
        assert_eq!(response, PauseResponse::Resumed);
    });
}

/// @scenario: Handler sets pause state on session
/// @step: Given a handler that captures session context
/// @step: When pause_for_user is called
/// @step: Then the handler sets pause state on the session
#[test]
fn test_handler_sets_session_pause_state() {
    with_clean_handler(|| {
        let session = Arc::new(MockSession::new());
        let session_clone = session.clone();

        // Create handler that captures session context (like real implementation)
        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            // Convert request to state and set on session
            let state = PauseState {
                kind: request.kind,
                tool_name: request.tool_name,
                message: request.message,
                details: request.details,
            };
            session_clone.set_pause_state(Some(state));
            session_clone.set_status(STATUS_PAUSED);

            // In real impl, would block here; for test, return immediately
            PauseResponse::Resumed
        });

        set_pause_handler(Some(handler));

        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        // Verify pause state was set on session
        let state = session.get_pause_state();
        assert!(state.is_some(), "Pause state should be set on session");
        let state = state.unwrap();
        assert_eq!(state.tool_name, "WebSearch");
        assert_eq!(state.message, "Page loaded");
        assert_eq!(session.get_status(), STATUS_PAUSED);
    });
}

/// @scenario: Handler blocks until response received
/// @step: Given a handler that blocks on condvar
/// @step: When pause_for_user is called
/// @step: And a separate thread sends response
/// @step: Then the handler unblocks and returns response
#[test]
fn test_handler_blocks_until_response() {
    with_clean_handler(|| {
        let session = Arc::new(MockSession::new());
        let session_for_handler = session.clone();
        let session_for_signaler = session.clone();

        // Handler that blocks like real implementation
        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            let state = PauseState {
                kind: request.kind,
                tool_name: request.tool_name,
                message: request.message,
                details: request.details,
            };
            session_for_handler.set_pause_state(Some(state));
            session_for_handler.set_status(STATUS_PAUSED);
            
            // Block until response received
            let response = session_for_handler.wait_for_pause_response();
            
            session_for_handler.set_status(STATUS_RUNNING);
            response
        });

        // Spawn thread to send response after delay
        let signaler = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            session_for_signaler.send_pause_response(PauseResponse::Resumed);
        });

        set_pause_handler(Some(handler));

        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        signaler.join().unwrap();

        assert_eq!(response, PauseResponse::Resumed);
        assert_eq!(session.get_status(), STATUS_RUNNING);
    });
}

/// @scenario: No handler returns Resumed immediately
/// @step: Given no handler is set
/// @step: When pause_for_user is called
/// @step: Then it returns Resumed without blocking
#[test]
fn test_no_handler_returns_resumed() {
    with_clean_handler(|| {
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Resumed);
    });
}

/// @scenario: Handler can be cleared
/// @step: Given a handler is set
/// @step: When set_pause_handler(None) is called
/// @step: Then has_pause_handler returns false
#[test]
fn test_handler_can_be_cleared() {
    with_clean_handler(|| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Resumed);
        
        set_pause_handler(Some(handler));
        assert!(has_pause_handler());
        
        set_pause_handler(None);
        assert!(!has_pause_handler());
    });
}
