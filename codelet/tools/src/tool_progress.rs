//! Tool progress callback registry (TOOL-011)
//!
//! Provides a global callback registry for streaming tool execution progress to the UI.
//! This allows tools (like BashTool) to emit real-time output without direct coupling
//! to the StreamOutput trait.
//!
//! # Usage
//!
//! 1. Stream loop registers a callback before agent execution:
//!    ```ignore
//!    set_tool_progress_callback(Some(Arc::new(|chunk| {
//!        output.emit_tool_progress("", "bash", chunk);
//!    })));
//!    ```
//!
//! 2. BashTool calls emit_tool_progress during execution:
//!    ```ignore
//!    emit_tool_progress("line 1\n");
//!    ```
//!
//! 3. Stream loop clears the callback after agent execution:
//!    ```ignore
//!    set_tool_progress_callback(None);
//!    ```

use std::sync::{Arc, RwLock};

/// Callback type for tool progress events
/// Parameter: output_chunk (new text since last progress event)
pub type ToolProgressCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Global callback storage
static TOOL_PROGRESS_CALLBACK: RwLock<Option<ToolProgressCallback>> = RwLock::new(None);

/// Register a callback for tool progress events
///
/// Call with `Some(callback)` before starting agent execution.
/// Call with `None` after agent execution completes.
///
/// # Thread Safety
/// Uses RwLock for thread-safe access. Multiple threads can emit progress
/// concurrently while a callback is registered.
pub fn set_tool_progress_callback(callback: Option<ToolProgressCallback>) {
    if let Ok(mut guard) = TOOL_PROGRESS_CALLBACK.write() {
        *guard = callback;
    }
}

/// Emit a tool progress event
///
/// If a callback is registered, calls it with the progress information.
/// If no callback is registered, this is a no-op.
///
/// # Arguments
/// * `output_chunk` - New output text since last progress event
pub fn emit_tool_progress(output_chunk: &str) {
    if let Ok(guard) = TOOL_PROGRESS_CALLBACK.read() {
        if let Some(callback) = guard.as_ref() {
            callback(output_chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Test lock to ensure tests run sequentially (global callback is shared)
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_emit_with_no_callback_is_noop() {
        let _guard = TEST_LOCK.lock().unwrap();

        // Clear any existing callback
        set_tool_progress_callback(None);

        // Should not panic
        emit_tool_progress("output");
    }

    #[test]
    fn test_emit_with_callback() {
        let _guard = TEST_LOCK.lock().unwrap();

        // Clear any existing state first
        set_tool_progress_callback(None);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_tool_progress_callback(Some(Arc::new(move |chunk| {
            captured_clone.lock().unwrap().push(chunk.to_string());
        })));

        emit_tool_progress("line 1\n");
        emit_tool_progress("line 2\n");

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "line 1\n");
        assert_eq!(events[1], "line 2\n");

        // Clean up
        set_tool_progress_callback(None);
    }

    #[test]
    fn test_clear_callback() {
        let _guard = TEST_LOCK.lock().unwrap();

        // Clear any existing state first
        set_tool_progress_callback(None);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = captured.clone();

        set_tool_progress_callback(Some(Arc::new(move |chunk| {
            captured_clone.lock().unwrap().push(chunk.to_string());
        })));

        emit_tool_progress("before clear\n");

        // Clear callback
        set_tool_progress_callback(None);

        // This should be a no-op
        emit_tool_progress("after clear\n");

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "before clear\n");
    }
}
