//! Stream event handlers with output abstraction
//!
//! Handles stream events (text, tool calls, tool results) with:
//! - Message history management (shared between CLI and NAPI)
//! - Debug capture (shared)
//! - Output rendering via StreamOutput trait (polymorphic)

use super::message_helpers::{add_assistant_text_message, add_assistant_tool_calls_message};
use super::output::StreamOutput;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use tracing::debug;

/// Detect if a tool result indicates an error.
///
/// Error detection works in two ways:
///
/// 1. **JSON responses from tool facades**: Check for `{"success": false}` or `{"error": "..."}`.
///    Tool facades (FileToolFacadeWrapper, BashToolFacadeWrapper, etc.) return JSON
///    with a structured format.
///
/// 2. **Plain-text error messages**: Detect patterns from tool error messages.
///    When a tool returns `Err(ToolError::...)`, the rig framework converts it to
///    a plain string. We detect common error patterns:
///    - "Command failed with exit code N" (from bash)
///    - "Operation timed out" (from timeouts)
///    - Other error-indicating patterns
///
/// This avoids false positives from file content containing "Error:" or "error:".
fn detect_tool_error(raw_result: &str) -> bool {
    // 1. Try to parse as JSON to check for structured error indicators
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw_result) {
        // Check for {"success": false, ...} pattern used by tool facades
        if let Some(success) = json.get("success").and_then(serde_json::Value::as_bool) {
            return !success;
        }
        // Check for {"error": "...", ...} pattern (error field present with non-null value)
        if let Some(error) = json.get("error") {
            if !error.is_null() && error.as_str().map(|s| !s.is_empty()).unwrap_or(true) {
                return true;
            }
        }
    }

    // 2. Check for plain-text error patterns from ToolError messages
    // These are generated when a tool returns Err(ToolError::...)
    // and the rig framework converts it to a string.
    //
    // IMPORTANT: We check for SPECIFIC patterns, not generic "error" strings,
    // to avoid false positives from file content.
    let error_patterns = [
        "Command failed with exit code",  // Bash execution failure
        "Operation timed out",             // Timeout errors
        "Token limit exceeded:",           // Token limit errors
        "Invalid pattern:",                // Pattern errors (grep, glob)
        "Unsupported language:",           // AstGrep language errors
        "Tool not found:",                 // ToolSet errors
        "Toolset error:",                  // ToolServerError wrapper (legacy)
        "ToolCallError:",                  // ToolSetError wrapper (legacy)
        "ToolServerError:",                // RequestError wrapper (legacy)
    ];

    for pattern in error_patterns {
        if raw_result.starts_with(pattern) {
            return true;
        }
    }

    false
}

/// Handle text chunk - accumulates text and emits via output
pub(super) fn handle_text_chunk<O: StreamOutput>(
    text: &str,
    assistant_text: &mut String,
    request_id: Option<&str>,
    output: &O,
) -> Result<()> {
    // CLI-022: Capture api.response.chunk event (shared)
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.response.chunk",
                    serde_json::json!({
                        "chunkLength": text.len(),
                    }),
                    request_id.map(|id| codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(id.to_string()),
                    }),
                );
            }
        }
    }

    // CRITICAL: Accumulate for message history (CLI-008) - shared
    assistant_text.push_str(text);

    // Emit via output (polymorphic - CLI prints, NAPI sends callback)
    output.emit_text(text);

    Ok(())
}

