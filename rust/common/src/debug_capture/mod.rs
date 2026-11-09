//! Debug Capture System for LLM Session Diagnostics
//!
//! Provides comprehensive capture of LLM API communication, tool executions,
//! and application logs for debugging agent issues.
//!
//! ACDD Work Unit: CLI-022
//! Port of rust's debug-capture.ts to Rust.

mod capture;
mod error;
mod manager;
mod mutex;
mod session_lifecycle;
mod summary;
mod types;

pub use error::DebugCaptureError;
pub use manager::DebugCaptureManager;
pub use mutex::PoisonRecoveryMutex;
pub use types::{CaptureOptions, DebugCommandResult, DebugEvent, DebugEventType, SessionMetadata};

use std::sync::{Arc, OnceLock};

// Global singleton instance with poison recovery
static DEBUG_CAPTURE_MANAGER: OnceLock<Arc<PoisonRecoveryMutex<DebugCaptureManager>>> =
    OnceLock::new();

/// Get the singleton debug capture manager instance
///
/// This implementation handles mutex poisoning gracefully by clearing the poison
/// and continuing. For debug capture, this is acceptable since the worst case
/// is missing some debug events.
#[allow(clippy::expect_used)]
pub fn get_debug_capture_manager(
) -> Result<Arc<PoisonRecoveryMutex<DebugCaptureManager>>, DebugCaptureError> {
    // Note: get_or_init requires infallible initialization. If the data directory
    // is not set, this is a startup issue and panic is appropriate.
    let manager = DEBUG_CAPTURE_MANAGER.get_or_init(|| {
        let mgr = DebugCaptureManager::new().expect("Failed to create DebugCaptureManager");
        Arc::new(PoisonRecoveryMutex::new(mgr))
    });

    Ok(manager.clone())
}

/// Handle the /debug command to toggle debug capture
pub fn handle_debug_command() -> DebugCommandResult {
    handle_debug_command_with_dir(None)
}

/// Capture a debug event if debug capture is enabled
///
/// This is a convenience function that handles the boilerplate of:
/// 1. Getting the debug capture manager
/// 2. Locking the mutex
/// 3. Checking if capture is enabled
/// 4. Calling capture with the event data
///
/// Failures are silently ignored since debug capture is optional.
pub fn capture_event(event_type: &str, data: serde_json::Value) {
    with_enabled_capture(|manager| manager.capture(event_type, data, None));
}

/// BUG-467: run `action` on the debug capture manager while capture is
/// enabled. A manager or lock failure is logged — before any capture lock
/// is held, so the tracing `DebugCaptureLayer` cannot recurse into it —
/// and `action` does not run.
pub fn with_enabled_capture(action: impl FnOnce(&mut DebugCaptureManager)) {
    let manager_arc = match get_debug_capture_manager() {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(%error, "debug capture is unavailable");
            return;
        }
    };
    let locked = manager_arc.lock();
    match locked {
        Ok(mut manager) => {
            if manager.is_enabled() {
                action(&mut manager);
            }
        }
        Err(error) => tracing::error!(%error, "the debug capture manager lock is unavailable"),
    }
}

/// Increment the turn counter if debug capture is enabled
///
/// This is a convenience function for incrementing the turn counter.
/// Should be called once per user input to track conversation turns.
///
/// Failures are silently ignored since debug capture is optional.
pub fn increment_debug_turn() {
    with_enabled_capture(DebugCaptureManager::increment_turn);
}

/// Handle the /debug command with a custom base directory
///
/// If base_dir is provided, debug files will be written to `{base_dir}/debug/`
pub fn handle_debug_command_with_dir(base_dir: Option<&str>) -> DebugCommandResult {
    use std::path::PathBuf;

    let manager_arc = match get_debug_capture_manager() {
        Ok(m) => m,
        Err(e) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to initialize debug capture: {e}"),
            }
        }
    };

    let mut manager = match manager_arc.lock() {
        Ok(m) => m,
        Err(_) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: "Failed to acquire lock on debug capture manager".to_string(),
            }
        }
    };

    // Set custom directory if provided (before starting capture)
    if let Some(dir) = base_dir {
        manager.set_debug_directory(PathBuf::from(dir));
    }

    if manager.is_enabled() {
        // Turn off
        match manager.stop_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: false,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture stopped. Session saved to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to stop debug capture: {e}"),
            },
        }
    } else {
        // Turn on
        match manager.start_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: true,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture started. Writing to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to start debug capture: {e}"),
            },
        }
    }
}

#[cfg(test)]
mod tests;
