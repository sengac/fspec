//! Type definitions for NAPI bindings
//!
//! These types are exposed to JavaScript/TypeScript.
//!
//! ## RPC-007 type-uniqueness invariant
//!
//! All types that cross the RPC wire (StreamChunk + its 13 supporting
//! structs/enums, SessionState, NotificationSeverity, the five RPC-007
//! contract types: SessionId, SessionInfo, SessionStatus, StreamChunk,
//! LogRecord) are defined exactly once in `codelet-rpc-types` and
//! re-exported here. The TypeScript shape is preserved verbatim because
//! the lifted definitions retain every `#[cfg_attr(feature = "napi",
//! napi(js_name = ...))]` rename. Only NAPI-only types (HITL, NapiHitl*,
//! NapiToolCall, NapiTurnDetails, etc.) remain locally defined here.

use serde::{Deserialize, Serialize};

// ============================================================================
// RPC-007: Re-exports of types lifted into codelet-rpc-types as the
// single source of truth for the dual-transport RPC.
// ============================================================================

/// PERF-002: Progress information for compaction process
pub use codelet_rpc_types::CompactionProgress;

/// BRIDGE-007: Image data for supervisor input (from Telegram bridge)
pub use codelet_rpc_types::IncomingMessageImage;

/// Token usage tracking information (NAPI-005, TUI-033, TUI-091)
pub use codelet_rpc_types::TokenTracker;

/// Tool call information
pub use codelet_rpc_types::ToolCallInfo;

/// Tool result information
pub use codelet_rpc_types::ToolResultInfo;

/// Tool execution progress information (TOOL-011)
pub use codelet_rpc_types::ToolProgressInfo;

/// Context window fill information (TUI-033)
pub use codelet_rpc_types::ContextFillInfo;

/// Supervisor pending injection information (WATCH-020)
pub use codelet_rpc_types::SupervisorPendingInjectionInfo;

/// Work unit information for file watcher updates.
///
/// RPC-005: lifted into `codelet-rpc-types` so the dual-transport RPC and
/// the NAPI surface share a single source of truth. The `napi` feature
/// gate on `codelet-rpc-types` re-applies the `#[napi(object)]` derive so
/// the existing TypeScript shape (camelCase `workType`) is preserved.
pub use codelet_rpc_types::WorkUnitInfo;

/// RPC-007: Session and log types lifted into `codelet-rpc-types`.
///
/// `SessionInfo` is re-exported from `codelet/napi/src/session_manager.rs`
/// to preserve its placement next to the SessionManager. `SessionId`
/// (newtype around String) and `LogRecord` (structured tracing event) are
/// re-exported here so codelet/napi has the full RPC-007 contract surface
/// available without depending on rpc-types directly.
pub use codelet_rpc_types::{LogRecord, SessionId};

/// NAPI-010: Session state for internal state machine tracking
/// NOT for conversation display - use SessionStateChange chunk variant
pub use codelet_rpc_types::SessionState;

/// NAPI-010: User notification severity levels
pub use codelet_rpc_types::NotificationSeverity;

/// NAPI-010: Stream chunk - proper discriminated union (RPC-007 lift)
pub use codelet_rpc_types::StreamChunk;

/// Compaction result (NAPI-005)
/// Returned by compact() with metrics about the compaction operation
pub use codelet_rpc_types::CompactionResult;

/// CODE-009: Fspec command request data
/// Sent when LLM invokes FspecTool - TypeScript intercepts and executes
pub use codelet_rpc_types::FspecRequest;

/// CODE-009: Fspec command result data
/// Sent by TypeScript after executing the fspec command
pub use codelet_rpc_types::FspecResult;

// ============================================================================
// NAPI-only types (not lifted to codelet-rpc-types because they do not
// cross the RPC wire — they are surface-only types for the JS frontend).
// ============================================================================

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

