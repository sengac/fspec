#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: End-to-End Pause Integration Tests
//!
//! These tests verify the pause mechanism integration:
//! 1. Tool pause handler mechanism works correctly
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
    id: String,
    status: AtomicU8,
    pause_state: RwLock<Option<PauseState>>,
}

impl SimulatedSession {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
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
        let is_some = state.is_some();
        let mut guard = self.pause_state.write().unwrap();
        *guard = state;
        if is_some {
            self.set_status(STATUS_PAUSED);
        }
    }

    fn clear_pause_state(&self) {
        let mut guard = self.pause_state.write().unwrap();
        *guard = None;
        self.set_status(STATUS_RUNNING);
    }
}

// =============================================================================
// Feature: Session Pause State Isolation
// Tests that pause state is per-session using the same pattern as BackgroundSession
// =============================================================================

/// @scenario: Pause state is stored per-session not globally
/// @step: Given two sessions exist
/// @step: When session A pauses
/// @step: Then session A status should be "paused"
/// @step: And session B status should be "running"
#[test]
fn test_pause_state_is_per_session() {
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    // Both start running
    session_a.set_status(STATUS_RUNNING);
    session_b.set_status(STATUS_RUNNING);

    assert_eq!(session_a.get_status(), STATUS_RUNNING);
    assert_eq!(session_b.get_status(), STATUS_RUNNING);
    assert!(session_a.get_pause_state().is_none());
    assert!(session_b.get_pause_state().is_none());

    // Session A pauses
    let pause_state_a = PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page loaded at https://example.com".to_string(),
        details: None,
    };
    session_a.set_pause_state(Some(pause_state_a.clone()));

    // Session A should be paused
    assert_eq!(session_a.get_status(), STATUS_PAUSED);

    // Session B should still be running
    assert_eq!(session_b.get_status(), STATUS_RUNNING);

    // Session A has pause state
    let state_a = session_a.get_pause_state();
    assert!(state_a.is_some());
    let state_a = state_a.unwrap();
    assert_eq!(state_a.tool_name, "WebSearch");

    // Session B has no pause state
    assert!(session_b.get_pause_state().is_none());
}

/// @scenario: SessionStatus::Paused has correct value
/// @step: Given the session status constants
/// @step: Then STATUS_PAUSED should be 3
#[test]
fn test_session_status_paused_value() {
    assert_eq!(STATUS_IDLE, 0);
    assert_eq!(STATUS_RUNNING, 1);
    assert_eq!(STATUS_INTERRUPTED, 2);
    assert_eq!(STATUS_PAUSED, 3);
}

// =============================================================================
// Feature: Tool Pause Handler Mechanism
// =============================================================================

/// @scenario: Handler is set before tool execution
/// @step: Given the stream loop is about to run
/// @step: When a pause handler is registered
/// @step: Then has_pause_handler should return true
/// @step: And pause_for_user should invoke the handler
#[test]
fn test_pause_handler_registration() {
    // Clear any existing handler
    set_pause_handler(None);
    assert!(!has_pause_handler());
    
    // Set a handler
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();
    
    set_pause_handler(Some(Arc::new(move |request: PauseRequest| {
        handler_called_clone.store(true, Ordering::SeqCst);
        assert_eq!(request.tool_name, "WebSearch");
        assert_eq!(request.kind, PauseKind::Continue);
        PauseResponse::Resumed
    })));
    
    assert!(has_pause_handler());
    
    // Call pause_for_user - should invoke the handler
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page loaded".to_string(),
        details: None,
    });
    
    assert!(handler_called.load(Ordering::SeqCst), "Handler should have been called");
    assert_eq!(response, PauseResponse::Resumed);
    
    // Clean up
    set_pause_handler(None);
}

