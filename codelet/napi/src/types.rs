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

/// Tool execution progress information (TOOL-011)
/// Streaming output from bash/shell tools during execution
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressInfo {
    /// Tool call ID this progress is for
    pub tool_call_id: String,
    /// Tool name (e.g., "bash", "run_shell_command")
    pub tool_name: String,
    /// Output chunk (new text since last progress event)
    pub output_chunk: String,
}

/// Stream chunk types for streaming responses (TOOL-010)
#[napi(string_enum)]
#[derive(Debug, PartialEq, Eq)]
pub enum ChunkType {
    Text,
    /// Thinking/reasoning content from extended thinking (TOOL-010)
    Thinking,
    ToolCall,
    ToolResult,
    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    ToolProgress,
    Status,
    Interrupted,
    TokenUpdate,
    ContextFillUpdate,
    Done,
    Error,
    /// User input message (NAPI-009: for resume/attach to restore user messages)
    UserInput,
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

/// A chunk of streaming response (TOOL-010: added thinking field, TOOL-011: added tool_progress)
#[napi(object)]
#[derive(Debug, Clone)]
pub struct StreamChunk {
    #[napi(js_name = "type")]
    pub chunk_type: String,
    pub text: Option<String>,
    /// Thinking/reasoning content from extended thinking (TOOL-010)
    pub thinking: Option<String>,
    pub tool_call: Option<ToolCallInfo>,
    pub tool_result: Option<ToolResultInfo>,
    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    pub tool_progress: Option<ToolProgressInfo>,
    pub status: Option<String>,
    pub queued_inputs: Option<Vec<String>>,
    pub tokens: Option<TokenTracker>,
    pub context_fill: Option<ContextFillInfo>,
    pub error: Option<String>,
    /// Correlation ID for cross-pane selection highlighting (WATCH-011)
    /// Assigned by handle_output() using per-session atomic counter
    pub correlation_id: Option<String>,
    /// IDs of observed parent chunks that triggered this watcher response (WATCH-011)
    /// Only populated on watcher session output chunks
    pub observed_correlation_ids: Option<Vec<String>>,
}

impl StreamChunk {
    pub fn text(text: String) -> Self {
        Self {
            chunk_type: "Text".to_string(),
            text: Some(text),
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Create a thinking/reasoning content chunk (TOOL-010)
    pub fn thinking(thinking: String) -> Self {
        Self {
            chunk_type: "Thinking".to_string(),
            text: None,
            thinking: Some(thinking),
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self {
            chunk_type: "ToolCall".to_string(),
            text: None,
            thinking: None,
            tool_call: Some(info),
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_result(info: ToolResultInfo) -> Self {
        Self {
            chunk_type: "ToolResult".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: Some(info),
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    pub fn tool_progress(info: ToolProgressInfo) -> Self {
        Self {
            chunk_type: "ToolProgress".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: Some(info),
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn status(message: String) -> Self {
        Self {
            chunk_type: "Status".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: Some(message),
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn interrupted(queued_inputs: Vec<String>) -> Self {
        Self {
            chunk_type: "Interrupted".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: Some(queued_inputs),
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn token_update(tokens: TokenTracker) -> Self {
        Self {
            chunk_type: "TokenUpdate".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: Some(tokens),
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Context fill percentage update (TUI-033)
    pub fn context_fill_update(info: ContextFillInfo) -> Self {
        Self {
            chunk_type: "ContextFillUpdate".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: Some(info),
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn done() -> Self {
        Self {
            chunk_type: "Done".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            chunk_type: "Error".to_string(),
            text: None,
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: Some(message),
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// User input message (NAPI-009: for resume/attach to restore user messages)
    pub fn user_input(text: String) -> Self {
        Self {
            chunk_type: "UserInput".to_string(),
            text: Some(text),
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Watcher input message (WATCH-006: for watcher injection into parent session)
    pub fn watcher_input(formatted_message: String) -> Self {
        Self {
            chunk_type: "WatcherInput".to_string(),
            text: Some(formatted_message),
            thinking: None,
            tool_call: None,
            tool_result: None,
            tool_progress: None,
            status: None,
            queued_inputs: None,
            tokens: None,
            context_fill: None,
            error: None,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Set observed correlation IDs for watcher response chunks (WATCH-011)
    pub fn with_observed_correlation_ids(mut self, ids: Vec<String>) -> Self {
        self.observed_correlation_ids = Some(ids);
        self
    }
}

/// Provider configuration for programmatic credential passing (CONFIG-004)
///
/// Used by CodeletSession.newWithCredentials() to pass explicit API keys
/// without reading from environment variables.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiProviderConfig {
    /// Provider ID (e.g., "anthropic", "openai", "gemini")
    pub provider_id: String,
    /// API key for the provider
    pub api_key: Option<String>,
    /// Custom base URL (optional)
    pub base_url: Option<String>,
    /// Whether the provider is enabled
    pub enabled: bool,
    /// Default model (optional)
    pub default_model: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test StreamChunk::user_input creates correct chunk type
    #[test]
    fn test_user_input_chunk_creation() {
        let user_message = "Hello, can you help me with this task?";
        let chunk = StreamChunk::user_input(user_message.to_string());

        assert_eq!(chunk.chunk_type, "UserInput");
        assert_eq!(chunk.text, Some(user_message.to_string()));
        assert!(chunk.thinking.is_none());
        assert!(chunk.tool_call.is_none());
        assert!(chunk.tool_result.is_none());
        assert!(chunk.tool_progress.is_none());
        assert!(chunk.status.is_none());
        assert!(chunk.queued_inputs.is_none());
        assert!(chunk.tokens.is_none());
        assert!(chunk.context_fill.is_none());
        assert!(chunk.error.is_none());
        assert!(chunk.correlation_id.is_none());
        assert!(chunk.observed_correlation_ids.is_none());
    }

    /// Test empty user input is handled correctly
    #[test]
    fn test_empty_user_input_chunk() {
        let chunk = StreamChunk::user_input(String::new());

        assert_eq!(chunk.chunk_type, "UserInput");
        assert_eq!(chunk.text, Some(String::new()));
    }

    /// Test user input with multiline content
    #[test]
    fn test_multiline_user_input_chunk() {
        let multiline_message = "First line\nSecond line\nThird line with code:\n```rust\nfn main() {}\n```";
        let chunk = StreamChunk::user_input(multiline_message.to_string());

        assert_eq!(chunk.chunk_type, "UserInput");
        assert_eq!(chunk.text, Some(multiline_message.to_string()));
        assert!(chunk.text.as_ref().unwrap().contains('\n'));
    }

    /// Test user input with special characters
    #[test]
    fn test_special_characters_in_user_input() {
        let special_message = "Test with émojis 🎉 and symbols: <>&\"' and unicode: 你好世界";
        let chunk = StreamChunk::user_input(special_message.to_string());

        assert_eq!(chunk.chunk_type, "UserInput");
        assert_eq!(chunk.text, Some(special_message.to_string()));
    }

    /// Test UserInput chunk type is distinct from Text chunk
    #[test]
    fn test_user_input_distinct_from_text() {
        let message = "Same content";
        let user_chunk = StreamChunk::user_input(message.to_string());
        let text_chunk = StreamChunk::text(message.to_string());

        assert_ne!(user_chunk.chunk_type, text_chunk.chunk_type);
        assert_eq!(user_chunk.chunk_type, "UserInput");
        assert_eq!(text_chunk.chunk_type, "Text");
        // Both store content in the text field
        assert_eq!(user_chunk.text, text_chunk.text);
    }
}
