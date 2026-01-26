//! Tool pause mechanism
//!
//! Provides a generic pause/resume API for tools that need user interaction.
//! Supports two pause kinds:
//! - Continue: Press Enter to resume
//! - Confirm: Press Y to approve, N to deny
//!
//! Pause state is PER-SESSION. This module provides type definitions shared 
//! between tools and session manager, plus a global handler mechanism.
//!
//! Uses a global `RwLock<Option<PauseHandler>>` because Rig spawns tool 
//! execution on separate tokio tasks where task-local storage doesn't propagate.

use std::sync::{Arc, RwLock};

/// Kind of pause requested by a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    Continue,
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
    Resumed,
    Approved,
    Denied,
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

pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>;

static PAUSE_HANDLER: RwLock<Option<PauseHandler>> = RwLock::new(None);

pub fn set_pause_handler(handler: Option<PauseHandler>) {
    if let Ok(mut guard) = PAUSE_HANDLER.write() {
        *guard = handler;
    }
}

pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    let handler = match PAUSE_HANDLER.read() {
        Ok(guard) => guard.clone(),
        Err(_) => return PauseResponse::Resumed,
    };
    
    match handler {
        Some(h) => h(request),
        None => PauseResponse::Resumed,
    }
}

pub fn has_pause_handler() -> bool {
    PAUSE_HANDLER.read()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
        set_pause_handler(None);
        let result = f();
        set_pause_handler(None);
        result
    }

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

    #[test]
    fn test_has_pause_handler_when_not_set() {
        with_clean_handler(|| {
            assert!(!has_pause_handler());
        });
    }

    #[test]
    fn test_set_pause_handler_sets_handler() {
        with_clean_handler(|| {
            let called = Arc::new(AtomicBool::new(false));
            let called_clone = called.clone();

            let handler: PauseHandler = Arc::new(move |_| {
                called_clone.store(true, Ordering::SeqCst);
                PauseResponse::Resumed
            });

            set_pause_handler(Some(handler));
            assert!(has_pause_handler());

            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            });

            assert_eq!(response, PauseResponse::Resumed);
            assert!(called.load(Ordering::SeqCst));

            set_pause_handler(None);
            assert!(!has_pause_handler());
        });
    }

    #[test]
    fn test_handler_receives_correct_request() {
        with_clean_handler(|| {
            let handler: PauseHandler = Arc::new(|request| {
                assert_eq!(request.kind, PauseKind::Continue);
                assert_eq!(request.tool_name, "WebSearch");
                assert_eq!(request.message, "Page loaded");
                PauseResponse::Resumed
            });

            set_pause_handler(Some(handler));
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

            let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
            set_pause_handler(Some(handler));
            let response = pause_for_user(PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            });
            assert_eq!(response, PauseResponse::Denied);

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
}