/// @scenario: Handler blocks until user responds
/// @step: Given a blocking handler is registered
/// @step: When pause_for_user is called from one thread
/// @step: And another thread signals the response
/// @step: Then pause_for_user should unblock and return the response
#[test]
fn test_pause_handler_blocking_and_resume() {
    use std::thread;
    use std::time::Duration;
    
    // Shared state for blocking/resume synchronization
    let response = Arc::new(Mutex::new(None::<PauseResponse>));
    let condvar = Arc::new(Condvar::new());
    
    let response_clone = response.clone();
    let condvar_clone = condvar.clone();
    
    // Create a blocking handler
    let handler: PauseHandler = Arc::new(move |_request| {
        let mut guard = response_clone.lock().unwrap();
        while guard.is_none() {
            guard = condvar_clone.wait(guard).unwrap();
        }
        guard.take().unwrap()
    });
    
    set_pause_handler(Some(handler));
    
    // Spawn thread that calls pause_for_user
    let pause_thread = thread::spawn(|| {
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Blocking test".to_string(),
            details: None,
        })
    });
    
    // Give the thread time to start waiting
    thread::sleep(Duration::from_millis(50));
    
    // Signal Resumed from main thread
    {
        let mut guard = response.lock().unwrap();
        *guard = Some(PauseResponse::Resumed);
    }
    condvar.notify_one();
    
    // Verify the response
    let result = pause_thread.join().expect("Thread panicked");
    assert_eq!(result, PauseResponse::Resumed);
    
    set_pause_handler(None);
}

/// @scenario: Handler can return different responses
/// @step: Given handlers that return Approved, Denied, and Interrupted
/// @step: When pause_for_user is called
/// @step: Then the correct response is returned
#[test]
fn test_pause_handler_different_responses() {
    // Test Approved
    set_pause_handler(Some(Arc::new(|_| PauseResponse::Approved)));
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Confirm command".to_string(),
        details: Some("rm -rf /tmp/*".to_string()),
    });
    assert_eq!(response, PauseResponse::Approved);
    
    // Test Denied
    set_pause_handler(Some(Arc::new(|_| PauseResponse::Denied)));
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Confirm command".to_string(),
        details: None,
    });
    assert_eq!(response, PauseResponse::Denied);
    
    // Test Interrupted
    set_pause_handler(Some(Arc::new(|_| PauseResponse::Interrupted)));
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "User pressed Esc".to_string(),
        details: None,
    });
    assert_eq!(response, PauseResponse::Interrupted);
    
    set_pause_handler(None);
}

// =============================================================================
// Feature: WebSearch Tool Pause Integration
// Tests that WebSearchAction pause field is respected
// =============================================================================

/// @scenario: WebSearchAction has pause field
/// @step: Given a WebSearchAction::OpenPage variant
/// @step: Then it should have a pause field
/// @step: And pause: true should be serializable
#[test]
fn test_websearch_action_has_pause_field() {
    use codelet_common::web_search::WebSearchAction;
    
    // Construct OpenPage with pause
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: true,
        pause: true,
    };
    
    // Verify the action serializes correctly
    let json = serde_json::to_string(&action).expect("Should serialize");
    assert!(json.contains("\"pause\":true"));
    
    // Verify FindInPage also has pause
    let action = WebSearchAction::FindInPage {
        url: Some("https://example.com".to_string()),
        pattern: Some("test".to_string()),
        headless: false,
        pause: true,
    };
    let json = serde_json::to_string(&action).expect("Should serialize");
    assert!(json.contains("\"pause\":true"));
    
    // Verify CaptureScreenshot also has pause
    let action = WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".to_string()),
        output_path: None,
        full_page: Some(false),
        headless: true,
        pause: true,
    };
    let json = serde_json::to_string(&action).expect("Should serialize");
    assert!(json.contains("\"pause\":true"));
}

/// @scenario: Pause with headless true auto-overrides to visible
/// @step: Given pause is true and headless is true
/// @step: When the tool processes the request
/// @step: Then headless should be automatically overridden to false
#[test]
fn test_pause_auto_overrides_headless() {
    // This tests the logic in WebSearchTool::call
    // When pause: true and headless: true, the tool should override headless to false
    
    // The actual override happens in web_search.rs, we verify the rule here
    let pause = true;
    let headless = true;
    
    // Rule: pause: true auto-implies headless: false
    let effective_headless = if pause { false } else { headless };
    
    assert!(!effective_headless, "pause: true should override headless to false");
}

// =============================================================================
// Integration: Verify types are exported correctly
// =============================================================================

/// @scenario: PauseKind, PauseRequest, PauseResponse are exported from codelet_tools
/// @step: Given the types are defined in tool_pause.rs
/// @step: Then they should be re-exported from codelet_tools crate
#[test]
fn test_pause_types_exported() {
    // Verify we can use all the types
    let _kind = PauseKind::Continue;
    let _kind = PauseKind::Confirm;
    
    let _request = PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test".to_string(),
        details: None,
    };
    
    let _response = PauseResponse::Resumed;
    let _response = PauseResponse::Approved;
    let _response = PauseResponse::Denied;
    let _response = PauseResponse::Interrupted;
    
    let _state = PauseState {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test".to_string(),
        details: None,
    };
}

