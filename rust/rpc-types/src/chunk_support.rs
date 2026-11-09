//! StreamChunk supporting types (RPC-007).

use super::*;

// ============================================================================
// RPC-007: StreamChunk supporting types (lifted verbatim from
//   rust/napi/src/types.rs)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Compaction activity; legacy count fields are not measured completion units.
pub struct CompactionProgress {
    pub phase: String,
    /// Retained for wire compatibility; zero with total zero means unknown work.
    pub current: u32,
    /// Retained for wire compatibility; not a turn count or percentage basis.
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressInfo {
    pub tool_call_id: String,
    pub tool_name: String,
    pub output_chunk: String,
    pub is_stderr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFillInfo {
    pub fill_percentage: u32,
    pub effective_tokens: f64,
    pub threshold: f64,
    pub context_window: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorPendingInjectionInfo {
    pub urgent: bool,
    pub content: String,
}

/// CONT-007: live auto-continue / goal counter snapshot pushed by the
/// engine at every counter transition (nudge consumed/refunded, turn
/// reset, budget exhaustion, accepted done()). Pure state — carries no
/// transition reason; the TUI folds it into the chrome cache so the
/// footer indicator paints the REAL `nudges_used`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinueStateInfo {
    pub enabled: bool,
    pub budget: u32,
    #[serde(rename = "nudgesUsed")]
    pub nudges_used: u32,
    #[serde(rename = "goalActive")]
    pub goal_active: bool,
    /// Display budget: `max(explicit, 15)` while a goal is active,
    /// the explicit `/continue` budget otherwise.
    #[serde(rename = "effectiveBudget")]
    pub effective_budget: u32,
    /// CONT-008: true ONLY on the goal-satisfied teardown snapshot — the
    /// engine cleared an active goal via an accepted done(). Consumers
    /// (BackgroundOutput write-back, TUI goal cache) must key off THIS
    /// flag, never off an incidental `goal_active: false` (every non-goal
    /// snapshot carries that). `#[serde(default)]` keeps older payloads
    /// deserializable.
    #[serde(rename = "goalCleared", default)]
    pub goal_cleared: bool,
    /// CONT-008: done() rejection count for the session, registry-synced
    /// at the FinalResponse settle point (may lag one in-flight segment).
    /// Drives the bare `/goal` "rejections: n" display on the TUI.
    #[serde(rename = "doneRejections", default)]
    pub done_rejections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IncomingMessageImage {
    pub data: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
}

/// RPC-061: wire-portable payload for `receive_incoming_message`. Mirrors
/// the fields of `codelet_sessions::IncomingMessage` (formerly
/// `SupervisorInput` — see WATCH-003/006/008/011/019/020) so the
/// supervisor → subordinate injection path round-trips identically
/// across both embedded and WebSocket transports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IncomingMessageInput {
    #[serde(rename = "sourceSessionId")]
    pub source_session_id: String,
    #[serde(rename = "roleName")]
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<IncomingMessageImage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_tokens: u32,
    pub compacted_tokens: u32,
    /// PERCENT of tokens removed by compaction, range [0,100] — e.g.
    /// 60.0 means 60% of the original tokens were eliminated. Producers
    /// compute `interactive_helpers::compression_ratio(orig, compacted)
    /// * 100.0`; consumers display this value directly (RPC-420 — do
    /// NOT re-derive via `(1.0 - ratio) * 100.0`).
    pub compression_ratio: f64,
    pub turns_summarized: u32,
    pub turns_kept: u32,
}

/// RPC-025: transport-portable match returned by
/// `FspecService::persistence_search_history`. Mirrors the relevant
/// fields of `codelet_core::persistence::history::HistoryEntry` but
/// formats the timestamp as an RFC3339 string so non-Rust consumers
/// don't need a chrono dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMatch {
    pub session_id: SessionId,
    pub text: String,
    pub timestamp_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecRequest {
    pub command: String,
    #[serde(rename = "argsJson")]
    pub args_json: String,
    #[serde(rename = "projectRoot")]
    pub project_root: String,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FspecResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    #[serde(rename = "systemReminder")]
    pub system_reminder: Option<String>,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTracker {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub tokens_per_second: Option<f64>,
    pub cumulative_billed_input: Option<u32>,
    pub cumulative_billed_output: Option<u32>,
    pub reasoning_tokens: Option<u32>,
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
            reasoning_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
    Cleared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}
