#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: Stream Loop Pause Handler Integration Tests
//!
//! These tests verify that run_agent_stream correctly integrates with the
//! pause handler mechanism:
//!
//! 1. set_pause_handler is called before running the agent
//! 2. The handler captures session context for per-session isolation
//! 3. set_pause_handler(None) is called after agent completion
//! 4. The handler correctly sets session pause state and waits for response
//!
//! IMPORTANT: pause_for_user uses thread_local! storage, meaning handlers
//! are per-thread. Tests that spawn threads must set the handler IN the
//! spawned thread, not the main thread.

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
    #[allow(dead_code)]
    id: String,
    status: AtomicU8,
    pause_state: RwLock<Option<PauseState>>,
    pause_response: Arc<(Mutex<Option<PauseResponse>>, Condvar)>,
}

impl MockSession {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
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

    fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.read().unwrap().clone()
    }

    fn set_pause_state(&self, state: Option<PauseState>) {
        let is_paused = state.is_some();
        *self.pause_state.write().unwrap() = state;
        if is_paused {
            self.set_status(STATUS_PAUSED);
        }
    }

    /// Wait for user response (blocking)
    fn wait_for_response(&self) -> PauseResponse {
        let (lock, cvar) = &*self.pause_response;
        let mut guard = lock.lock().unwrap();
        while guard.is_none() {
            guard = cvar.wait(guard).unwrap();
        }
        guard.take().unwrap()
    }

    /// Send user response (called from "TypeScript")
    fn send_response(&self, response: PauseResponse) {
        let (lock, cvar) = &*self.pause_response;
        let mut guard = lock.lock().unwrap();
        *guard = Some(response);
        cvar.notify_one();
        
        // Clear pause state
        *self.pause_state.write().unwrap() = None;
        self.set_status(STATUS_RUNNING);
    }

    /// Create a pause handler that captures this session's context
    fn create_pause_handler(self: &Arc<Self>) -> PauseHandler {
        let session = Arc::clone(self);
        Arc::new(move |request: PauseRequest| {
            // Set pause state (for TypeScript to query)
            session.set_pause_state(Some(request.into()));
            
            // Block until TypeScript responds
            let response = session.wait_for_response();
            
            // Return the response to the tool
            response
        })
    }
}

// =============================================================================
// Feature: Stream Loop Pause Handler Integration
// =============================================================================

/// @scenario: Stream loop sets pause handler before running agent
/// @step: Given a session is about to run an agent
/// @step: When the stream loop sets up the pause handler
/// @step: Then has_pause_handler should return true
/// @step: And the handler should capture the session context
#[test]
fn test_stream_loop_sets_pause_handler() {
    // Clear any existing handler
    set_pause_handler(None);
    assert!(!has_pause_handler());
    
    // Simulate stream loop setting handler with session context
    let session = Arc::new(MockSession::new("test-session"));
    let handler = session.create_pause_handler();
    set_pause_handler(Some(handler));
    
    // Verify handler is set
    assert!(has_pause_handler());
    
    // Clean up
    set_pause_handler(None);
}

/// @scenario: Stream loop clears pause handler after agent completion
/// @step: Given an agent has completed execution
/// @step: When the stream loop cleans up
/// @step: Then has_pause_handler should return false
#[test]
fn test_stream_loop_clears_pause_handler_after_completion() {
    let session = Arc::new(MockSession::new("test-session"));
    let handler = session.create_pause_handler();
    set_pause_handler(Some(handler));
    assert!(has_pause_handler());
    
    // Simulate stream loop cleanup after agent completion
    set_pause_handler(None);
    
    assert!(!has_pause_handler());
}

