//! codelet-rpc-types: shared serde types for the fspec dual-transport RPC.
//!
//! Single source of truth for any type that crosses the RPC boundary in
//! either direction. Default builds have zero dependencies on tarpc, tokio,
//! or napi — those crates depend on us, never the other way around.
//!
//! ## NAPI feature gate
//!
//! Enabling the `napi` feature applies `#[napi(object)]` (or
//! `#[napi(discriminant = "type")]` / `#[napi(string_enum)]`) to types that
//! cross the JS boundary so that `codelet/napi` can re-export them
//! verbatim and preserve the existing TypeScript shape (most notably
//! `correlationId`/`observedCorrelationIds`/`toolCall` etc. via
//! `#[napi(js_name = ...)]`). The feature is off by default; only
//! `codelet-napi` opts in.
//!
//! RPC-005 lifted only `WorkUnitInfo` from `codelet/napi/src/types.rs:182`.
//! RPC-007 lifts five additional types that the session REPL needs as a
//! single source of truth shared by the embedded transport, the WebSocket
//! transport, and the NAPI surface:
//!   * [`SessionId`] — newtype around String
//!   * [`SessionInfo`] — opaque metadata returned by `list_sessions`
//!   * [`SessionStatus`] — coarse session lifecycle state
//!   * [`StreamChunk`] — the 23-variant streaming chunk discriminated union
//!     plus its supporting structs
//!   * [`LogRecord`] — structured tracing event payload

use serde::{Deserialize, Serialize};

/// Work unit information shared across all transports and the NAPI surface.
///
/// Field order and naming match the original NAPI definition so that the
/// `napi` feature gate can preserve the existing TypeScript shape without
/// breaking changes.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    #[cfg_attr(feature = "napi", napi(js_name = "workType"))]
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
}

// ============================================================================
// RPC-007: Session types
// ============================================================================

/// Stable identifier for a session. Newtype around `String` so the wire
/// shape stays a plain string but the type system distinguishes session
/// IDs from arbitrary strings.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId {
    pub value: String,
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

/// Coarse session lifecycle state.
///
/// Variant ORDER is preserved exactly so that `as u8` casts in
/// `codelet/napi/src/session_manager.rs`
/// (`AtomicU8::new(SessionStatus::Idle as u8)` /
/// `status.swap(status as u8, ...)`) keep the historical discriminant
/// values 0..=4 stable after the type was lifted out of NAPI. `Cleared`
/// is appended (5) as RPC-007's only new variant.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SessionStatus {
    #[default]
    Idle,
    Running,
    Interrupted,
    /// PAUSE-001: Session is paused waiting for user input (Enter/Y/N/Esc)
    Paused,
    /// PERF-002: Session is compacting context - supports progress tracking
    Compacting,
    /// RPC-007: Session has been cleared (post-cleanup terminal state).
    Cleared,
}

impl SessionStatus {
    /// Convert status to string representation for TypeScript / log output.
    /// Lifted from `codelet/napi/src/session_manager.rs` as part of the
    /// RPC-007 type-uniqueness rule so callers in `codelet/napi` can use
    /// the inherent method via the re-export.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Running => "running",
            SessionStatus::Interrupted => "interrupted",
            SessionStatus::Paused => "paused",
            SessionStatus::Compacting => "compacting",
            SessionStatus::Cleared => "cleared",
        }
    }
}

impl From<u8> for SessionStatus {
    /// Inverse of `SessionStatus as u8`, lifted from
    /// `codelet/napi/src/session_manager.rs` so the historical
    /// `AtomicU8`-based round-trip continues to compile after the
    /// lift. Unknown values fall back to `Idle` (matches the pre-lift
    /// behaviour).
    fn from(v: u8) -> Self {
        match v {
            0 => SessionStatus::Idle,
            1 => SessionStatus::Running,
            2 => SessionStatus::Interrupted,
            3 => SessionStatus::Paused,
            4 => SessionStatus::Compacting,
            5 => SessionStatus::Cleared,
            _ => SessionStatus::Idle,
        }
    }
}

/// Public metadata about a session returned by `list_sessions`.
///
/// Field names match the NAPI surface verbatim (see
/// codelet/napi/src/session_manager.rs:380) so that codelet/napi can
/// re-export this type without changing the existing TypeScript shape.
/// `id` is a plain `String` rather than a [`SessionId`] newtype so the
/// TS shape stays a flat string.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project: String,
    pub message_count: u32,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    /// GIT-029: Whether this is an isolated session with a git worktree
    pub is_isolated: bool,
    /// GIT-029: Path to the worktree (if isolated)
    pub worktree_path: Option<String>,
    /// RPC-007: optional role string the session was created with.
    pub role: Option<String>,
}

/// Structured log event payload pushed to subscribers via the
/// `logs_rx` broadcast channel.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub level: String,
    pub target: String,
    pub message: String,
    /// Unix epoch milliseconds when the event was captured.
    #[cfg_attr(feature = "napi", napi(js_name = "timestampMs"))]
    pub timestamp_ms: i64,
}

