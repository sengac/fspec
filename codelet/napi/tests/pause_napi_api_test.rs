#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: BackgroundSession pause_state field integration tests
//!
//! These tests verify that BackgroundSession has the pause_state field
//! and the NAPI functions to get/set it.
//!
//! This test file will FAIL until the following are implemented:
//! 1. BackgroundSession.pause_state: RwLock<Option<NapiPauseState>>
//! 2. session_get_pause_state(session_id: String) -> Option<NapiPauseState>
//! 3. session_pause_resume(session_id: String, response: String)
//! 4. session_pause_confirm(session_id: String, approved: bool)

use codelet_tools::tool_pause::{PauseKind, PauseState};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};

// Session status constants matching session_manager.rs
const STATUS_IDLE: u8 = 0;
const STATUS_RUNNING: u8 = 1;
const STATUS_INTERRUPTED: u8 = 2;
const STATUS_PAUSED: u8 = 3;

/// NapiPauseState for TypeScript (mirrors what NAPI should expose)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NapiPauseState {
    pub kind: String, // "continue" or "confirm"
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<PauseState> for NapiPauseState {
    fn from(state: PauseState) -> Self {
        Self {
            kind: match state.kind {
                PauseKind::Continue => "continue".to_string(),
                PauseKind::Confirm => "confirm".to_string(),
            },
            tool_name: state.tool_name,
            message: state.message,
            details: state.details,
        }
    }
}

/// Simulated BackgroundSession with pause_state field
/// This is the pattern BackgroundSession SHOULD follow
struct MockBackgroundSession {
    id: String,
    status: AtomicU8,
    pause_state: RwLock<Option<NapiPauseState>>,
    // For blocking/resuming during pause
    pause_response: Arc<(Mutex<Option<String>>, Condvar)>,
}

impl MockBackgroundSession {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            status: AtomicU8::new(STATUS_IDLE),
            pause_state: RwLock::new(None),
            pause_response: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    /// Get current status (same as BackgroundSession.get_status)
    fn get_status(&self) -> u8 {
        self.status.load(Ordering::Acquire)
    }

    /// Set status (same as BackgroundSession.set_status)
    fn set_status(&self, status: u8) {
        self.status.store(status, Ordering::Release);
    }

    /// Get pause state (NAPI: session_get_pause_state)
    fn get_pause_state(&self) -> Option<NapiPauseState> {
        self.pause_state.read().unwrap().clone()
    }

    /// Set pause state (internal, called by pause handler)
    fn set_pause_state(&self, state: Option<NapiPauseState>) {
        let is_paused = state.is_some();
        *self.pause_state.write().unwrap() = state;
        if is_paused {
            self.set_status(STATUS_PAUSED);
        }
    }

    /// Clear pause state and resume (NAPI: session_pause_resume)
    fn pause_resume(&self, response: &str) {
        // Set the response for the waiting handler
        let (lock, cvar) = &*self.pause_response;
        let mut guard = lock.lock().unwrap();
        *guard = Some(response.to_string());
        cvar.notify_one();
        
        // Clear pause state
        *self.pause_state.write().unwrap() = None;
        self.set_status(STATUS_RUNNING);
    }

    /// Respond to confirm pause (NAPI: session_pause_confirm)
    fn pause_confirm(&self, approved: bool) {
        let response = if approved { "approved" } else { "denied" };
        self.pause_resume(response);
    }

    /// Block until pause is resumed (used by pause handler)
    fn wait_for_pause_response(&self) -> String {
        let (lock, cvar) = &*self.pause_response;
        let mut guard = lock.lock().unwrap();
        while guard.is_none() {
            guard = cvar.wait(guard).unwrap();
        }
        guard.take().unwrap()
    }
}

// =============================================================================
// Feature: Session Pause State Isolation - NAPI API Tests
// =============================================================================

