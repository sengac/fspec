//! Metadata extraction from StoredMessage metadata HashMap.
//!
//! Extracts tool use and tool result information from the serialized
//! MessageEnvelope metadata stored on StoredMessage instances.

use serde_json::Value;
use std::collections::HashMap;

/// Information about a tool use extracted from assistant messages.
#[derive(Debug, Clone)]
pub struct ToolUseInfo {
    /// Tool name (e.g., "Read", "Write", "Edit", "Bash", "Grep")
    pub name: String,
    /// Tool input parameters
    pub input: Value,
}

/// Extracted info from a tool result block in user message metadata.
#[derive(Debug)]
pub struct ToolResultInfo {
    pub tool_use_id: String,
    pub is_image: bool,
    pub is_error: bool,
}

/// Extract tool use blocks from assistant message metadata.
/// Returns vec of (tool_use_id, ToolUseInfo).
pub fn extract_tool_uses(metadata: &HashMap<String, Value>) -> Vec<(String, ToolUseInfo)> {
    let mut result = Vec::new();

    let message = match metadata.get("message") {
        Some(m) => m,
        None => return result,
    };

    let content_arr = match message.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return result,
    };

    for block in content_arr {
        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
            let id = block
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = block
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let input = block.get("input").cloned().unwrap_or(Value::Null);

            if !id.is_empty() && !name.is_empty() {
                result.push((id, ToolUseInfo { name, input }));
            }
        }
    }

    result
}

/// Extract tool result info from user message metadata.
pub fn extract_tool_result_info(metadata: &HashMap<String, Value>) -> Option<ToolResultInfo> {
    let message = metadata.get("message")?;
    let content_arr = message.get("content")?.as_array()?;

    for block in content_arr {
        if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
            let tool_use_id = block
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let is_image = block
                .get("tool_use_result")
                .and_then(|r| r.get("isImage"))
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let is_error = block
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if !tool_use_id.is_empty() {
                return Some(ToolResultInfo {
                    tool_use_id,
                    is_image,
                    is_error,
                });
            }
        }
    }

    None
}
