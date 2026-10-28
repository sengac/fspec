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
/// `SessionInfo` is re-exported from `rust/napi/src/session_manager.rs`
/// to preserve its placement next to the SessionManager. `SessionId`
/// (newtype around String) and `LogRecord` (structured tracing event) are
/// re-exported here so rust/napi has the full RPC-007 contract surface
/// available without depending on rpc-types directly.
pub use codelet_rpc_types::{ExecStdinRequest, LogRecord, SessionId};

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

// Preserve the historical native-module import path. The codec itself
// is pure Rust and is also available in noop builds.
pub use crate::stream_wire::stream_chunk_to_json_value;

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
mod tests;
