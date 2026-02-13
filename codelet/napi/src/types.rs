//! Type definitions for NAPI bindings
//!
//! These types are exposed to JavaScript/TypeScript.

use serde::{Deserialize, Serialize};

/// PERF-002: Progress information for compaction process
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionProgress {
    /// Current compaction phase (e.g., "Analyzing anchors", "Generating summary")
    pub phase: String,
    /// Current progress count (e.g., current turn being processed)
    pub current: u32,
    /// Total items to process (e.g., total turns to analyze)
    pub total: u32,
}

/// TUI-056: Anchor point types for NAPI
#[napi(string_enum)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NapiAnchorType {
    ErrorResolution,
    TaskCompletion,
    UserCheckpoint,
    FeatureMilestone,
}

/// TUI-056: Anchor point for NAPI
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiAnchorPoint {
    /// Index of turn in conversation history
    pub turn_index: u32,
    /// Type of anchor
    pub anchor_type: NapiAnchorType,
    /// Weight for preservation (0.7-0.9)
    pub weight: f64,
    /// Detection confidence (0.0-1.0)
    pub confidence: f64,
    /// Human-readable description
    pub description: String,
    /// Timestamp when anchor was created (Unix timestamp in milliseconds)
    pub timestamp: f64,
    /// User message content at this turn (captured at anchor creation time)
    /// None for old anchors that don't have this data
    pub user_message: Option<String>,
    /// Assistant response content at this turn (captured at anchor creation time)
    /// None for old anchors that don't have this data
    pub assistant_response: Option<String>,
    /// Tool calls made in this turn (captured at anchor creation time)
    pub tool_calls: Vec<NapiAnchorToolCall>,
}

/// TUI-057: Tool call info stored with anchor point
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiAnchorToolCall {
    /// Tool name (e.g., "Edit", "Write", "Bash")
    pub tool: String,
    /// Whether the tool call succeeded
    pub success: bool,
}

/// TUI-056: Tool call info for turn details
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiToolCall {
    /// Tool name
    pub tool: String,
    /// Tool parameters as JSON string
    pub parameters: String,
    /// Whether tool call was successful
    pub success: bool,
}

/// TUI-056: File modification info for turn details
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiFileModification {
    /// File path
    pub path: String,
    /// Type of operation
    pub operation: String, // "create" | "edit" | "delete"
    /// Summary of what was changed
    pub summary: String,
}

/// TUI-056: Turn details for NAPI
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapiTurnDetails {
    /// Turn index for reference
    pub turn_index: u32,
    /// User message for this turn
    pub user_message: String,
    /// Assistant response for this turn
    pub assistant_response: String,
    /// Tool calls made during this turn
    pub tool_calls: Vec<NapiToolCall>,
    /// File modifications made during this turn
    pub file_modifications: Vec<NapiFileModification>,
    /// Overall success/failure status of turn
    pub status: String, // "success" | "partial" | "failed"
    /// Brief context about what happened
    pub context: String,
}

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
    /// Whether this output is from stderr (should be styled as error/red)
    pub is_stderr: bool,
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

/// Watcher pending injection information (WATCH-020)
/// Sent when auto_inject=false and watcher detects an [INTERJECT] block
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WatcherPendingInjectionInfo {
    /// Whether this is an urgent injection
    pub urgent: bool,
    /// The message content that would be injected
    pub content: String,
}

/// Work unit information for file watcher updates
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    #[napi(js_name = "workType")]
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
}

/// NAPI-010: Session state for internal state machine tracking
/// NOT for conversation display - use SessionStateChange chunk variant
#[napi(string_enum)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
}

/// NAPI-010: User notification severity levels
#[napi(string_enum)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}

