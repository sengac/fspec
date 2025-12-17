use super::message_helpers::{add_assistant_text_message, add_assistant_tool_calls_message};
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use std::io::Write;
use tracing::debug;

pub(super) fn handle_text_chunk(
    text: &str,
    assistant_text: &mut String,
    request_id: Option<&str>,
) -> Result<()> {
    // CLI-022: Capture api.response.chunk event
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

    // CRITICAL: Accumulate for message history (CLI-008)
    assistant_text.push_str(text);

    // Print text immediately for real-time streaming
    // Replace \n with \r\n for proper terminal display in raw mode
    let display_text = text.replace('\n', "\r\n");
    print!("{display_text}");
    std::io::stdout().flush()?;

    Ok(())
}

pub(super) fn handle_tool_call(
    tool_call: &rig::message::ToolCall,
    messages: &mut Vec<rig::message::Message>,
    assistant_text: &mut String,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &mut Option<String>,
) -> Result<()> {
    use rig::message::AssistantContent;

    // CRITICAL: Add assistant text to messages if we have any (CLI-008)
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.clone());
        assistant_text.clear();
    }

    // Display tool name and arguments before execution
    let tool_name = &tool_call.function.name;
    let args = &tool_call.function.arguments;

    // CLI-022: Capture tool.call event
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

    // Store tool name for debug logging
    *last_tool_name = Some(tool_name.clone());

    // CRITICAL: Track tool call for message history (CLI-008)
    tool_calls_buffer.push(AssistantContent::ToolCall(tool_call.clone()));

    // Log full details
    debug!("Tool call: {} with args: {:?}", tool_name, args);

    // Display tool name
    print!("\r\n[Planning to use tool: {tool_name}]");

    // Display arguments
    if let Some(obj) = args.as_object() {
        if !obj.is_empty() {
            for (key, value) in obj.iter() {
                // Format value based on type
                let formatted_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Array(_) => format!("{value}"),
                    serde_json::Value::Object(_) => format!("{value}"),
                    serde_json::Value::Null => "null".to_string(),
                };
                print!("\r\n  {key}: {formatted_value}");
            }
        }
    }
    println!("\r\n");
    std::io::stdout().flush()?;

    Ok(())
}

pub(super) fn handle_tool_result(
    tool_result: &rig::message::ToolResult,
    messages: &mut Vec<rig::message::Message>,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &Option<String>,
) -> Result<()> {
    use rig::message::{Message, ToolResultContent, UserContent};
    use rig::OneOrMany;

    // CRITICAL: Add buffered tool calls as assistant message (CLI-008)
    // This happens ONCE before the first tool result
    if !tool_calls_buffer.is_empty() {
        add_assistant_tool_calls_message(messages, tool_calls_buffer.clone())?;
        tool_calls_buffer.clear();
    }

    // CRITICAL: Add tool result to message history (CLI-008)
    let tool_result_clone = tool_result.clone();
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::ToolResult(tool_result_clone)),
    });

    // Display tool result with preview (similar to codelet pattern)
    debug!("Tool result received: {:?}", tool_result);

    // CLI-022: Capture tool.result event
    // Determine success from result content (check for error indicators)
    let is_error = tool_result.content.clone().into_iter().any(|c| match c {
        ToolResultContent::Text(t) => t.text.contains("Error:") || t.text.contains("error:"),
        _ => false,
    });

    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                let event_type = if is_error {
                    "tool.error"
                } else {
                    "tool.result"
                };
                manager.capture(
                    event_type,
                    serde_json::json!({
                        "toolName": last_tool_name.as_deref().unwrap_or("unknown"),
                        "toolId": tool_result.id,
                        "success": !is_error,
                    }),
                    None,
                );
            }
        }
    }

    // Extract text content from tool result by iterating over OneOrMany
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

    // Truncate result if too long (like codelet does at 500 chars)
    const MAX_PREVIEW_LENGTH: usize = 500;
    let preview = if result_text.len() > MAX_PREVIEW_LENGTH {
        format!("{}...", &result_text[..MAX_PREVIEW_LENGTH])
    } else {
        result_text
    };

    // Display with proper formatting for raw mode
    // Indent each line by 2 spaces and replace \n with \r\n
    let indented_lines: Vec<String> = preview.lines().map(|line| format!("  {line}")).collect();
    let formatted_preview = indented_lines.join("\r\n");

    print!("\r\n[Tool result preview]\r\n-------\r\n{formatted_preview}\r\n-------\r\n");
    std::io::stdout().flush()?;

    Ok(())
}

pub(super) fn handle_final_response(
    assistant_text: &str,
    messages: &mut Vec<rig::message::Message>,
) -> Result<()> {
    // CRITICAL: Add final assistant text to message history (CLI-008)
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.to_owned());
    }

    Ok(())
}