/// Handle tool call - flushes text, buffers tool call, emits via output
pub(super) fn handle_tool_call<O: StreamOutput>(
    tool_call: &rig::message::ToolCall,
    messages: &mut Vec<rig::message::Message>,
    assistant_text: &mut String,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &mut Option<String>,
    output: &O,
) -> Result<()> {
    use rig::message::AssistantContent;

    // CRITICAL: Add assistant text to messages if we have any (CLI-008) - shared
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.clone());
        assistant_text.clear();
    }

    let tool_name = &tool_call.function.name;
    let args = &tool_call.function.arguments;

    // CLI-022: Capture tool.call event (shared)
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "tool.call",
                    serde_json::json!({
                        "toolName": tool_name,
                        "toolId": tool_call.id,
                        "arguments": args,
                    }),
                    None,
                );
            }
        }
    }

    // Store tool name for debug logging - shared
    *last_tool_name = Some(tool_name.clone());

    // CRITICAL: Track tool call for message history (CLI-008) - shared
    tool_calls_buffer.push(AssistantContent::ToolCall(tool_call.clone()));

    debug!("Tool call: {} with args: {:?}", tool_name, args);

    // Emit via output (polymorphic)
    output.emit_tool_call(&tool_call.id, tool_name, args);

    Ok(())
}

/// Handle tool result - adds to messages, emits via output
pub(super) fn handle_tool_result<O: StreamOutput>(
    tool_result: &rig::message::ToolResult,
    messages: &mut Vec<rig::message::Message>,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &Option<String>,
    output: &O,
) -> Result<()> {
    use rig::message::{Message, ToolResultContent, UserContent};
    use rig::OneOrMany;

    // CRITICAL: Add buffered tool calls as assistant message (CLI-008) - shared
    if !tool_calls_buffer.is_empty() {
        add_assistant_tool_calls_message(messages, tool_calls_buffer.clone())?;
        tool_calls_buffer.clear();
    }

    // CRITICAL: Add tool result to message history (CLI-008) - shared
    let tool_result_clone = tool_result.clone();
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::ToolResult(tool_result_clone)),
    });

    debug!("Tool result received: {:?}", tool_result);

    // Extract content for debug capture and output - shared
    let result_parts: Vec<String> = tool_result
        .content
        .clone()
        .into_iter()
        .map(|content| match content {
            ToolResultContent::Text(text) => text.text,
            ToolResultContent::Image(_) => "[Image]".to_string(),
        })
        .collect();

    let mut result_text = result_parts.join("\n");

    // Strip surrounding quotes if present (JSON-escaped string)
    if result_text.starts_with('"') && result_text.ends_with('"') {
        result_text = result_text[1..result_text.len() - 1].to_string();
    }

    // Unescape common JSON escape sequences
    result_text = result_text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");

    // Determine if error using structured data from tool results
    // Tool facades return JSON with {"success": bool, "error": Option<String>}
    // We check for success=false in the JSON structure rather than string matching
    // which would cause false positives when file content contains "Error:" or "error:"
    let is_error = detect_tool_error(&result_parts.join("\n"));

    // CLI-022: Capture tool.result event (shared)
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                let event_type = if is_error {
                    "tool.error"
                } else {
                    "tool.result"
                };
                // Include error message in tool.error events for debugging
                let mut data = serde_json::json!({
                    "toolName": last_tool_name.as_deref().unwrap_or("unknown"),
                    "toolId": tool_result.id,
                    "success": !is_error,
                });
                if is_error {
                    // Truncate error message to avoid bloating the debug log
                    let error_preview: String = result_text.chars().take(500).collect();
                    data["error"] = serde_json::json!(error_preview);
                }
                manager.capture(event_type, data, None);
            }
        }
    }

    // Emit via output (polymorphic)
    output.emit_tool_result(&tool_result.id, &result_text, is_error);

    Ok(())
}