/// NAPI-010: Stream chunk - proper discriminated union
///
/// The type system enforces correct handling in TypeScript via exhaustive switch statements.
/// This replaces the old struct-based StreamChunk that required fragile string parsing.
///
/// Key distinction:
/// - SessionStateChange: INTERNAL state updates, do NOT add to conversation
/// - UserNotification: User-facing messages, DISPLAY in conversation
#[napi(discriminant = "type")]
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content from assistant
    Text {
        text: String,
        /// Correlation ID for cross-pane selection highlighting (WATCH-011)
        #[napi(js_name = "correlationId")]
        correlation_id: Option<String>,
        /// IDs of observed parent chunks that triggered this watcher response (WATCH-011)
        #[napi(js_name = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },

    /// Thinking/reasoning content from extended thinking (TOOL-010)
    Thinking {
        thinking: String,
        #[napi(js_name = "correlationId")]
        correlation_id: Option<String>,
        #[napi(js_name = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },

    /// Tool invocation from assistant
    ToolCall {
        #[napi(js_name = "toolCall")]
        tool_call: ToolCallInfo,
        #[napi(js_name = "correlationId")]
        correlation_id: Option<String>,
        #[napi(js_name = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },

    /// Tool execution result
    ToolResult {
        #[napi(js_name = "toolResult")]
        tool_result: ToolResultInfo,
        #[napi(js_name = "correlationId")]
        correlation_id: Option<String>,
        #[napi(js_name = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },

    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    ToolProgress {
        #[napi(js_name = "toolProgress")]
        tool_progress: ToolProgressInfo,
        #[napi(js_name = "correlationId")]
        correlation_id: Option<String>,
        #[napi(js_name = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },

    /// NAPI-010: Internal session state change - NOT for conversation display
    /// TypeScript should update state machine and UI indicators, but NOT add to conversation
    SessionStateChange {
        state: SessionState,
    },

    /// NAPI-010: User-facing notification - DISPLAY in conversation
    /// For messages that should be visible to the user in the conversation area
    UserNotification {
        message: String,
        severity: NotificationSeverity,
    },

    /// User interrupted agent execution
    Interrupted {
        #[napi(js_name = "queuedInputs")]
        queued_inputs: Vec<String>,
    },

    /// Token usage update
    TokenUpdate {
        tokens: TokenTracker,
    },

    /// Context fill percentage update (TUI-033)
    ContextFillUpdate {
        #[napi(js_name = "contextFill")]
        context_fill: ContextFillInfo,
    },

    /// Stream completed
    Done,

    /// Error occurred
    Error {
        error: String,
    },

    /// User input message (NAPI-009: for resume/attach to restore user messages)
    UserInput {
        text: String,
    },

    /// Watcher input message (WATCH-006: for watcher injection into parent session)
    WatcherInput {
        text: String,
    },

    /// Watcher pending injection - when auto_inject=false (WATCH-020)
    WatcherPendingInjection {
        #[napi(js_name = "watcherPendingInjection")]
        watcher_pending_injection: WatcherPendingInjectionInfo,
    },

    /// UX-002: Compaction completed with structured result data
    /// NOT a string to parse - direct access to compression metrics
    CompactionComplete {
        #[napi(js_name = "compactionResult")]
        compaction_result: CompactionResult,
    },

    /// CODE-009: Fspec command request - sent when LLM invokes FspecTool
    /// TypeScript must intercept this, execute the command, and call session_send_fspec_result()
    FspecCommandRequest {
        #[napi(js_name = "fspecRequest")]
        fspec_request: FspecRequest,
    },

    /// CODE-009: Fspec command result - sent by TypeScript after executing command
    /// This is emitted after session_send_fspec_result() is called
    FspecCommandResult {
        #[napi(js_name = "fspecResult")]
        fspec_result: FspecResult,
    },

    /// Work units updated - emitted by global file watcher when work-units.json changes
    WorkUnitsUpdate {
        #[napi(js_name = "workUnits")]
        work_units: Vec<WorkUnitInfo>,
    },
}

impl StreamChunk {
    pub fn text(text: String) -> Self {
        Self::Text {
            text,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Create a thinking/reasoning content chunk (TOOL-010)
    pub fn thinking(thinking: String) -> Self {
        Self::Thinking {
            thinking,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self::ToolCall {
            tool_call: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_result(info: ToolResultInfo) -> Self {
        Self::ToolResult {
            tool_result: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    pub fn tool_progress(info: ToolProgressInfo) -> Self {
        Self::ToolProgress {
            tool_progress: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// NAPI-010: Create a session state change chunk (internal state, not for conversation)
    pub fn session_state_change(state: SessionState) -> Self {
        Self::SessionStateChange { state }
    }

    /// NAPI-010: Create a user notification chunk (for conversation display)
    pub fn user_notification(message: String, severity: NotificationSeverity) -> Self {
        Self::UserNotification { message, severity }
    }

    pub fn interrupted(queued_inputs: Vec<String>) -> Self {
        Self::Interrupted { queued_inputs }
    }

    pub fn token_update(tokens: TokenTracker) -> Self {
        Self::TokenUpdate { tokens }
    }

    /// Context fill percentage update (TUI-033)
    pub fn context_fill_update(info: ContextFillInfo) -> Self {
        Self::ContextFillUpdate { context_fill: info }
    }

    pub fn done() -> Self {
        Self::Done
    }

    pub fn error(message: String) -> Self {
        Self::Error { error: message }
    }

    /// User input message (NAPI-009: for resume/attach to restore user messages)
    pub fn user_input(text: String) -> Self {
        Self::UserInput { text }
    }

    /// Watcher input message (WATCH-006: for watcher injection into parent session)
    pub fn watcher_input(formatted_message: String) -> Self {
        Self::WatcherInput { text: formatted_message }
    }

    /// Set correlation ID on the chunk (for variants that support it)
    pub fn with_correlation_id(mut self, id: String) -> Self {
        match &mut self {
            Self::Text { correlation_id, .. } => *correlation_id = Some(id),
            Self::Thinking { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolCall { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolResult { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolProgress { correlation_id, .. } => *correlation_id = Some(id),
            // Other variants don't have correlation_id
            _ => {}
        }
        self
    }

    /// Set observed correlation IDs for watcher response chunks (WATCH-011)
    pub fn with_observed_correlation_ids(mut self, ids: Vec<String>) -> Self {
        match &mut self {
            Self::Text { observed_correlation_ids, .. } => *observed_correlation_ids = Some(ids),
            Self::Thinking { observed_correlation_ids, .. } => *observed_correlation_ids = Some(ids),
            Self::ToolCall { observed_correlation_ids, .. } => *observed_correlation_ids = Some(ids),
            Self::ToolResult { observed_correlation_ids, .. } => *observed_correlation_ids = Some(ids),
            Self::ToolProgress { observed_correlation_ids, .. } => *observed_correlation_ids = Some(ids),
            // Other variants don't have observed_correlation_ids
            _ => {}
        }
        self
    }

    /// Watcher pending injection - when auto_inject=false (WATCH-020)
    pub fn watcher_pending_injection(urgent: bool, content: String) -> Self {
        Self::WatcherPendingInjection {
            watcher_pending_injection: WatcherPendingInjectionInfo { urgent, content },
        }
    }

    /// UX-002: Compaction completed with structured result
    pub fn compaction_complete(result: CompactionResult) -> Self {
        Self::CompactionComplete {
            compaction_result: result,
        }
    }

    /// CODE-009: Fspec command request - sent to TypeScript for execution
    pub fn fspec_command_request(request: FspecRequest) -> Self {
        Self::FspecCommandRequest {
            fspec_request: request,
        }
    }

    /// CODE-009: Fspec command result - sent after TypeScript executes command
    pub fn fspec_command_result(result: FspecResult) -> Self {
        Self::FspecCommandResult {
            fspec_result: result,
        }
    }

    /// Work units updated - emitted by global file watcher
    pub fn work_units_update(work_units: Vec<WorkUnitInfo>) -> Self {
        Self::WorkUnitsUpdate { work_units }
    }

    /// Convert StreamChunk to serde_json::Value for bridge relay (BRIDGE-001)
    ///
    /// This manual serialization is needed because StreamChunk uses NAPI's
    /// discriminant-based serialization which doesn't implement serde::Serialize.
    /// The bridge needs to serialize chunks to JSON for WebSocket transmission.
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::json;

        match self {
            Self::Text { text, correlation_id, observed_correlation_ids } => json!({
                "type": "text",
                "text": text,
                "correlationId": correlation_id,
                "observedCorrelationIds": observed_correlation_ids,
            }),
            Self::Thinking { thinking, correlation_id, observed_correlation_ids } => json!({
                "type": "thinking",
                "thinking": thinking,
                "correlationId": correlation_id,
                "observedCorrelationIds": observed_correlation_ids,
            }),
            Self::ToolCall { tool_call, correlation_id, observed_correlation_ids } => json!({
                "type": "toolCall",
                "toolCall": {
                    "id": tool_call.id,
                    "name": tool_call.name,
                    "input": tool_call.input,
                },
                "correlationId": correlation_id,
                "observedCorrelationIds": observed_correlation_ids,
            }),
            Self::ToolResult { tool_result, correlation_id, observed_correlation_ids } => json!({
                "type": "toolResult",
                "toolResult": {
                    "toolCallId": tool_result.tool_call_id,
                    "content": tool_result.content,
                    "isError": tool_result.is_error,
                },
                "correlationId": correlation_id,
                "observedCorrelationIds": observed_correlation_ids,
            }),
            Self::ToolProgress { tool_progress, correlation_id, observed_correlation_ids } => json!({
                "type": "toolProgress",
                "toolProgress": {
                    "toolCallId": tool_progress.tool_call_id,
                    "toolName": tool_progress.tool_name,
                    "outputChunk": tool_progress.output_chunk,
                    "isStderr": tool_progress.is_stderr,
                },
                "correlationId": correlation_id,
                "observedCorrelationIds": observed_correlation_ids,
            }),
            Self::SessionStateChange { state } => json!({
                "type": "sessionStateChange",
                "state": format!("{:?}", state),
            }),
            Self::UserNotification { message, severity } => json!({
                "type": "userNotification",
                "message": message,
                "severity": format!("{:?}", severity),
            }),
            Self::Interrupted { queued_inputs } => json!({
                "type": "interrupted",
                "queuedInputs": queued_inputs,
            }),
            Self::TokenUpdate { tokens } => json!({
                "type": "tokenUpdate",
                "tokens": {
                    "inputTokens": tokens.input_tokens,
                    "outputTokens": tokens.output_tokens,
                    "cacheCreationInputTokens": tokens.cache_creation_input_tokens,
                    "cacheReadInputTokens": tokens.cache_read_input_tokens,
                    "tokensPerSecond": tokens.tokens_per_second,
                },
            }),
            Self::ContextFillUpdate { context_fill } => json!({
                "type": "contextFillUpdate",
                "contextFill": {
                    "fillPercentage": context_fill.fill_percentage,
                    "effectiveTokens": context_fill.effective_tokens,
                    "threshold": context_fill.threshold,
                    "contextWindow": context_fill.context_window,
                },
            }),
            Self::Done => json!({
                "type": "done",
            }),
            Self::Error { error } => json!({
                "type": "error",
                "error": error,
            }),
            Self::UserInput { text } => json!({
                "type": "userInput",
                "text": text,
            }),
            Self::WatcherInput { text } => json!({
                "type": "watcherInput",
                "text": text,
            }),
            Self::WatcherPendingInjection { watcher_pending_injection } => json!({
                "type": "watcherPendingInjection",
                "watcherPendingInjection": {
                    "urgent": watcher_pending_injection.urgent,
                    "content": watcher_pending_injection.content,
                },
            }),
            Self::CompactionComplete { compaction_result } => json!({
                "type": "compactionComplete",
                "compactionResult": {
                    "originalTokens": compaction_result.original_tokens,
                    "compactedTokens": compaction_result.compacted_tokens,
                    "compressionRatio": compaction_result.compression_ratio,
                    "turnsSummarized": compaction_result.turns_summarized,
                    "turnsKept": compaction_result.turns_kept,
                },
            }),
            Self::FspecCommandRequest { fspec_request } => json!({
                "type": "fspecCommandRequest",
                "fspecRequest": {
                    "command": fspec_request.command,
                    "argsJson": fspec_request.args_json,
                    "projectRoot": fspec_request.project_root,
                    "toolCallId": fspec_request.tool_call_id,
                },
            }),
            Self::FspecCommandResult { fspec_result } => json!({
                "type": "fspecCommandResult",
                "fspecResult": {
                    "success": fspec_result.success,
                    "data": fspec_result.data,
                    "error": fspec_result.error,
                    "systemReminder": fspec_result.system_reminder,
                    "toolCallId": fspec_result.tool_call_id,
                },
            }),
            Self::WorkUnitsUpdate { work_units } => json!({
                "type": "workUnitsUpdate",
                "workUnits": work_units.iter().map(|wu| json!({
                    "id": wu.id,
                    "title": wu.title,
                    "status": wu.status,
                    "workType": wu.work_type,
                })).collect::<Vec<_>>(),
            }),
        }
    }
}

/// Provider configuration for programmatic credential passing (CONFIG-004)
///
/// Used by sessionManagerCreateWithCredentials() to pass explicit API keys
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

/// CODE-009: Fspec command request data
/// Sent when LLM invokes FspecTool - TypeScript intercepts and executes
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecRequest {
    /// The fspec command (e.g., "create-story", "show-work-unit")
    pub command: String,
    /// Command arguments as JSON string
    #[napi(js_name = "argsJson")]
    pub args_json: String,
    /// Project root directory
    #[napi(js_name = "projectRoot")]
    pub project_root: String,
    /// Tool call ID for correlation with response
    #[napi(js_name = "toolCallId")]
    pub tool_call_id: String,
}

/// CODE-009: Fspec command result data
/// Sent by TypeScript after executing the fspec command
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Command output (structured data as JSON or human-readable text)
    pub data: String,
    /// Error message if failed
    pub error: Option<String>,
    /// System reminder for workflow orchestration (to be injected into LLM context)
    #[napi(js_name = "systemReminder")]
    pub system_reminder: Option<String>,
    /// Tool call ID for correlation
    #[napi(js_name = "toolCallId")]
    pub tool_call_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test StreamChunk::user_input creates correct variant
    #[test]
    fn test_user_input_chunk_creation() {
        let user_message = "Hello, can you help me with this task?";
        let chunk = StreamChunk::user_input(user_message.to_string());

        match chunk {
            StreamChunk::UserInput { text } => {
                assert_eq!(text, user_message);
            }
            _ => panic!("Expected UserInput variant"),
        }
    }

    /// Test empty user input is handled correctly
    #[test]
    fn test_empty_user_input_chunk() {
        let chunk = StreamChunk::user_input(String::new());

        match chunk {
            StreamChunk::UserInput { text } => {
                assert_eq!(text, "");
            }
            _ => panic!("Expected UserInput variant"),
        }
    }

    /// Test user input with multiline content
    #[test]
    fn test_multiline_user_input_chunk() {
        let multiline_message = "First line\nSecond line\nThird line with code:\n```rust\nfn main() {}\n```";
        let chunk = StreamChunk::user_input(multiline_message.to_string());

        match chunk {
            StreamChunk::UserInput { text } => {
                assert_eq!(text, multiline_message);
                assert!(text.contains('\n'));
            }
            _ => panic!("Expected UserInput variant"),
        }
    }

    /// Test user input with special characters
    #[test]
    fn test_special_characters_in_user_input() {
        let special_message = "Test with émojis 🎉 and symbols: <>&\"' and unicode: 你好世界";
        let chunk = StreamChunk::user_input(special_message.to_string());

        match chunk {
            StreamChunk::UserInput { text } => {
                assert_eq!(text, special_message);
            }
            _ => panic!("Expected UserInput variant"),
        }
    }

    /// Test UserInput chunk is distinct from Text chunk
    #[test]
    fn test_user_input_distinct_from_text() {
        let message = "Same content";
        let user_chunk = StreamChunk::user_input(message.to_string());
        let text_chunk = StreamChunk::text(message.to_string());

        match (&user_chunk, &text_chunk) {
            (StreamChunk::UserInput { .. }, StreamChunk::Text { .. }) => {
                // They are different variants - good!
            }
            _ => panic!("Expected different variants"),
        }
    }

    /// NAPI-010: Test SessionStateChange for compacting state
    #[test]
    fn test_session_state_change_compacting() {
        let chunk = StreamChunk::session_state_change(SessionState::Compacting);

        match chunk {
            StreamChunk::SessionStateChange { state } => {
                assert_eq!(state, SessionState::Compacting);
            }
            _ => panic!("Expected SessionStateChange variant"),
        }
    }

    /// NAPI-010: Test UserNotification with severity
    #[test]
    fn test_user_notification_with_severity() {
        let chunk = StreamChunk::user_notification(
            "API rate limit exceeded".to_string(),
            NotificationSeverity::Warning,
        );

        match chunk {
            StreamChunk::UserNotification { message, severity } => {
                assert_eq!(message, "API rate limit exceeded");
                assert_eq!(severity, NotificationSeverity::Warning);
            }
            _ => panic!("Expected UserNotification variant"),
        }
    }
}
