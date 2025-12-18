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
    pub error: Option<String>,
}

impl StreamChunk {
    pub fn text(text: String) -> Self {
        Self {
            chunk_type: "Text".to_string(),
            text: Some(text),
            tool_call: None,
            tool_result: None,
            error: None,
        }
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self {
            chunk_type: "ToolCall".to_string(),
            text: None,
            tool_call: Some(info),
            tool_result: None,
            error: None,
        }
    }

    pub fn tool_result(info: ToolResultInfo) -> Self {
        Self {
            chunk_type: "ToolResult".to_string(),
            text: None,
            tool_call: None,
            tool_result: Some(info),
            error: None,
        }
    }

    pub fn done() -> Self {
        Self {
            chunk_type: "Done".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            chunk_type: "Error".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
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
