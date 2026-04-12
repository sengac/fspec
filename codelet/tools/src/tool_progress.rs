//! Tool progress callback registry (TOOL-011, BUG-126)
//!
//! Provides a per-session callback registry for streaming tool execution progress
//! to the UI. This allows tools (like BashTool) to emit real-time output without
//! direct coupling to the StreamOutput trait.
//!
//! # Session Isolation (BUG-126)
//!
//! Callbacks are keyed by `session_id: Uuid` via [`SessionRegistry`] so that
//! concurrent sessions never leak tool progress to each other.
//!
//! # Usage
//!
//! 1. Stream loop registers a callback before agent execution:
//!    ```ignore
//!    set_tool_progress_callback(session_id, Some(Arc::new(|chunk, is_stderr| {
//!        output.emit_tool_progress("", "bash", chunk, is_stderr);
//!    })));
//!    ```
//!
//! 2. BashTool calls emit_tool_progress during execution:
//!    ```ignore
//!    emit_tool_progress(session_id, "line 1\n", false);  // stdout
//!    emit_tool_progress(session_id, "error\n", true);    // stderr
//!    ```
//!
//! 3. Stream loop clears the callback after agent execution:
//!    ```ignore
//!    set_tool_progress_callback(session_id, None);
//!    ```

use std::sync::Arc;

use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::session_registry::SessionRegistry;

/// Callback type for tool progress events.
/// Parameters: (output_chunk, is_stderr)
pub type ToolProgressCallback = Arc<dyn Fn(&str, bool) + Send + Sync>;

/// Per-session callback storage (BUG-126: replaced global singleton).
static TOOL_PROGRESS_CALLBACKS: Lazy<SessionRegistry<ToolProgressCallback>> =
    Lazy::new(SessionRegistry::new);

/// Register or clear a callback for tool progress events for a specific session.
///
/// Call with `Some(callback)` before starting agent execution.
/// Call with `None` after agent execution completes.
///
/// # Thread Safety
/// Multiple sessions can register/clear concurrently without affecting each other.
pub fn set_tool_progress_callback(session_id: Uuid, callback: Option<ToolProgressCallback>) {
    TOOL_PROGRESS_CALLBACKS.set(session_id, callback);
}

/// Emit a tool progress event for a specific session.
///
/// If a callback is registered for the given session_id, calls it with the
/// progress information. If no callback is registered, this is a no-op.
///
/// # Arguments
/// * `session_id`   - The session to emit progress for
/// * `output_chunk` - New output text since last progress event
/// * `is_stderr`    - Whether this output is from stderr (styled as error)
pub fn emit_tool_progress(session_id: Uuid, output_chunk: &str, is_stderr: bool) {
    TOOL_PROGRESS_CALLBACKS.with(&session_id, |callback| {
        callback(output_chunk, is_stderr);
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_emit_with_no_callback_is_noop() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();

        set_tool_progress_callback(session_id, None);
        // Should not panic
        emit_tool_progress(session_id, "output", false);
    }

    #[test]
    fn test_emit_with_callback() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();

        set_tool_progress_callback(session_id, None);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_tool_progress_callback(session_id, Some(Arc::new(move |chunk, is_stderr| {
            captured_clone.lock().unwrap().push((chunk.to_string(), is_stderr));
        })));

        emit_tool_progress(session_id, "line 1\n", false);
        emit_tool_progress(session_id, "error\n", true);

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], ("line 1\n".to_string(), false));
        assert_eq!(events[1], ("error\n".to_string(), true));

        set_tool_progress_callback(session_id, None);
    }

    #[test]
    fn test_clear_callback() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();

        set_tool_progress_callback(session_id, None);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_tool_progress_callback(session_id, Some(Arc::new(move |chunk, is_stderr| {
            captured_clone.lock().unwrap().push((chunk.to_string(), is_stderr));
        })));

        emit_tool_progress(session_id, "before clear\n", false);
        set_tool_progress_callback(session_id, None);
        emit_tool_progress(session_id, "after clear\n", false);

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ("before clear\n".to_string(), false));
    }

    #[test]
    fn test_stderr_flag() {
        let _guard = TEST_LOCK.lock().unwrap();
        let session_id = Uuid::new_v4();

        set_tool_progress_callback(session_id, None);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_tool_progress_callback(session_id, Some(Arc::new(move |chunk, is_stderr| {
            captured_clone.lock().unwrap().push((chunk.to_string(), is_stderr));
        })));

        emit_tool_progress(session_id, "stdout line\n", false);
        emit_tool_progress(session_id, "stderr line\n", true);
        emit_tool_progress(session_id, "more stdout\n", false);

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert!(!events[0].1); // stdout
        assert!(events[1].1);  // stderr
        assert!(!events[2].1); // stdout

        set_tool_progress_callback(session_id, None);
    }
}
