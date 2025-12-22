//! Type definitions for NAPI bindings
//!
//! These types are exposed to JavaScript/TypeScript.

use serde::{Deserialize, Serialize};

/// Token usage tracking information
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTracker {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
        }
    }
}

/// Debug command result (AGENT-021)
/// Returned by toggleDebug() to indicate debug capture state
#[napi(object)]
#[derive(Debug, Clone)]
pub struct DebugCommandResult {
    /// Whether debug capture is now enabled
    pub enabled: bool,
    /// Path to the debug session file (if available)
    pub session_file: Option<String>,
    /// Human-readable message about the result
    pub message: String,
}

/// Tool call information
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: String, // JSON string of input
}

/// Tool result information
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

/// Stream chunk types for streaming responses
#[napi(string_enum)]
#[derive(Debug, PartialEq, Eq)]
pub enum ChunkType {
    Text,
    ToolCall,
    ToolResult,
    Status,
    Interrupted,
    TokenUpdate,
    Done,
    Error,
}

/// A chunk of streaming response
#[napi(object)]
#[derive(Debug, Clone)]
pub struct StreamChunk {
    #[napi(js_name = "type")]
    pub chunk_type: String,
    pub text: Option<String>,
    pub tool_call: Option<ToolCallInfo>,
    pub tool_result: Option<ToolResultInfo>,
    pub status: Option<String>,
    pub queued_inputs: Option<Vec<String>>,
    pub tokens: Option<TokenTracker>,
    pub error: Option<String>,
}

impl StreamChunk {
    pub fn text(text: String) -> Self {
        Self {
            chunk_type: "Text".to_string(),
            text: Some(text),
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            error: None,
        }
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self {
            chunk_type: "ToolCall".to_string(),
            text: None,
            tool_call: Some(info),
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            error: None,
        }
    }

    pub fn tool_result(info: ToolResultInfo) -> Self {
        Self {
            chunk_type: "ToolResult".to_string(),
            text: None,
            tool_call: None,
            tool_result: Some(info),
            status: None,
            queued_inputs: None,
            tokens: None,
            error: None,
        }
    }

    pub fn status(message: String) -> Self {
        Self {
            chunk_type: "Status".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: Some(message),
            queued_inputs: None,
            tokens: None,
            error: None,
        }
    }

    pub fn interrupted(queued_inputs: Vec<String>) -> Self {
        Self {
            chunk_type: "Interrupted".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: Some(queued_inputs),
            tokens: None,
            error: None,
        }
    }

    pub fn token_update(tokens: TokenTracker) -> Self {
        Self {
            chunk_type: "TokenUpdate".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: Some(tokens),
            error: None,
        }
    }

    pub fn done() -> Self {
        Self {
            chunk_type: "Done".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            chunk_type: "Error".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            error: Some(message),
        }
    }
}

/// Message role enum
#[napi(string_enum)]
#[derive(Debug, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A conversation message (simplified for JS)
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}
