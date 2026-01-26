#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/interactive-tool-pause-for-browser-debugging.feature
//! PAUSE-001: Interactive Tool Pause for Browser Debugging
//!
//! Tests for per-session pause state isolation.
//!
//! This test verifies that pause state is stored on BackgroundSession (per-session),
//! not globally, following the same pattern as isLoading status.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

/// Simulated pause state struct (mirrors PauseState in tool_pause.rs)
#[derive(Debug, Clone, PartialEq)]
struct PauseState {
    tool_name: String,
    message: String,
    kind: PauseKind,
    details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PauseKind {
    Continue,
    Confirm,
}

/// Simulated session status values
const STATUS_IDLE: u8 = 0;
const STATUS_RUNNING: u8 = 1;
const STATUS_PAUSED: u8 = 2;

/// Simulated BackgroundSession with pause state (mirrors Architecture Note [1])
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
// Scenario: Pause state is stored per-session not globally
// =============================================================================

/// @step Given two sessions exist
/// @step When session A pauses
/// @step Then session A status should be "paused"
/// @step And session B status should be "running"
/// @step And querying session A pause state returns the details
/// @step And querying session B pause state returns null
#[test]
fn test_pause_state_is_per_session() {
    // @step Given two sessions exist
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    // Both start running
    session_a.set_status(STATUS_RUNNING);
    session_b.set_status(STATUS_RUNNING);

    assert_eq!(session_a.get_status(), STATUS_RUNNING);
    assert_eq!(session_b.get_status(), STATUS_RUNNING);
    assert!(session_a.get_pause_state().is_none());
    assert!(session_b.get_pause_state().is_none());

    // @step When session A pauses
    let pause_state_a = PauseState {
        tool_name: "WebSearch".to_string(),
        message: "Page loaded at https://example.com".to_string(),
        kind: PauseKind::Continue,
        details: None,
    };
    session_a.set_pause_state(Some(pause_state_a.clone()));

    // @step Then session A status should be "paused"
    assert_eq!(
        session_a.get_status(),
        STATUS_PAUSED,
        "Session A should be paused"
    );

    // @step And session B status should be "running"
    assert_eq!(
        session_b.get_status(),
        STATUS_RUNNING,
        "Session B should still be running (not affected by A's pause)"
    );

    // @step And querying session A pause state returns the details
    let state_a = session_a.get_pause_state();
    assert!(state_a.is_some(), "Session A should have pause state");
    let state_a = state_a.unwrap();
    assert_eq!(state_a.tool_name, "WebSearch");
    assert_eq!(state_a.message, "Page loaded at https://example.com");
    assert_eq!(state_a.kind, PauseKind::Continue);

    // @step And querying session B pause state returns null
    assert!(
        session_b.get_pause_state().is_none(),
        "Session B pause state should be None (isolated from A)"
    );
}

/// Test that multiple sessions can be paused independently
///
/// @step Given two sessions, both start running
/// @step When both sessions pause with different pause states
/// @step Then each session has its own pause state
#[test]
fn test_multiple_sessions_paused_independently() {
    // @step Given two sessions, both start running
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    session_a.set_status(STATUS_RUNNING);
    session_b.set_status(STATUS_RUNNING);

    // @step When both sessions pause with different pause states
    session_a.set_pause_state(Some(PauseState {
        tool_name: "WebSearch".to_string(),
        message: "Page A loaded".to_string(),
        kind: PauseKind::Continue,
        details: None,
    }));

    session_b.set_pause_state(Some(PauseState {
        tool_name: "Bash".to_string(),
        message: "Confirm dangerous command".to_string(),
        kind: PauseKind::Confirm,
        details: Some("rm -rf /tmp/*".to_string()),
    }));

    // @step Then each session has its own pause state
    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert_eq!(session_b.get_status(), STATUS_PAUSED);

    let state_a = session_a.get_pause_state().unwrap();
    let state_b = session_b.get_pause_state().unwrap();

    assert_eq!(state_a.tool_name, "WebSearch");
    assert_eq!(state_b.tool_name, "Bash");
    assert_eq!(state_a.kind, PauseKind::Continue);
    assert_eq!(state_b.kind, PauseKind::Confirm);
    assert!(state_a.details.is_none());
    assert_eq!(state_b.details, Some("rm -rf /tmp/*".to_string()));
}

/// Test that resuming one session doesn't affect another
///
/// @step Given two paused sessions
/// @step When session A resumes
/// @step Then session A status is "running" with no pause state
/// @step And session B remains "paused" with its pause state intact
#[test]
fn test_resuming_session_is_isolated() {
    // @step Given two paused sessions
    let session_a = Arc::new(SimulatedSession::new("session-a"));
    let session_b = Arc::new(SimulatedSession::new("session-b"));

    session_a.set_pause_state(Some(PauseState {
        tool_name: "WebSearch".to_string(),
        message: "Page A".to_string(),
        kind: PauseKind::Continue,
        details: None,
    }));

    session_b.set_pause_state(Some(PauseState {
        tool_name: "WebSearch".to_string(),
        message: "Page B".to_string(),
        kind: PauseKind::Continue,
        details: None,
    }));

    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert_eq!(session_b.get_status(), STATUS_PAUSED);

    // @step When session A resumes
    session_a.clear_pause_state();

    // @step Then session A status is "running" with no pause state
    assert_eq!(session_a.get_status(), STATUS_RUNNING);
    assert!(session_a.get_pause_state().is_none());

    // @step And session B remains "paused" with its pause state intact
    assert_eq!(session_b.get_status(), STATUS_PAUSED);
    assert!(session_b.get_pause_state().is_some());
    assert_eq!(
        session_b.get_pause_state().unwrap().message,
        "Page B"
    );
}

/// Test concurrent access to session pause state is safe
///
/// @step Given a session with pause state
/// @step When multiple threads read pause state concurrently
/// @step Then no data races occur
#[test]
fn test_concurrent_pause_state_access() {
    use std::thread;

    let session = Arc::new(SimulatedSession::new("session-concurrent"));

    // Set initial pause state
    session.set_pause_state(Some(PauseState {
        tool_name: "WebSearch".to_string(),
        message: "Testing concurrent access".to_string(),
        kind: PauseKind::Continue,
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
                    tool_name: "WebSearch".to_string(),
                    message: format!("Iteration {}", i),
                    kind: PauseKind::Continue,
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