/// RPC-011: live health summary returned by `FspecService::health` and
/// reused by the `fspec status` subcommand for human-readable output.
///
/// All counters are point-in-time reads from `ServerStats`. `version`
/// is the daemon process's `env!("CARGO_PKG_VERSION")` so the caller
/// can sanity-check protocol compatibility.
///
/// Lifted into `codelet-rpc-types` (rather than living on the server
/// crate) so both transports — `EmbeddedFspecBackend` (which reads
/// `ServerStats` directly) and `WebSocketFspecBackend` (which receives
/// the struct over tarpc) — share the SAME wire shape. The napi
/// feature gate follows the existing `WorkUnitInfo`/`SessionInfo`
/// pattern so the type can be re-exported through `codelet-napi`
/// verbatim if a future JS surface needs it.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Seconds since the daemon's `ServerStats::started_at` instant.
    ///
    /// Typed `i64` (rather than `u64`) so the `napi(object)` cfg-gate
    /// compiles under napi-derive v3 + `napi4` feature, which does not
    /// support `u64` in `napi(object)` field positions. The wire format
    /// (tarpc bincode) carries the same 8 bytes either way, and uptime
    /// can never exceed 2^63 seconds in practice.
    pub uptime_secs: i64,
    /// Live count of attached WebSocket clients (decremented via the
    /// `ConnectedClientGuard` Drop impl when each connection task
    /// exits).
    pub connected_clients: i64,
    /// Elapsed seconds since the workspace watcher last fired an Ok
    /// snapshot into the work-units fanout task. `None` if no
    /// snapshot has ever been observed by this daemon.
    pub last_watcher_event_secs_ago: Option<i64>,
    /// Cumulative `RecvError::Lagged` count surfaced by the chunks
    /// broadcast fanout.
    pub lag_chunks: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the logs
    /// broadcast fanout.
    pub lag_logs: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the work-units
    /// broadcast fanout.
    pub lag_work_units: i64,
    /// Daemon process's `env!("CARGO_PKG_VERSION")`.
    pub version: String,
}

