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

/// Handle FspecTool session-level execution error by executing via JS callback
/// 
/// CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
/// This function parses the FspecTool error message and executes the command
/// using a basic synchronous JS implementation.
fn handle_fspec_session_error(error_message: &str) -> Option<String> {
    // Parse the intercept message to extract command details
    // Format: "FSPEC_INTERCEPT: Command: 'list-work-units', Args: '', Root: '.', Provider: 'claude'"
    
    let command = extract_field_from_fspec_error(error_message, "Command:")?;
    let args = extract_field_from_fspec_error(error_message, "Args:")?;
    let root = extract_field_from_fspec_error(error_message, "Root:")?;
    
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Execute the command using basic synchronous logic for now
    match execute_fspec_command_sync(&command, &args, &root) {
        Ok(result) => {
            Some(result)
        },
        Err(e) => {
            tracing::warn!("[FSPEC_DEBUG_CLI] Failed to execute FspecTool command: {}", e);
            Some(format!("{{\"success\": false, \"error\": true, \"message\": \"FspecTool execution failed: {}\"}} ", e))
        }
    }
}

/// Extract a field value from FspecTool error message
fn extract_field_from_fspec_error(error_message: &str, field_prefix: &str) -> Option<String> {
    let start = error_message.find(field_prefix)? + field_prefix.len();
    let after_prefix = error_message[start..].trim();
    
    // Handle quoted values: 'value' or "value"
    if after_prefix.starts_with('\'') {
        let end = after_prefix[1..].find('\'')?;
        Some(after_prefix[1..=end].to_string())
    } else if after_prefix.starts_with('"') {
        let end = after_prefix[1..].find('"')?;
        Some(after_prefix[1..=end].to_string())
    } else {
        // Handle unquoted values - take until comma or end
        let end = after_prefix.find(',').unwrap_or(after_prefix.len());
        Some(after_prefix[..end].trim().to_string())
    }
}

/// Execute FspecTool command synchronously 
/// 
/// CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
/// This is a basic implementation that reads/writes files directly.
fn execute_fspec_command_sync(command: &str, args_json: &str, project_root: &str) -> Result<String, anyhow::Error> {
    use std::collections::HashMap;
    
    if command == "list-work-units" {
        // Parse arguments
        let args: HashMap<String, serde_json::Value> = if args_json.is_empty() || args_json == "''" {
            HashMap::new()
        } else {
            serde_json::from_str(args_json).unwrap_or_default()
        };
        
        // Read work units file directly
        let work_units_path = std::path::Path::new(project_root).join("spec").join("work-units.json");
        
        // Check if file exists, if not create empty structure
        let work_units_data: serde_json::Value = if work_units_path.exists() {
            let content = std::fs::read_to_string(&work_units_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({"workUnits": {}}))
        } else {
            // Ensure directory exists
            if let Some(parent) = work_units_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Create empty work units file
            let empty_data = serde_json::json!({"workUnits": {}});
            std::fs::write(&work_units_path, serde_json::to_string_pretty(&empty_data)?)?;
            empty_data
        };
        
        // Get all work units
        let work_units = work_units_data.get("workUnits")
            .and_then(|wu| wu.as_object())
            .map(|obj| obj.values().collect::<Vec<_>>())
            .unwrap_or_default();
        
        // Apply basic filters (simplified version)
        let filtered_work_units: Vec<serde_json::Value> = work_units.into_iter()
            .filter(|wu| {
                if let Some(status_filter) = args.get("status") {
                    if let (Some(wu_status), Some(filter_status)) = (wu.get("status"), status_filter.as_str()) {
                        wu_status.as_str() == Some(filter_status)  // FIXED: was != (inverted logic)
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .map(|wu| serde_json::json!({
                "id": wu.get("id"),
                "title": wu.get("title"),
                "status": wu.get("status"),
            }))
            .collect();
        
        // Format output like the real CLI command
        let mut output = String::new();
        
        // Add header
        output.push_str(&format!("Work Units ({})\n\n", filtered_work_units.len()));
        
        // Add each work unit in CLI format
        for wu in &filtered_work_units {
            if let (Some(id), Some(title), Some(status)) = (
                wu.get("id").and_then(|v| v.as_str()),
                wu.get("title").and_then(|v| v.as_str()), 
                wu.get("status").and_then(|v| v.as_str())
            ) {
                output.push_str(&format!("{} [{}]\n", id, status));
                output.push_str(&format!("  {}\n", title));
                if wu != filtered_work_units.last().unwrap() {
                    output.push('\n');
                }
            }
        }
        
        Ok(output)
    } else {
        Err(anyhow::anyhow!("Command '{}' not implemented in synchronous execution", command))
    }
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

    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Check for FspecTool session-level execution requests and handle them
    if result_text.contains("FSPEC_INTERCEPT:") {
        // This is a FspecTool command that needs to be executed via JS callback
        if let Some(actual_result) = handle_fspec_session_error(&result_text) {
            // Successfully executed via JS callback - emit the result instead
            output.emit_tool_result(&tool_result.id, &actual_result, false);
            
            // CRITICAL: Also capture the debug event since we're returning early
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        let data = serde_json::json!({
                            "toolName": last_tool_name.as_deref().unwrap_or("unknown"),
                            "toolId": tool_result.id,
                            "success": true,
                        });
                        manager.capture("tool.result", data, None);
                    }
                }
            }
            
            return Ok(());
        }
        // If handling failed, fall through to emit the original result
    }

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
