//! Fspec command handler mechanism
//!
//! Provides per-session handlers for FspecTool to execute commands via TypeScript.
//! Similar architecture to tool_pause.rs but for fspec command execution.
//!
//! The handler is set per-session in agent_loop and routes fspec commands
//! through TypeScript's fspecCallback implementation.
//!
//! ## Architecture
//!
//! 1. Session manager registers handler via `set_fspec_handler_for_session(session_id, handler)`
//! 2. FspecToolFacadeWrapper (constructed with session_id) calls `execute_fspec_command_for_session(session_id, request)`
//! 3. Handler emits FspecCommandRequest chunk to TypeScript
//! 4. Handler blocks on channel waiting for TypeScript response
//! 5. TypeScript executes command and calls `sessionSendFspecResult()`
//! 6. Handler receives result and returns to FspecTool
//! 7. FspecTool returns actual result (not marker) to LLM
//!
//! ## Session Association (TOOL-012)
//!
//! Tools are constructed WITH their session_id at creation time. The session_id
//! is stored as a field on the wrapper struct. At call time, tools use
//! `self.session_id` directly to look up their handler - no thread-local state.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

/// Request to execute an fspec command
#[derive(Debug, Clone)]
pub struct FspecRequest {
    /// The fspec command (e.g., "list-work-units", "show-work-unit")
    pub command: String,
    /// Command arguments as JSON string
    pub args_json: String,
    /// Project root directory
    pub project_root: String,
    /// Provider name (for facade-specific handling)
    pub provider: String,
}

/// Result from fspec command execution
#[derive(Debug, Clone)]
pub struct FspecResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Command output data
    pub data: String,
    /// Error message if failed
    pub error: Option<String>,
    /// System reminder for workflow orchestration
    pub system_reminder: Option<String>,
}

impl Default for FspecResult {
    fn default() -> Self {
        Self {
            success: false,
            data: String::new(),
            error: Some("Fspec handler not configured".to_string()),
            system_reminder: None,
        }
    }
}

/// Handler function type for fspec command execution
/// Takes a request and returns the result (blocking until TypeScript responds)
pub type FspecHandler = Arc<dyn Fn(FspecRequest) -> FspecResult + Send + Sync>;

/// Per-session handler storage.
/// Uses a global HashMap keyed by session UUID - handlers are shared across threads.
static FSPEC_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, FspecHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the fspec command handler for a specific session
///
/// Called by session manager before agent run to configure how fspec commands
/// are routed to TypeScript for this session.
pub fn set_fspec_handler_for_session(session_id: Uuid, handler: Option<FspecHandler>) {
    if let Ok(mut guard) = FSPEC_HANDLERS.write() {
        match handler {
            Some(h) => {
                guard.insert(session_id, h);
            }
            None => {
                guard.remove(&session_id);
            }
        }
    }
}

/// Check if a fspec handler is configured for a specific session
pub fn has_fspec_handler_for_session(session_id: Uuid) -> bool {
    FSPEC_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute an fspec command via the handler for a specific session
///
/// Called by FspecToolFacadeWrapper when the LLM invokes the Fspec tool.
/// Blocks until TypeScript executes the command and returns the result.
pub fn execute_fspec_command_for_session(session_id: Uuid, request: FspecRequest) -> FspecResult {
    let handler = match FSPEC_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return FspecResult {
                success: false,
                data: String::new(),
                error: Some("Failed to acquire fspec handlers lock".to_string()),
                system_reminder: None,
            };
        }
    };

    match handler {
        Some(h) => h(request),
        None => FspecResult {
            success: false,
            data: String::new(),
            error: Some(format!(
                "Fspec handler not configured for session {session_id} - FspecTool requires session context"
            )),
            system_reminder: None,
        },
    }
}

/// Clear all fspec handlers (for testing)
pub fn clear_all_fspec_handlers() {
    if let Ok(mut guard) = FSPEC_HANDLERS.write() {
        guard.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn with_clean_handlers<T>(f: impl FnOnce() -> T) -> T {
        clear_all_fspec_handlers();
        let result = f();
        clear_all_fspec_handlers();
        result
    }

    #[test]
    #[serial]
    fn test_no_handler_returns_error() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();
            let result = execute_fspec_command_for_session(
                session_id,
                FspecRequest {
                    command: "list-work-units".to_string(),
                    args_json: "{}".to_string(),
                    project_root: ".".to_string(),
                    provider: "claude".to_string(),
                },
            );

            assert!(!result.success);
            assert!(result.error.is_some());
            assert!(result.error.unwrap().contains("not configured"));
        });
    }

    #[test]
    #[serial]
    fn test_handler_receives_request() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();
            let called = Arc::new(AtomicBool::new(false));
            let called_clone = called.clone();

            let handler: FspecHandler = Arc::new(move |req| {
                called_clone.store(true, Ordering::SeqCst);
                assert_eq!(req.command, "test-command");
                FspecResult {
                    success: true,
                    data: "test result".to_string(),
                    error: None,
                    system_reminder: None,
                }
            });

            set_fspec_handler_for_session(session_id, Some(handler));

            let result = execute_fspec_command_for_session(
                session_id,
                FspecRequest {
                    command: "test-command".to_string(),
                    args_json: "{}".to_string(),
                    project_root: ".".to_string(),
                    provider: "claude".to_string(),
                },
            );

            assert!(called.load(Ordering::SeqCst));
            assert!(result.success);
            assert_eq!(result.data, "test result");
        });
    }

    #[test]
    #[serial]
    fn test_has_fspec_handler_for_session() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();

            assert!(!has_fspec_handler_for_session(session_id));

            let handler: FspecHandler = Arc::new(|_| FspecResult::default());
            set_fspec_handler_for_session(session_id, Some(handler));

            assert!(has_fspec_handler_for_session(session_id));

            set_fspec_handler_for_session(session_id, None);
            assert!(!has_fspec_handler_for_session(session_id));
        });
    }

    #[test]
    #[serial]
    fn test_concurrent_sessions_isolated() {
        with_clean_handlers(|| {
            let session_a = Uuid::new_v4();
            let session_b = Uuid::new_v4();

            let handler_a: FspecHandler = Arc::new(|_| FspecResult {
                success: true,
                data: "from_session_a".to_string(),
                error: None,
                system_reminder: None,
            });
            set_fspec_handler_for_session(session_a, Some(handler_a));

            let handler_b: FspecHandler = Arc::new(|_| FspecResult {
                success: true,
                data: "from_session_b".to_string(),
                error: None,
                system_reminder: None,
            });
            set_fspec_handler_for_session(session_b, Some(handler_b));

            let result_a = execute_fspec_command_for_session(
                session_a,
                FspecRequest {
                    command: "test".to_string(),
                    args_json: "{}".to_string(),
                    project_root: ".".to_string(),
                    provider: "claude".to_string(),
                },
            );
            assert_eq!(result_a.data, "from_session_a");

            let result_b = execute_fspec_command_for_session(
                session_b,
                FspecRequest {
                    command: "test".to_string(),
                    args_json: "{}".to_string(),
                    project_root: ".".to_string(),
                    provider: "claude".to_string(),
                },
            );
            assert_eq!(result_b.data, "from_session_b");

            // Remove session B's handler - session A should still work
            set_fspec_handler_for_session(session_b, None);
            let result_a2 = execute_fspec_command_for_session(
                session_a,
                FspecRequest {
                    command: "test".to_string(),
                    args_json: "{}".to_string(),
                    project_root: ".".to_string(),
                    provider: "claude".to_string(),
                },
            );
            assert_eq!(result_a2.data, "from_session_a");
        });
    }
}