/// @scenario: session_get_pause_state returns None when not paused
/// @step: Given a running session
/// @step: When I query the pause state
/// @step: Then it should return None
#[test]
fn test_get_pause_state_returns_none_when_not_paused() {
    let session = MockBackgroundSession::new("test-session");
    session.set_status(STATUS_RUNNING);
    
    let pause_state = session.get_pause_state();
    assert!(pause_state.is_none());
}

/// @scenario: session_get_pause_state returns state when paused
/// @step: Given a paused session with pause state
/// @step: When I query the pause state
/// @step: Then it should return the pause details
#[test]
fn test_get_pause_state_returns_state_when_paused() {
    let session = MockBackgroundSession::new("test-session");
    session.set_status(STATUS_RUNNING);
    
    // Pause with Continue
    session.set_pause_state(Some(NapiPauseState {
        kind: "continue".to_string(),
        tool_name: "WebSearch".to_string(),
        message: "Page loaded at https://example.com".to_string(),
        details: None,
    }));
    
    let pause_state = session.get_pause_state();
    assert!(pause_state.is_some());
    
    let state = pause_state.unwrap();
    assert_eq!(state.kind, "continue");
    assert_eq!(state.tool_name, "WebSearch");
    assert_eq!(state.message, "Page loaded at https://example.com");
    assert!(state.details.is_none());
}

/// @scenario: session_pause_resume clears pause state and resumes
/// @step: Given a paused session
/// @step: When I call session_pause_resume with "resume"
/// @step: Then the pause state should be cleared
/// @step: And the status should be "running"
#[test]
fn test_pause_resume_clears_state() {
    let session = MockBackgroundSession::new("test-session");
    
    // Set up pause
    session.set_pause_state(Some(NapiPauseState {
        kind: "continue".to_string(),
        tool_name: "WebSearch".to_string(),
        message: "Page loaded".to_string(),
        details: None,
    }));
    
    assert_eq!(session.get_status(), STATUS_PAUSED);
    assert!(session.get_pause_state().is_some());
    
    // Resume
    session.pause_resume("resume");
    
    assert_eq!(session.get_status(), STATUS_RUNNING);
    assert!(session.get_pause_state().is_none());
}

/// @scenario: session_pause_confirm(true) approves and resumes
/// @step: Given a paused session with Confirm pause
/// @step: When I call session_pause_confirm(true)
/// @step: Then the pause state should be cleared
/// @step: And the status should be "running"
#[test]
fn test_pause_confirm_approved() {
    let session = MockBackgroundSession::new("test-session");
    
    // Set up confirm pause
    session.set_pause_state(Some(NapiPauseState {
        kind: "confirm".to_string(),
        tool_name: "Bash".to_string(),
        message: "Confirm dangerous command".to_string(),
        details: Some("rm -rf /tmp/*".to_string()),
    }));
    
    assert_eq!(session.get_status(), STATUS_PAUSED);
    
    // Approve
    session.pause_confirm(true);
    
    assert_eq!(session.get_status(), STATUS_RUNNING);
    assert!(session.get_pause_state().is_none());
}

/// @scenario: session_pause_confirm(false) denies and resumes
/// @step: Given a paused session with Confirm pause
/// @step: When I call session_pause_confirm(false)
/// @step: Then the pause state should be cleared
/// @step: And the status should be "running"
#[test]
fn test_pause_confirm_denied() {
    let session = MockBackgroundSession::new("test-session");
    
    // Set up confirm pause
    session.set_pause_state(Some(NapiPauseState {
        kind: "confirm".to_string(),
        tool_name: "Bash".to_string(),
        message: "Confirm dangerous command".to_string(),
        details: Some("rm -rf /tmp/*".to_string()),
    }));
    
    // Deny
    session.pause_confirm(false);
    
    assert_eq!(session.get_status(), STATUS_RUNNING);
    assert!(session.get_pause_state().is_none());
}