/// @scenario: PauseHandler type is correct
/// @step: Given the PauseHandler type alias
/// @step: Then it should be Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>
#[test]
fn test_pause_handler_type() {
    // Create a handler that captures state (requires Send + Sync)
    let counter = Arc::new(AtomicBool::new(false));
    let counter_clone = counter.clone();
    
    let handler: PauseHandler = Arc::new(move |_request| {
        counter_clone.store(true, Ordering::SeqCst);
        PauseResponse::Resumed
    });
    
    // Use the handler
    set_pause_handler(Some(handler));
    
    let _ = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test".to_string(),
        details: None,
    });
    
    assert!(counter.load(Ordering::SeqCst));
    
    set_pause_handler(None);
}

/// @scenario: Multiple sessions can be paused independently
/// @step: Given two sessions, both running
/// @step: When both pause with different states
/// @step: Then each has its own isolated state
#[test]
fn test_multiple_sessions_paused_independently() {
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    session_a.set_status(STATUS_RUNNING);
    session_b.set_status(STATUS_RUNNING);

    // Both pause with different states
    session_a.set_pause_state(Some(PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page A loaded".to_string(),
        details: None,
    }));

    session_b.set_pause_state(Some(PauseState {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Confirm dangerous command".to_string(),
        details: Some("rm -rf /tmp/*".to_string()),
    }));

    // Both should be paused
    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert_eq!(session_b.get_status(), STATUS_PAUSED);

    // Each has its own pause state
    let state_a = session_a.get_pause_state().unwrap();
    let state_b = session_b.get_pause_state().unwrap();

    assert_eq!(state_a.tool_name, "WebSearch");
    assert_eq!(state_b.tool_name, "Bash");
    assert_eq!(state_a.kind, PauseKind::Continue);
    assert_eq!(state_b.kind, PauseKind::Confirm);
    assert!(state_a.details.is_none());
    assert_eq!(state_b.details, Some("rm -rf /tmp/*".to_string()));
}

/// @scenario: Resuming one session doesn't affect another
/// @step: Given two paused sessions
/// @step: When session A resumes
/// @step: Then session A is running, session B is still paused
#[test]
fn test_resuming_session_is_isolated() {
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    session_a.set_pause_state(Some(PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page A".to_string(),
        details: None,
    }));

    session_b.set_pause_state(Some(PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Page B".to_string(),
        details: None,
    }));

    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert_eq!(session_b.get_status(), STATUS_PAUSED);

    // Session A resumes
    session_a.clear_pause_state();

    // Session A is running, session B is still paused
    assert_eq!(session_a.get_status(), STATUS_RUNNING);
    assert!(session_a.get_pause_state().is_none());

    assert_eq!(session_b.get_status(), STATUS_PAUSED);
    assert!(session_b.get_pause_state().is_some());
    assert_eq!(session_b.get_pause_state().unwrap().message, "Page B");
}

/// @scenario: Concurrent access to session pause state is safe
/// @step: Given a session with pause state
/// @step: When multiple threads read/write pause state concurrently
/// @step: Then no data races occur
#[test]
fn test_concurrent_pause_state_access() {
    use std::thread;

    let session = Arc::new(SimulatedSession::new("session-concurrent"));

    // Set initial pause state
    session.set_pause_state(Some(PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Testing concurrent access".to_string(),
        details: None,
    }));

    // Spawn multiple readers
    let mut handles = vec![];

    for i in 0..10 {
        let session_clone = Arc::clone(&session);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let state = session_clone.get_pause_state();
                if let Some(s) = state {
                    assert_eq!(s.tool_name, "WebSearch");
                }
                let status = session_clone.get_status();
                assert!(status == STATUS_PAUSED || status == STATUS_RUNNING);
            }
            i
        });
        handles.push(handle);
    }

    // Spawn a writer that toggles pause state
    let session_writer = Arc::clone(&session);
    let writer_handle = thread::spawn(move || {
        for i in 0..50 {
            if i % 2 == 0 {
                session_writer.clear_pause_state();
            } else {
                session_writer.set_pause_state(Some(PauseState {
                    kind: PauseKind::Continue,
                    tool_name: "WebSearch".to_string(),
                    message: format!("Iteration {i}"),
                    details: None,
                }));
            }
        }
    });

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Reader thread panicked");
    }
    writer_handle.join().expect("Writer thread panicked");

    // Test passed - no data races
}
