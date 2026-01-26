#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PAUSE-001: Stream Loop Pause Handler Integration Tests
//!
//! These tests verify that run_agent_stream correctly integrates with the
//! pause handler mechanism using task-local storage via with_pause_handler().
//!
//! Key behaviors tested:
//! 1. with_pause_handler() scopes the handler for the duration of the closure
//! 2. The handler captures session context for per-session isolation
//! 3. Concurrent tasks have isolated handlers (task-local storage)
//! 4. The handler correctly sets session pause state and waits for response

use codelet_tools::tool_pause::{
    has_pause_handler, pause_for_user, with_pause_handler, PauseHandler, PauseKind, PauseRequest,
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

/// @scenario: with_pause_handler scopes handler for the duration of the closure
/// @step: Given no handler is active outside the scope
/// @step: When we enter with_pause_handler scope
/// @step: Then has_pause_handler should return true inside the scope
/// @step: And has_pause_handler should return false after the scope
#[test]
fn test_with_pause_handler_scopes_correctly() {
    // Outside scope - no handler
    assert!(!has_pause_handler());
    
    let session = Arc::new(MockSession::new("test-session"));
    let handler = session.create_pause_handler();
    
    // Inside scope - handler active
    with_pause_handler(Some(handler), || {
        assert!(has_pause_handler());
    });
    
    // After scope - handler gone
    assert!(!has_pause_handler());
}

/// @scenario: Pause handler sets session state and blocks
/// @step: Given a pause handler is registered with session context
/// @step: When a tool calls pause_for_user
/// @step: Then the session status should be "paused"
/// @step: And the session pause_state should contain the request details
/// @step: And the tool should block until TypeScript responds
#[test]
fn test_pause_handler_sets_session_state_and_blocks() {
    use std::thread;
    use std::time::Duration;
    
    let session = Arc::new(MockSession::new("test-session"));
    session.set_status(STATUS_RUNNING);
    
    // Spawn thread simulating tool execution with scoped handler
    let session_clone = Arc::clone(&session);
    let tool_thread = thread::spawn(move || {
        let handler = session_clone.create_pause_handler();
        
        // with_pause_handler scopes the handler
        with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded at https://example.com".to_string(),
                details: None,
            })
        })
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
    
    // Spawn thread for session A's tool with scoped handler
    let session_a_clone = Arc::clone(&session_a);
    let session_a_clone2 = Arc::clone(&session_a);
    let tool_thread = thread::spawn(move || {
        let handler = session_a_clone.create_pause_handler();
        
        with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Session A page".to_string(),
                details: None,
            })
        })
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
            
            with_pause_handler(Some(handler), || {
                pause_for_user(PauseRequest {
                    kind: PauseKind::Confirm,
                    tool_name: "Bash".to_string(),
                    message: "Confirm command".to_string(),
                    details: Some("rm -rf /tmp/*".to_string()),
                })
            })
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
            
            with_pause_handler(Some(handler), || {
                pause_for_user(PauseRequest {
                    kind: PauseKind::Confirm,
                    tool_name: "Bash".to_string(),
                    message: "Confirm command".to_string(),
                    details: None,
                })
            })
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
        
        with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            })
        })
    });
    
    thread::sleep(Duration::from_millis(50));
    
    // User presses Esc
    session_clone2.send_response(PauseResponse::Interrupted);
    let response = tool_thread.join().unwrap();
    assert_eq!(response, PauseResponse::Interrupted);
}

/// @scenario: Task-local handler isolation between concurrent threads
/// @step: Given multiple threads running concurrently
/// @step: When each thread uses with_pause_handler with its own handler
/// @step: Then each thread's handler is isolated
#[test]
fn test_task_local_handler_isolation() {
    use std::thread;
    
    // Spawn multiple threads, each with its own scoped handler
    let handles: Vec<_> = (0..5)
        .map(|i| {
            thread::spawn(move || {
                let called = Arc::new(AtomicBool::new(false));
                let called_clone = called.clone();
                let expected_tool = format!("Tool{i}");
                let expected_tool_clone = expected_tool.clone();
                
                let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
                    called_clone.store(true, Ordering::SeqCst);
                    assert_eq!(request.tool_name, expected_tool_clone);
                    PauseResponse::Resumed
                });
                
                // Use with_pause_handler to scope the handler
                let response = with_pause_handler(Some(handler), || {
                    pause_for_user(PauseRequest {
                        kind: PauseKind::Continue,
                        tool_name: expected_tool,
                        message: format!("Message {i}"),
                        details: None,
                    })
                });
                
                // Verify own handler was called
                assert!(called.load(Ordering::SeqCst));
                assert_eq!(response, PauseResponse::Resumed);
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
/// @step: Given no pause handler is registered (outside any scope)
/// @step: When a tool calls pause_for_user
/// @step: Then it should return Resumed immediately without blocking
#[test]
fn test_no_handler_auto_resumes() {
    // Outside any scope - no handler
    let response = pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".to_string(),
        message: "Should auto-resume".to_string(),
        details: None,
    });
    
    // No handler = auto-resume (useful for headless mode / tests)
    assert_eq!(response, PauseResponse::Resumed);
}

/// @scenario: None handler in scope also auto-resumes
/// @step: Given with_pause_handler is called with None
/// @step: When a tool calls pause_for_user
/// @step: Then it should return Resumed immediately
#[test]
fn test_none_handler_auto_resumes() {
    let response = with_pause_handler(None, || {
        assert!(!has_pause_handler());
        pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Should auto-resume with None handler".to_string(),
            details: None,
        })
    });
    
    assert_eq!(response, PauseResponse::Resumed);
}
