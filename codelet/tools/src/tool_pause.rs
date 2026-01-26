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
//! 2. Task-local handler mechanism for tools to request pauses
//!
//! The actual pause state is stored in `BackgroundSession` (napi crate).
//! The stream loop sets a handler that captures session context.
//!
//! ## Important: Task-Local Storage
//!
//! This module uses `tokio::task_local!` because tokio's multi-threaded runtime
//! can migrate async tasks between threads at `.await` points. Task-local storage
//! stays with the task across thread migrations, unlike `thread_local!`.
//!
//! ## Usage
//!
//! ```ignore
//! // Stream loop wraps agent execution with pause handler:
//! let handler = Arc::new(move |request: PauseRequest| {
//!     session.set_pause_state(Some(request.into()));
//!     let response = session.wait_for_pause_response();
//!     session.clear_pause_state();
//!     response
//! });
//!
//! // For sync code (blocking agent execution):
//! with_pause_handler(Some(handler), || {
//!     run_agent_stream(...)
//! });
//!
//! // Tool requests pause (inside the scope):
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
//! ```

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

// Task-local handler - follows the async task across thread migrations
tokio::task_local! {
    pub static PAUSE_HANDLER: Option<PauseHandler>;
}

/// Execute synchronous code with a pause handler in scope.
///
/// The handler is available to all `pause_for_user` calls within the closure.
/// Uses `sync_scope` for synchronous code execution.
///
/// # Example
///
/// ```ignore
/// let handler = Arc::new(|request| PauseResponse::Resumed);
/// with_pause_handler(Some(handler), || {
///     // pause_for_user() calls will use this handler
///     some_sync_function()
/// });
/// ```
pub fn with_pause_handler<T>(handler: Option<PauseHandler>, f: impl FnOnce() -> T) -> T {
    PAUSE_HANDLER.sync_scope(handler, f)
}

/// Execute async code with a pause handler in scope.
///
/// The handler is available to all `pause_for_user` calls within the async block.
/// Uses `scope` which properly handles async code with `.await` points.
///
/// # Example
///
/// ```ignore
/// let handler = Arc::new(|request| PauseResponse::Resumed);
/// let result = with_pause_handler_async(Some(handler), async {
///     // pause_for_user() calls will use this handler
///     some_async_function().await
/// }).await;
/// ```
pub async fn with_pause_handler_async<F, T>(handler: Option<PauseHandler>, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    PAUSE_HANDLER.scope(handler, f).await
}

/// Pause tool execution and wait for user response.
///
/// This function BLOCKS until the user responds (Enter/Y/N/Esc).
///
/// If no handler is registered (e.g., running outside a scope, in headless
/// mode, or in tests without a handler), returns `PauseResponse::Resumed`
/// immediately (no-op).
///
/// # Returns
///
/// - `Resumed` - User pressed Enter (Continue pause) or no handler registered
/// - `Approved` - User pressed Y (Confirm pause)
/// - `Denied` - User pressed N (Confirm pause)
/// - `Interrupted` - User pressed Esc
pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    match PAUSE_HANDLER.try_with(|h| h.clone()) {
        Ok(Some(handler)) => handler(request),
        Ok(None) | Err(_) => {
            // No handler = auto-resume (headless mode, tests, outside scope, etc.)
            PauseResponse::Resumed
        }
    }
}

/// Check if a pause handler is currently registered.
///
/// Useful for tools to skip pause-related setup if no UI is attached.
pub fn has_pause_handler() -> bool {
    matches!(PAUSE_HANDLER.try_with(|h| h.is_some()), Ok(true))
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
        // Outside any scope - should return Resumed immediately
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "Test".to_string(),
            message: "Test".to_string(),
            details: None,
        });
        assert_eq!(response, PauseResponse::Resumed);
    }

    #[test]
    fn test_has_pause_handler_outside_scope() {
        // Outside any scope
        assert!(!has_pause_handler());
    }

    #[test]
    fn test_with_pause_handler_sets_handler() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: PauseHandler = Arc::new(move |_| {
            called_clone.store(true, Ordering::SeqCst);
            PauseResponse::Resumed
        });

        with_pause_handler(Some(handler), || {
            assert!(has_pause_handler());

            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            });

            assert_eq!(response, PauseResponse::Resumed);
        });

        assert!(called.load(Ordering::SeqCst));

        // After scope, handler should not be available
        assert!(!has_pause_handler());
    }

    #[test]
    fn test_handler_receives_correct_request() {
        let handler: PauseHandler = Arc::new(|request| {
            assert_eq!(request.kind, PauseKind::Continue);
            assert_eq!(request.tool_name, "WebSearch");
            assert_eq!(request.message, "Page loaded");
            PauseResponse::Resumed
        });

        with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            });
        });
    }

    #[test]
    fn test_handler_can_return_different_responses() {
        // Test Approved
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
        let response = with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            })
        });
        assert_eq!(response, PauseResponse::Approved);

        // Test Denied
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
        let response = with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            })
        });
        assert_eq!(response, PauseResponse::Denied);

        // Test Interrupted
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Interrupted);
        let response = with_pause_handler(Some(handler), || {
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            })
        });
        assert_eq!(response, PauseResponse::Interrupted);
    }

    #[test]
    fn test_none_handler_in_scope() {
        // Explicitly passing None handler
        let response = with_pause_handler(None, || {
            assert!(!has_pause_handler());
            pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            })
        });
        assert_eq!(response, PauseResponse::Resumed);
    }
}
