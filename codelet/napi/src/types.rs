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
    /// Tokens per second (EMA-smoothed, calculated in Rust)
    pub tokens_per_second: Option<f64>,
    /// Cumulative billed input tokens (sum of all API calls)
    pub cumulative_billed_input: Option<u32>,
    /// Cumulative billed output tokens (sum of all API calls)
    pub cumulative_billed_output: Option<u32>,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
            tokens_per_second: None,
            cumulative_billed_input: Some(0),
            cumulative_billed_output: Some(0),
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
    ContextFillUpdate,
    Done,
    Error,
}

/// Context window fill information (TUI-033)
/// Sent with each token update to show context window usage
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFillInfo {
    /// Fill percentage (0-100+, can exceed 100 near compaction)
    pub fill_percentage: u32,
    /// Effective tokens (after cache discount) - using f64 for NAPI compatibility
    pub effective_tokens: f64,
    /// Compaction threshold (usable context after output reservation) - using f64 for NAPI compatibility
    pub threshold: f64,
    /// Provider's context window size - using f64 for NAPI compatibility
    pub context_window: f64,
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
    pub context_fill: Option<ContextFillInfo>,
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
            context_fill: None,
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
            context_fill: None,
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
            context_fill: None,
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
            context_fill: None,
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
            context_fill: None,
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
            context_fill: None,
            error: None,
        }
    }

    /// Context fill percentage update (TUI-033)
    pub fn context_fill_update(info: ContextFillInfo) -> Self {
        Self {
            chunk_type: "ContextFillUpdate".to_string(),
            text: None,
            tool_call: None,
            tool_result: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: Some(info),
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
            context_fill: None,
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
            context_fill: None,
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

/// Compaction result (NAPI-005)
/// Returned by compact() with metrics about the compaction operation
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Original token count before compaction
    pub original_tokens: u32,
    /// Token count after compaction
    pub compacted_tokens: u32,
    /// Compression ratio as percentage (0-100)
    pub compression_ratio: f64,
    /// Number of turns summarized
    pub turns_summarized: u32,
    /// Number of turns kept
    pub turns_kept: u32,
}