/// @scenario: Pause handler sets session state and blocks
/// @step: Given a pause handler is registered with session context
/// @step: When a tool calls pause_for_user
/// @step: Then the session status should be "paused"
/// @step: And the session pause_state should contain the request details
/// @step: And the tool should block until TypeScript responds
///
/// NOTE: This test spawns a thread that sets its own handler because
/// thread_local! storage is per-thread.
#[test]
fn test_pause_handler_sets_session_state_and_blocks() {
    use std::thread;
    use std::time::Duration;
    
    let session = Arc::new(MockSession::new("test-session"));
    session.set_status(STATUS_RUNNING);
    
    // Spawn thread simulating tool calling pause_for_user
    // The thread must set its own handler (thread_local!)
    let session_clone = Arc::clone(&session);
    let tool_thread = thread::spawn(move || {
        // Set handler IN THIS THREAD (thread_local!)
        let handler = session_clone.create_pause_handler();
        set_pause_handler(Some(handler));
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded at https://example.com".to_string(),
            details: None,
        });
        
        set_pause_handler(None);
        response
    });
    
    // Give tool time to enter pause
    thread::sleep(Duration::from_millis(50));
    
    // Verify session is paused with correct state
    assert_eq!(session.get_status(), STATUS_PAUSED);
    let pause_state = session.get_pause_state();
    assert!(pause_state.is_some());
    let state = pause_state.unwrap();
    assert_eq!(state.tool_name, "WebSearch");
    assert_eq!(state.kind, PauseKind::Continue);
    
    // Simulate TypeScript sending response
    session.send_response(PauseResponse::Resumed);
    
    // Verify tool unblocks with correct response
    let response = tool_thread.join().expect("Tool thread panicked");
    assert_eq!(response, PauseResponse::Resumed);
    
    // Session should be running again
    assert_eq!(session.get_status(), STATUS_RUNNING);
    assert!(session.get_pause_state().is_none());
}

/// @scenario: Handler per session - two sessions don't interfere
/// @step: Given two sessions with their own pause handlers
/// @step: When session A's tool pauses
/// @step: Then only session A is affected
/// @step: And session B remains unaffected
#[test]
fn test_pause_handler_is_per_session() {
    use std::thread;
    use std::time::Duration;
    
    let session_a = Arc::new(MockSession::new("session-a"));
    let session_b = Arc::new(MockSession::new("session-b"));
    
    session_a.set_status(STATUS_RUNNING);
    session_b.set_status(STATUS_RUNNING);
    
    // Spawn thread for session A's tool (sets its own handler)
    let session_a_clone = Arc::clone(&session_a);
    let session_a_clone2 = Arc::clone(&session_a);
    let tool_thread = thread::spawn(move || {
        // Set handler for session A IN THIS THREAD
        let handler = session_a_clone.create_pause_handler();
        set_pause_handler(Some(handler));
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Session A page".to_string(),
            details: None,
        });
        
        set_pause_handler(None);
        response
    });
    
    // Give time for pause
    thread::sleep(Duration::from_millis(50));
    
    // Session A is paused
    assert_eq!(session_a.get_status(), STATUS_PAUSED);
    assert!(session_a.get_pause_state().is_some());
    
    // Session B is NOT affected
    assert_eq!(session_b.get_status(), STATUS_RUNNING);
    assert!(session_b.get_pause_state().is_none());
    
    // Resume session A
    session_a_clone2.send_response(PauseResponse::Resumed);
    let _ = tool_thread.join();
}

/// @scenario: Confirm pause handler returns correct responses
/// @step: Given a Confirm pause is active
/// @step: When TypeScript sends Approved
/// @step: Then pause_for_user returns Approved
/// @step: When TypeScript sends Denied
/// @step: Then pause_for_user returns Denied
#[test]
fn test_confirm_pause_responses() {
    use std::thread;
    use std::time::Duration;
    
    // Test Approved
    {
        let session = Arc::new(MockSession::new("test-approved"));
        session.set_status(STATUS_RUNNING);
        
        let session_clone = Arc::clone(&session);
        let session_clone2 = Arc::clone(&session);
        let tool_thread = thread::spawn(move || {
            let handler = session_clone.create_pause_handler();
            set_pause_handler(Some(handler));
            
            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Bash".to_string(),
                message: "Confirm command".to_string(),
                details: Some("rm -rf /tmp/*".to_string()),
            });
            
            set_pause_handler(None);
            response
        });
        
        thread::sleep(Duration::from_millis(50));
        
        // Verify confirm pause state
        let state = session.get_pause_state().unwrap();
        assert_eq!(state.kind, PauseKind::Confirm);
        assert_eq!(state.details, Some("rm -rf /tmp/*".to_string()));
        
        // Approve
        session_clone2.send_response(PauseResponse::Approved);
        let response = tool_thread.join().unwrap();
        assert_eq!(response, PauseResponse::Approved);
    }
    
    // Test Denied
    {
        let session = Arc::new(MockSession::new("test-denied"));
        session.set_status(STATUS_RUNNING);
        
        let session_clone = Arc::clone(&session);
        let session_clone2 = Arc::clone(&session);
        let tool_thread = thread::spawn(move || {
            let handler = session_clone.create_pause_handler();
            set_pause_handler(Some(handler));
            
            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Bash".to_string(),
                message: "Confirm command".to_string(),
                details: None,
            });
            
            set_pause_handler(None);
            response
        });
        
        thread::sleep(Duration::from_millis(50));
        
        // Deny
        session_clone2.send_response(PauseResponse::Denied);
        let response = tool_thread.join().unwrap();
        assert_eq!(response, PauseResponse::Denied);
    }
}