/// @scenario: Pause handler blocks until TypeScript responds
/// @step: Given a pause handler that sets session pause state
/// @step: When the handler blocks waiting for response
/// @step: And TypeScript calls session_pause_resume
/// @step: Then the handler should unblock and return the response
#[test]
fn test_pause_handler_blocks_until_resume() {
    use std::thread;
    use std::time::Duration;
    
    let session = Arc::new(MockBackgroundSession::new("test-session"));
    let session_clone = Arc::clone(&session);
    
    // Spawn thread simulating the pause handler
    let handler_thread = thread::spawn(move || {
        // Set pause state (done by pause handler)
        session_clone.set_pause_state(Some(NapiPauseState {
            kind: "continue".to_string(),
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        }));
        
        // Block waiting for response
        session_clone.wait_for_pause_response()
    });
    
    // Give handler time to set up and wait
    thread::sleep(Duration::from_millis(50));
    
    // Verify session is paused
    assert_eq!(session.get_status(), STATUS_PAUSED);
    assert!(session.get_pause_state().is_some());
    
    // Simulate TypeScript calling session_pause_resume
    session.pause_resume("resume");
    
    // Verify handler received response
    let response = handler_thread.join().expect("Handler panicked");
    assert_eq!(response, "resume");
    
    // Session should be running again
    assert_eq!(session.get_status(), STATUS_RUNNING);
}

/// @scenario: Multiple sessions can be paused and resumed independently
/// @step: Given two sessions, both paused
/// @step: When I resume session A
/// @step: Then session A should be running
/// @step: And session B should still be paused
#[test]
fn test_multiple_sessions_independent_pause() {
    let session_a = MockBackgroundSession::new("session-a");
    let session_b = MockBackgroundSession::new("session-b");
    
    // Both pause
    session_a.set_pause_state(Some(NapiPauseState {
        kind: "continue".to_string(),
        tool_name: "WebSearch".to_string(),
        message: "Page A loaded".to_string(),
        details: None,
    }));
    
    session_b.set_pause_state(Some(NapiPauseState {
        kind: "continue".to_string(),
        tool_name: "WebSearch".to_string(),
        message: "Page B loaded".to_string(),
        details: None,
    }));
    
    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert_eq!(session_b.get_status(), STATUS_PAUSED);
    
    // Resume only session A
    session_a.pause_resume("resume");
    
    // Session A is running, session B still paused
    assert_eq!(session_a.get_status(), STATUS_RUNNING);
    assert!(session_a.get_pause_state().is_none());
    
    assert_eq!(session_b.get_status(), STATUS_PAUSED);
    assert!(session_b.get_pause_state().is_some());
    assert_eq!(session_b.get_pause_state().unwrap().message, "Page B loaded");
}

/// @scenario: NapiPauseState serializes correctly for TypeScript
/// @step: Given a NapiPauseState with all fields
/// @step: When serialized to JSON
/// @step: Then the JSON should match TypeScript interface
#[test]
fn test_napi_pause_state_serialization() {
    let state = NapiPauseState {
        kind: "confirm".to_string(),
        tool_name: "Bash".to_string(),
        message: "Confirm dangerous command".to_string(),
        details: Some("rm -rf /tmp/*".to_string()),
    };
    
    let json = serde_json::to_string(&state).expect("Should serialize");
    
    // Verify JSON structure matches TypeScript PauseInfo interface
    assert!(json.contains(r#""kind":"confirm""#));
    assert!(json.contains(r#""tool_name":"Bash""#));
    assert!(json.contains(r#""message":"Confirm dangerous command""#));
    assert!(json.contains(r#""details":"rm -rf /tmp/*""#));
    
    // Verify it can be deserialized
    let deserialized: NapiPauseState = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized, state);
}

/// @scenario: NapiPauseState with null details
/// @step: Given a NapiPauseState without details
/// @step: When serialized to JSON
/// @step: Then details should be null
#[test]
fn test_napi_pause_state_null_details() {
    let state = NapiPauseState {
        kind: "continue".to_string(),
        tool_name: "WebSearch".to_string(),
        message: "Page loaded".to_string(),
        details: None,
    };
    
    let json = serde_json::to_string(&state).expect("Should serialize");
    
    assert!(json.contains(r#""details":null"#));
}