/// Handle final response - adds final text to message history
pub(super) fn handle_final_response(
    assistant_text: &str,
    messages: &mut Vec<rig::message::Message>,
) -> Result<()> {
    // CRITICAL: Add final assistant text to message history (CLI-008) - shared
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.to_owned());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== JSON-based error detection tests ==========

    #[test]
    fn test_detect_tool_error_success_true() {
        // Tool facade returns success: true - should NOT be an error
        let result = r#"{"success":true,"content":"file contents here"}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_success_false() {
        // Tool facade returns success: false - IS an error
        let result = r#"{"success":false,"error":"File not found"}"#;
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_error_field() {
        // Error field present with non-empty value - IS an error
        let result = r#"{"error":"Something went wrong"}"#;
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_null_error() {
        // Error field present but null - NOT an error
        let result = r#"{"success":true,"error":null}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_empty_error() {
        // Error field present but empty string - NOT an error
        let result = r#"{"success":true,"error":""}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_json_content_with_error_word() {
        // JSON with content containing "Error:" but success: true
        let result = r#"{"success":true,"content":"Error: this is just example text"}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_json_without_success_or_error() {
        // JSON without success or error fields - NOT an error
        let result = r#"{"data":"some value","count":42}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_nested_json() {
        // Nested JSON - only top-level success/error counts
        let result = r#"{"success":true,"data":{"error":"nested error doesn't count"}}"#;
        assert!(!detect_tool_error(result));
    }

    // ========== Plain-text error pattern detection tests ==========

    #[test]
    fn test_detect_tool_error_bash_exit_code() {
        // Bash command failure - IS an error
        let result = "Command failed with exit code 1\nerror: unknown command 'bootstrap2'\n(Did you mean bootstrap?)";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_timeout() {
        // Timeout error - IS an error
        let result = "Operation timed out after 30 seconds";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_token_limit() {
        // Token limit error - IS an error
        let result = "Token limit exceeded: /path/to/file has ~50000 tokens (limit: 25000)";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_invalid_pattern() {
        // Pattern error - IS an error
        let result = "Invalid pattern: [unclosed bracket";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_unsupported_language() {
        // Language error - IS an error
        let result = "Unsupported language: brainfuck";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_not_found() {
        // Tool not found error - IS an error
        let result = "Tool not found: nonexistent_tool";
        assert!(detect_tool_error(result));
    }

    // ========== False positive prevention tests ==========

    #[test]
    fn test_detect_tool_error_plain_text_with_error_word() {
        // Plain text containing "Error:" - should NOT be marked as error
        // This was the bug: file content containing "error:" was falsely flagged
        let result = r#"1: # Agent Development Guidelines
2: This document describes error handling.
3: Use ToolError for tool failures.
4: Example: console.error('Error: something failed');"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_non_json_normal_output() {
        // Normal plain text output - NOT an error
        let result = "This is plain text output from a tool";
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_command_in_middle() {
        // Error pattern appearing in middle of content - NOT an error
        // Only patterns at START of message indicate actual errors
        let result = "The message 'Command failed with exit code 1' was logged";
        assert!(!detect_tool_error(result));
    }

    // ========== Legacy error wrapper pattern tests ==========

    #[test]
    fn test_detect_tool_error_toolset_error_wrapper() {
        // Legacy ToolServerError wrapper - IS an error
        let result = "Toolset error: Command failed with exit code 1\nerror: unknown command";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_call_error_wrapper() {
        // Legacy ToolSetError wrapper - IS an error
        let result = "ToolCallError: File not found: /path/to/file";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_server_error_wrapper() {
        // Legacy RequestError wrapper - IS an error
        let result = "ToolServerError: Connection refused";
        assert!(detect_tool_error(result));
    }

    // ========== Stderr marker tests (integration with bash output) ==========

    #[test]
    fn test_detect_tool_error_with_stderr_markers_success() {
        // Successful command with stderr markers should NOT be flagged as error
        // (The markers are for UI coloring, not error status)
        let result = "stdout output\n⚠stderr⚠warning: deprecated function";
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_stderr_markers_and_error() {
        // Failed command has error prefix + stderr markers
        let result = "Command failed with exit code 1\n⚠stderr⚠error: file not found";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_stderr_marker_only() {
        // Only stderr markers (no actual error) should NOT be flagged
        let result = "⚠stderr⚠this is just a warning";
        assert!(!detect_tool_error(result));
    }
}
