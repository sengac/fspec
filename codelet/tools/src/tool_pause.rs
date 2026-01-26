//! Tool pause mechanism (PAUSE-001)
//!
//! Provides a generic pause/resume API for tools that need user interaction.
//! Supports two pause kinds:
//! - Continue: Press Enter to resume
//! - Confirm: Press Y to approve, N to deny
//!
//! ## Architecture
//!
//! Pause state is PER-SESSION (not global). This module provides:
//! 1. Type definitions shared between tools and session manager
//! 2. Thread-local handler mechanism for tools to request pauses
//!
//! The actual pause state is stored in `BackgroundSession` (napi crate).
//! The stream loop sets a handler that captures session context.
//!
//! ## Usage
//!
//! ```ignore
//! // Stream loop sets handler with session context:
//! set_pause_handler(Some(Arc::new(move |request| {
//!     session.set_pause_state(Some(request.into()));
//!     session.set_status(SessionStatus::Paused);
//!     let response = session.wait_for_pause_response();
//!     session.set_pause_state(None);
//!     session.set_status(SessionStatus::Running);
//!     response
//! })));
//!
//! // Tool requests pause:
//! match pause_for_user(PauseRequest {
//!     kind: PauseKind::Continue,
//!     tool_name: "WebSearch".into(),
//!     message: "Page loaded".into(),
//!     details: None,
//! }) {
//!     PauseResponse::Resumed => { /* continue */ }
//!     PauseResponse::Interrupted => { /* abort */ }
//!     _ => unreachable!(),
//! }
//!
//! // Stream loop clears handler after agent completes:
//! set_pause_handler(None);
//! ```

use std::cell::RefCell;
use std::sync::Arc;

/// Kind of pause requested by a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    /// Simple pause - press Enter to continue
    Continue,
    /// Confirmation required - press Y to approve, N to deny
    Confirm,
}

/// Request to pause tool execution
#[derive(Debug, Clone)]
pub struct PauseRequest {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

/// Response from user after pause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseResponse {
    /// User pressed Enter (Continue pause)
    Resumed,
    /// User pressed Y (Confirm pause)
    Approved,
    /// User pressed N (Confirm pause)
    Denied,
    /// User pressed Esc (any pause type)
    Interrupted,
}

/// Current pause state (for UI display)
#[derive(Debug, Clone)]
pub struct PauseState {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<PauseRequest> for PauseState {
    fn from(req: PauseRequest) -> Self {
        Self {
            kind: req.kind,
            tool_name: req.tool_name,
            message: req.message,
            details: req.details,
        }
    }
}

/// Handler type for pause requests.
///
/// The handler is set by the stream loop and captures session context.
/// It blocks until the user responds (via NAPI calls from UI).
pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>;

// Thread-local handler - each agent task has its own handler
// This prevents concurrent sessions from interfering with each other
thread_local! {
    static PAUSE_HANDLER: RefCell<Option<PauseHandler>> = const { RefCell::new(None) };
}

/// Set the pause handler for the current thread.
///
/// Called by stream loop before running agent (with session context captured in closure).
/// Called with `None` after agent completes to clean up.
///
/// # Thread Safety
///
/// Uses thread-local storage, so each agent execution context has its own handler.
/// This is safe for concurrent sessions running in different threads/tasks.
pub fn set_pause_handler(handler: Option<PauseHandler>) {
    PAUSE_HANDLER.with(|h| {
        *h.borrow_mut() = handler;
    });
}

/// Pause tool execution and wait for user response.
///
/// This function BLOCKS until the user responds (Enter/Y/N/Esc).
///
/// If no handler is registered (e.g., running in headless mode or tests),
/// returns `PauseResponse::Resumed` immediately (no-op).
///
/// # Returns
///
/// - `Resumed` - User pressed Enter (Continue pause) or no handler registered
/// - `Approved` - User pressed Y (Confirm pause)
/// - `Denied` - User pressed N (Confirm pause)
/// - `Interrupted` - User pressed Esc
pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    PAUSE_HANDLER.with(|h| {
        let handler = h.borrow();
        if let Some(handler) = handler.as_ref() {
            handler(request)
        } else {
            // No handler = auto-resume (headless mode, tests, etc.)
            PauseResponse::Resumed
        }
    })
}

/// Check if a pause handler is currently registered.
///
/// Useful for tools to skip pause-related setup if no UI is attached.
pub fn has_pause_handler() -> bool {
    PAUSE_HANDLER.with(|h| h.borrow().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_pause_kind_enum() {
        assert_eq!(PauseKind::Continue, PauseKind::Continue);
        assert_eq!(PauseKind::Confirm, PauseKind::Confirm);
        assert_ne!(PauseKind::Continue, PauseKind::Confirm);
    }

    #[test]
    fn test_pause_response_enum() {
        assert_eq!(PauseResponse::Resumed, PauseResponse::Resumed);
        assert_eq!(PauseResponse::Approved, PauseResponse::Approved);
        assert_eq!(PauseResponse::Denied, PauseResponse::Denied);
        assert_eq!(PauseResponse::Interrupted, PauseResponse::Interrupted);
    }

    #[test]
    fn test_pause_state_from_request() {
        let request = PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test message".to_string(),
            details: Some("details".to_string()),
        };
        let state: PauseState = request.into();
        assert_eq!(state.kind, PauseKind::Continue);
        assert_eq!(state.tool_name, "Test");
        assert_eq!(state.message, "Test message");
        assert_eq!(state.details, Some("details".to_string()));
    }

    #[test]
    fn test_no_handler_returns_resumed() {
        // Clear any existing handler
        set_pause_handler(None);
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });
        
        assert_eq!(response, PauseResponse::Resumed);
    }

    #[test]
    fn test_has_pause_handler() {
        set_pause_handler(None);
        assert!(!has_pause_handler());
        
        set_pause_handler(Some(Arc::new(|_| PauseResponse::Resumed)));
        assert!(has_pause_handler());
        
        set_pause_handler(None);
        assert!(!has_pause_handler());
    }

    #[test]
    fn test_handler_is_called_with_correct_request() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        set_pause_handler(Some(Arc::new(move |request| {
            called_clone.store(true, Ordering::SeqCst);
            assert_eq!(request.kind, PauseKind::Continue);
            assert_eq!(request.tool_name, "WebSearch");
            assert_eq!(request.message, "Page loaded");
            PauseResponse::Resumed
        })));
        
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });
        
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(response, PauseResponse::Resumed);
        
        set_pause_handler(None);
    }

    #[test]
    fn test_handler_can_return_different_responses() {
        // Test Approved
        set_pause_handler(Some(Arc::new(|_| PauseResponse::Approved)));
        assert_eq!(
            pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            }),
            PauseResponse::Approved
        );
        
        // Test Denied
        set_pause_handler(Some(Arc::new(|_| PauseResponse::Denied)));
        assert_eq!(
            pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            }),
            PauseResponse::Denied
        );
        
        // Test Interrupted
        set_pause_handler(Some(Arc::new(|_| PauseResponse::Interrupted)));
        assert_eq!(
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            }),
            PauseResponse::Interrupted
        );
        
        set_pause_handler(None);
    }
}
