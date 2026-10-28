//! Stream event handlers with output abstraction
//!
//! Handles stream events (text, tool calls, tool results) with:
//! - Message history management (shared between CLI and NAPI)
//! - Debug capture (shared)
//! - Output rendering via StreamOutput trait (polymorphic)

use super::message_helpers::{add_assistant_text_message, add_assistant_tool_calls_message};
use super::output::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use tracing::debug;

/// Detect if a tool result indicates an error (public for annotation detection).
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
pub(super) fn detect_tool_error(raw_result: &str) -> bool {
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
        "Command failed with exit code", // Bash execution failure
        "Operation timed out",           // Timeout errors
        "Token limit exceeded:",         // Token limit errors
        "Invalid pattern:",              // Pattern errors (grep, glob)
        "Unsupported language:",         // AstGrep language errors
        "Tool not found:",               // ToolSet errors
        "Toolset error:",                // ToolServerError wrapper
        "ToolCallError:",                // ToolSetError wrapper
        "ToolServerError:",              // RequestError wrapper
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

    debug!(%tool_name, args = ?args, "Tool call");

    // Emit via output (polymorphic)
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: tool_call.id.clone(),
        call_id: tool_call.call_id.clone(),
        name: tool_name.clone(),
        args: args.clone(),
    }));

    output
        .prepare_tool_dispatch(&tool_call.id)
        .map_err(anyhow::Error::msg)?;

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

    debug!(result = ?tool_result, "Tool result received");

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

    let result_text = result_parts.join("\n");

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
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: tool_result.id.clone(),
        call_id: tool_result.call_id.clone(),
        content: result_text,
        is_error,
    }));

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
include!("stream_handler_tests.rs");