// ============================================================================
// RPC-007: StreamChunk supporting types (lifted verbatim from
//   codelet/napi/src/types.rs)
// ============================================================================

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionProgress {
    pub phase: String,
    pub current: u32,
    pub total: u32,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressInfo {
    pub tool_call_id: String,
    pub tool_name: String,
    pub output_chunk: String,
    pub is_stderr: bool,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFillInfo {
    pub fill_percentage: u32,
    pub effective_tokens: f64,
    pub threshold: f64,
    pub context_window: f64,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorPendingInjectionInfo {
    pub urgent: bool,
    pub content: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessageImage {
    pub data: String,
    #[cfg_attr(feature = "napi", napi(js_name = "mediaType"))]
    pub media_type: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_tokens: u32,
    pub compacted_tokens: u32,
    pub compression_ratio: f64,
    pub turns_summarized: u32,
    pub turns_kept: u32,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecRequest {
    pub command: String,
    #[cfg_attr(feature = "napi", napi(js_name = "argsJson"))]
    pub args_json: String,
    #[cfg_attr(feature = "napi", napi(js_name = "projectRoot"))]
    pub project_root: String,
    #[cfg_attr(feature = "napi", napi(js_name = "toolCallId"))]
    pub tool_call_id: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    #[cfg_attr(feature = "napi", napi(js_name = "systemReminder"))]
    pub system_reminder: Option<String>,
    #[cfg_attr(feature = "napi", napi(js_name = "toolCallId"))]
    pub tool_call_id: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
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

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
    Cleared,
}

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}

// ============================================================================
// RPC-007: StreamChunk (23 variants, lifted verbatim from
//   codelet/napi/src/types.rs:217 with #[napi(discriminant = "type")] and
//   every #[napi(js_name = ...)] rename preserved)
// ============================================================================

/// Streaming chunk discriminated union shared by the embedded transport,
/// the WebSocket transport, and the NAPI re-exports.
#[cfg_attr(feature = "napi", napi_derive::napi(discriminant = "type"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    Text {
        text: String,
        #[cfg_attr(feature = "napi", napi(js_name = "correlationId"))]
        correlation_id: Option<String>,
        #[cfg_attr(feature = "napi", napi(js_name = "observedCorrelationIds"))]
        observed_correlation_ids: Option<Vec<String>>,
    },
    Thinking {
        thinking: String,
        #[cfg_attr(feature = "napi", napi(js_name = "correlationId"))]
        correlation_id: Option<String>,
        #[cfg_attr(feature = "napi", napi(js_name = "observedCorrelationIds"))]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolCall {
        #[cfg_attr(feature = "napi", napi(js_name = "toolCall"))]
        tool_call: ToolCallInfo,
        #[cfg_attr(feature = "napi", napi(js_name = "correlationId"))]
        correlation_id: Option<String>,
        #[cfg_attr(feature = "napi", napi(js_name = "observedCorrelationIds"))]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolResult {
        #[cfg_attr(feature = "napi", napi(js_name = "toolResult"))]
        tool_result: ToolResultInfo,
        #[cfg_attr(feature = "napi", napi(js_name = "correlationId"))]
        correlation_id: Option<String>,
        #[cfg_attr(feature = "napi", napi(js_name = "observedCorrelationIds"))]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolProgress {
        #[cfg_attr(feature = "napi", napi(js_name = "toolProgress"))]
        tool_progress: ToolProgressInfo,
        #[cfg_attr(feature = "napi", napi(js_name = "correlationId"))]
        correlation_id: Option<String>,
        #[cfg_attr(feature = "napi", napi(js_name = "observedCorrelationIds"))]
        observed_correlation_ids: Option<Vec<String>>,
    },
    SessionStateChange {
        state: SessionState,
    },
    UserNotification {
        message: String,
        severity: NotificationSeverity,
    },
    Interrupted {
        #[cfg_attr(feature = "napi", napi(js_name = "queuedInputs"))]
        queued_inputs: Vec<String>,
    },
    TokenUpdate {
        tokens: TokenTracker,
    },
    ContextFillUpdate {
        #[cfg_attr(feature = "napi", napi(js_name = "contextFill"))]
        context_fill: ContextFillInfo,
    },
    Done,
    Error {
        error: String,
    },
    UserInput {
        text: String,
    },
    IncomingMessage {
        text: String,
        images: Option<Vec<IncomingMessageImage>>,
    },
    SupervisorPendingInjection {
        #[cfg_attr(feature = "napi", napi(js_name = "supervisorPendingInjection"))]
        supervisor_pending_injection: SupervisorPendingInjectionInfo,
    },
    CompactionComplete {
        #[cfg_attr(feature = "napi", napi(js_name = "compactionResult"))]
        compaction_result: CompactionResult,
    },
    FspecCommandRequest {
        #[cfg_attr(feature = "napi", napi(js_name = "fspecRequest"))]
        fspec_request: FspecRequest,
    },
    FspecCommandResult {
        #[cfg_attr(feature = "napi", napi(js_name = "fspecResult"))]
        fspec_result: FspecResult,
    },
    WorkUnitsUpdate {
        #[cfg_attr(feature = "napi", napi(js_name = "workUnits"))]
        work_units: Vec<WorkUnitInfo>,
    },
    IsolationStateChange {
        #[cfg_attr(feature = "napi", napi(js_name = "isIsolated"))]
        is_isolated: bool,
        #[cfg_attr(feature = "napi", napi(js_name = "worktreePath"))]
        worktree_path: Option<String>,
    },
    FooterStateUpdate {
        cwd: String,
        #[cfg_attr(feature = "napi", napi(js_name = "displayPath"))]
        display_path: String,
        #[cfg_attr(feature = "napi", napi(js_name = "isGitRepo"))]
        is_git_repo: bool,
        branch: Option<String>,
    },
    DebugStateChange {
        enabled: bool,
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

    /// Supervisor input message (WATCH-006: for supervisor injection into subordinate session)
    /// BRIDGE-007: Extended to support optional images
    pub fn incoming_message(formatted_message: String) -> Self {
        Self::IncomingMessage {
            text: formatted_message,
            images: None,
        }
    }

    /// Supervisor input message with images (BRIDGE-007)
    pub fn incoming_message_with_images(
        formatted_message: String,
        images: Vec<IncomingMessageImage>,
    ) -> Self {
        Self::IncomingMessage {
            text: formatted_message,
            images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
        }
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

    /// Set observed correlation IDs for supervisor response chunks (WATCH-011)
    pub fn with_observed_correlation_ids(mut self, ids: Vec<String>) -> Self {
        match &mut self {
            Self::Text {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::Thinking {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolCall {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolResult {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolProgress {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            // Other variants don't have observed_correlation_ids
            _ => {}
        }
        self
    }

    /// Supervisor pending injection - when auto_inject=false (WATCH-020)
    pub fn supervisor_pending_injection(urgent: bool, content: String) -> Self {
        Self::SupervisorPendingInjection {
            supervisor_pending_injection: SupervisorPendingInjectionInfo { urgent, content },
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

    /// GIT-029: Isolation state change - emitted when session isolation state changes
    pub fn isolation_state_change(is_isolated: bool, worktree_path: Option<String>) -> Self {
        Self::IsolationStateChange {
            is_isolated,
            worktree_path,
        }
    }

    /// TUI-091: Footer state update - emitted by background poller
    pub fn footer_state_update(
        cwd: String,
        display_path: String,
        is_git_repo: bool,
        branch: Option<String>,
    ) -> Self {
        Self::FooterStateUpdate {
            cwd,
            display_path,
            is_git_repo,
            branch,
        }
    }

    /// BUG-134: Debug state change - emitted when session debug capture toggles
    pub fn debug_state_change(enabled: bool) -> Self {
        Self::DebugStateChange { enabled }
    }
}