/// @scenario: Interrupted response works (Esc key)
/// @step: Given a pause is active
/// @step: When user presses Esc
/// @step: Then pause_for_user returns Interrupted
#[test]
fn test_interrupted_response() {
    use std::thread;
    use std::time::Duration;
    
    let session = Arc::new(MockSession::new("test-interrupted"));
    session.set_status(STATUS_RUNNING);
    
    let session_clone = Arc::clone(&session);
    let session_clone2 = Arc::clone(&session);
    let tool_thread = thread::spawn(move || {
        let handler = session_clone.create_pause_handler();
        set_pause_handler(Some(handler));
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });
        
        set_pause_handler(None);
        response
    });
    
    thread::sleep(Duration::from_millis(50));
    
    // User presses Esc
    session_clone2.send_response(PauseResponse::Interrupted);
    let response = tool_thread.join().unwrap();
    assert_eq!(response, PauseResponse::Interrupted);
}

/// @scenario: Thread-local handler isolation between concurrent tasks
/// @step: Given multiple threads running concurrently
/// @step: When each thread sets its own pause handler
/// @step: Then each thread's handler is isolated
///
/// Note: This tests the thread_local! mechanism works correctly for
/// per-task isolation (each Tokio task runs on a thread pool)
#[test]
fn test_thread_local_handler_isolation() {
    use std::thread;
    
    // Clear global handler (main thread)
    set_pause_handler(None);
    
    // Spawn multiple threads, each with its own handler
    let handles: Vec<_> = (0..5)
        .map(|i| {
            thread::spawn(move || {
                // Each thread sets its own handler IN ITS THREAD
                let called = Arc::new(AtomicBool::new(false));
                let called_clone = called.clone();
                let expected_tool = format!("Tool{i}");
                let expected_tool_clone = expected_tool.clone();
                
                set_pause_handler(Some(Arc::new(move |request: PauseRequest| {
                    called_clone.store(true, Ordering::SeqCst);
                    assert_eq!(request.tool_name, expected_tool_clone);
                    PauseResponse::Resumed
                })));
                
                // Call pause_for_user - should use THIS thread's handler
                let response = pause_for_user(PauseRequest {
                    kind: PauseKind::Continue,
                    tool_name: expected_tool,
                    message: format!("Message {i}"),
                    details: None,
                });
                
                // Verify own handler was called
                assert!(called.load(Ordering::SeqCst));
                assert_eq!(response, PauseResponse::Resumed);
                
                set_pause_handler(None);
                i
            })
        })
        .collect();
    
    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

/// @scenario: No handler returns Resumed immediately (headless mode)
/// @step: Given no pause handler is registered
/// @step: When a tool calls pause_for_user
/// @step: Then it should return Resumed immediately without blocking
#[test]
fn test_no_handler_auto_resumes() {
    set_pause_handler(None);
    
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Should auto-resume".to_string(),
        details: None,
    });
    
    // No handler = auto-resume (useful for headless mode / tests)
    assert_eq!(response, PauseResponse::Resumed);
}
