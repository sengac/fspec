//! Fspec command handler mechanism
//!
//! Provides a global handler for FspecTool to execute commands via TypeScript.
//! Similar architecture to tool_pause.rs but for fspec command execution.
//!
//! The handler is set per-session in agent_loop and routes fspec commands
//! through TypeScript's fspecCallback implementation.
//!
//! ## Architecture
//!
//! 1. Session manager sets handler via `set_fspec_handler()` before agent run
//! 2. FspecToolFacadeWrapper calls `execute_fspec_command()` 
//! 3. Handler emits FspecCommandRequest chunk to TypeScript
//! 4. Handler blocks on channel waiting for TypeScript response
//! 5. TypeScript executes command and calls `sessionSendFspecResult()`
//! 6. Handler receives result and returns to FspecTool
//! 7. FspecTool returns actual result (not marker) to LLM

use std::sync::{Arc, RwLock};

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

static FSPEC_HANDLER: RwLock<Option<FspecHandler>> = RwLock::new(None);

/// Set the fspec command handler
/// 
/// Called by session manager before agent run to configure how fspec commands
/// are routed to TypeScript.
pub fn set_fspec_handler(handler: Option<FspecHandler>) {
    if let Ok(mut guard) = FSPEC_HANDLER.write() {
        *guard = handler;
    }
}

/// Execute an fspec command via the configured handler
/// 
/// Called by FspecToolFacadeWrapper when the LLM invokes the Fspec tool.
/// Blocks until TypeScript executes the command and returns the result.
/// 
/// Returns an error result if no handler is configured.
pub fn execute_fspec_command(request: FspecRequest) -> FspecResult {
    tracing::warn!("[FSPEC_HANDLER] execute_fspec_command called: command={}, project_root={}", 
        request.command, request.project_root);
    
    let handler = match FSPEC_HANDLER.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            tracing::warn!("[FSPEC_HANDLER] Failed to acquire read lock: {:?}", e);
            return FspecResult::default();
        }
    };
    
    match handler {
        Some(h) => {
            tracing::warn!("[FSPEC_HANDLER] Handler found, calling handler closure...");
            let result = h(request);
            tracing::warn!("[FSPEC_HANDLER] Handler returned: success={}", result.success);
            result
        }
        None => {
            tracing::warn!("[FSPEC_HANDLER] No handler configured!");
            FspecResult {
                success: false,
                data: String::new(),
                error: Some("Fspec handler not configured - FspecTool requires session context".to_string()),
                system_reminder: None,
            }
        }
    }
}

/// Check if a fspec handler is configured
pub fn has_fspec_handler() -> bool {
    FSPEC_HANDLER.read()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
        set_fspec_handler(None);
        let result = f();
        set_fspec_handler(None);
        result
    }

    #[test]
    fn test_no_handler_returns_error() {
        with_clean_handler(|| {
            let result = execute_fspec_command(FspecRequest {
                command: "list-work-units".to_string(),
                args_json: "{}".to_string(),
                project_root: ".".to_string(),
                provider: "claude".to_string(),
            });
            
            assert!(!result.success);
            assert!(result.error.is_some());
        });
    }

    #[test]
    fn test_handler_receives_request() {
        with_clean_handler(|| {
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
            
            set_fspec_handler(Some(handler));
            
            let result = execute_fspec_command(FspecRequest {
                command: "test-command".to_string(),
                args_json: "{}".to_string(),
                project_root: ".".to_string(),
                provider: "claude".to_string(),
            });
            
            assert!(called.load(Ordering::SeqCst));
            assert!(result.success);
            assert_eq!(result.data, "test result");
        });
    }

    #[test]
    fn test_has_fspec_handler() {
        with_clean_handler(|| {
            assert!(!has_fspec_handler());
            
            let handler: FspecHandler = Arc::new(|_| FspecResult::default());
            set_fspec_handler(Some(handler));
            
            assert!(has_fspec_handler());
            
            set_fspec_handler(None);
            assert!(!has_fspec_handler());
        });
    }
}