/// Convert StreamChunk to serde_json::Value for bridge relay (BRIDGE-001)
///
/// This manual serialization is needed because StreamChunk uses NAPI's
/// discriminant-based serialization which doesn't implement serde::Serialize
/// in the shape the bridge needs. The bridge needs to serialize chunks to
/// JSON for WebSocket transmission.
///
/// Free function (NOT inherent method) because StreamChunk is now defined
/// in codelet-rpc-types and Rust's orphan rule forbids inherent impls on
/// types from another crate. The two callers (session_manager.rs and
/// agent_manager_handler.rs) use this function.
pub fn stream_chunk_to_json_value(chunk: &StreamChunk) -> serde_json::Value {
    use serde_json::json;

    match chunk {
        StreamChunk::Text {
            text,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "text",
            "text": text,
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::Thinking {
            thinking,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "thinking",
            "thinking": thinking,
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolCall {
            tool_call,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "toolCall",
            "toolCall": {
                "id": tool_call.id,
                "name": tool_call.name,
                "input": tool_call.input,
            },
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolResult {
            tool_result,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "toolResult",
            "toolResult": {
                "toolCallId": tool_result.tool_call_id,
                "content": tool_result.content,
                "isError": tool_result.is_error,
            },
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolProgress {
            tool_progress,
            correlation_id,
            observed_correlation_ids,
        } => json!({
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
        StreamChunk::SessionStateChange { state } => json!({
            "type": "sessionStateChange",
            "state": format!("{:?}", state),
        }),
        StreamChunk::UserNotification { message, severity } => json!({
            "type": "userNotification",
            "message": message,
            "severity": format!("{:?}", severity),
        }),
        StreamChunk::Interrupted { queued_inputs } => json!({
            "type": "interrupted",
            "queuedInputs": queued_inputs,
        }),
        StreamChunk::TokenUpdate { tokens } => json!({
            "type": "tokenUpdate",
            "tokens": {
                "inputTokens": tokens.input_tokens,
                "outputTokens": tokens.output_tokens,
                "cacheCreationInputTokens": tokens.cache_creation_input_tokens,
                "cacheReadInputTokens": tokens.cache_read_input_tokens,
                "tokensPerSecond": tokens.tokens_per_second,
            },
        }),
        StreamChunk::ContextFillUpdate { context_fill } => json!({
            "type": "contextFillUpdate",
            "contextFill": {
                "fillPercentage": context_fill.fill_percentage,
                "effectiveTokens": context_fill.effective_tokens,
                "threshold": context_fill.threshold,
                "contextWindow": context_fill.context_window,
            },
        }),
        StreamChunk::Done => json!({
            "type": "done",
        }),
        StreamChunk::Error { error } => json!({
            "type": "error",
            "error": error,
        }),
        StreamChunk::UserInput { text } => json!({
            "type": "userInput",
            "text": text,
        }),
        StreamChunk::IncomingMessage { text, images } => {
            let mut obj = json!({
                "type": "supervisorInput",
                "text": text,
            });
            if let Some(imgs) = images {
                obj["images"] = json!(imgs
                    .iter()
                    .map(|i| json!({
                        "data": i.data,
                        "mediaType": i.media_type,
                    }))
                    .collect::<Vec<_>>());
            }
            obj
        }
        StreamChunk::SupervisorPendingInjection {
            supervisor_pending_injection,
        } => json!({
            "type": "supervisorPendingInjection",
            "supervisorPendingInjection": {
                "urgent": supervisor_pending_injection.urgent,
                "content": supervisor_pending_injection.content,
            },
        }),
        StreamChunk::CompactionComplete { compaction_result } => json!({
            "type": "compactionComplete",
            "compactionResult": {
                "originalTokens": compaction_result.original_tokens,
                "compactedTokens": compaction_result.compacted_tokens,
                "compressionRatio": compaction_result.compression_ratio,
                "turnsSummarized": compaction_result.turns_summarized,
                "turnsKept": compaction_result.turns_kept,
            },
        }),
        StreamChunk::FspecCommandRequest { fspec_request } => json!({
            "type": "fspecCommandRequest",
            "fspecRequest": {
                "command": fspec_request.command,
                "argsJson": fspec_request.args_json,
                "projectRoot": fspec_request.project_root,
                "toolCallId": fspec_request.tool_call_id,
            },
        }),
        StreamChunk::FspecCommandResult { fspec_result } => json!({
            "type": "fspecCommandResult",
            "fspecResult": {
                "success": fspec_result.success,
                "data": fspec_result.data,
                "error": fspec_result.error,
                "systemReminder": fspec_result.system_reminder,
                "toolCallId": fspec_result.tool_call_id,
            },
        }),
        StreamChunk::WorkUnitsUpdate { work_units } => json!({
            "type": "workUnitsUpdate",
            "workUnits": work_units.iter().map(|wu| json!({
                "id": wu.id,
                "title": wu.title,
                "status": wu.status,
                "workType": wu.work_type,
            })).collect::<Vec<_>>(),
        }),
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => json!({
            "type": "isolationStateChange",
            "isIsolated": is_isolated,
            "worktreePath": worktree_path,
            "baseCommit": base_commit,
        }),
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path,
            is_git_repo,
            branch,
        } => json!({
            "type": "footerStateUpdate",
            "cwd": cwd,
            "displayPath": display_path,
            "isGitRepo": is_git_repo,
            "branch": branch,
        }),
        StreamChunk::DebugStateChange { enabled } => json!({
            "type": "debugStateChange",
            "enabled": enabled,
        }),
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

/// BUG-117: HITL request state — questions to present to the user
/// Returned by session_get_hitl_request NAPI getter for TypeScript to poll
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiHitlRequestState {
    /// Questions to present to the user (1-3 items)
    pub questions: Vec<HitlQuestionInfo>,
}

/// BUG-117: A single HITL question
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HitlQuestionInfo {
    /// Stable snake_case identifier for mapping answers
    pub id: String,
    /// Short UI label (≤12 chars)
    pub header: String,
    /// Single-sentence prompt shown to user
    pub question: String,
    /// Optional mutually exclusive choices (2-3 items)
    pub options: Option<Vec<HitlOptionInfo>>,
}

/// BUG-117: An option for a HITL question
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HitlOptionInfo {
    /// User-facing label (1-5 words)
    pub label: String,
    /// One sentence explaining impact
    pub description: String,
}

/// BUG-117: HITL response from TypeScript after user answers questions
/// Sent via session_send_hitl_response NAPI function
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HitlResponseInfo {
    /// Whether the user cancelled the modal
    pub cancelled: bool,
    /// Answers keyed by question id (None when cancelled)
    pub answers: Option<Vec<HitlAnswerEntry>>,
}

/// BUG-117: A single answer entry (id + answer data)
/// Using a Vec of entries instead of HashMap because NAPI doesn't support HashMap directly
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HitlAnswerEntry {
    /// Question id this answer corresponds to
    pub id: String,
    /// Labels of selected options
    pub selected: Vec<String>,
    /// Optional freeform text
    pub other: Option<String>,
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
        let multiline_message =
            "First line\nSecond line\nThird line with code:\n```rust\nfn main() {}\n```";
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
        let special_message =
            "Test with émojis 🎉 and symbols: <>&\"' and unicode: 你好世界";
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
