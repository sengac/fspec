//! The StreamChunk enum (RPC-007).

use super::*;

// ============================================================================
// RPC-007: StreamChunk (23 variants, lifted verbatim from
//   rust/napi/src/types.rs:217 with #[napi(discriminant = "type")] and
//   every #[napi(js_name = ...)] rename preserved)
// ============================================================================

/// Streaming chunk discriminated union shared by the embedded transport,
/// the WebSocket transport, and the NAPI re-exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    ResponseAttemptStarted,
    ResponseAttemptAborted,
    Text {
        text: String,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    Thinking {
        thinking: String,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolCall {
        #[serde(rename = "toolCall")]
        tool_call: ToolCallInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolResult {
        #[serde(rename = "toolResult")]
        tool_result: ToolResultInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolProgress {
        #[serde(rename = "toolProgress")]
        tool_progress: ToolProgressInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
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
        #[serde(rename = "queuedInputs")]
        queued_inputs: Vec<String>,
    },
    TokenUpdate {
        tokens: TokenTracker,
    },
    ContextFillUpdate {
        #[serde(rename = "contextFill")]
        context_fill: ContextFillInfo,
    },
    ContextSnapshot {
        snapshot: RequestContextSnapshot,
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
        #[serde(rename = "supervisorPendingInjection")]
        supervisor_pending_injection: SupervisorPendingInjectionInfo,
    },
    CompactionComplete {
        #[serde(rename = "compactionResult")]
        compaction_result: CompactionResult,
    },
    CommittedCompaction {
        snapshot: CommittedContextSnapshot,
    },
    FspecCommandRequest {
        #[serde(rename = "fspecRequest")]
        fspec_request: FspecRequest,
    },
    FspecCommandResult {
        #[serde(rename = "fspecResult")]
        fspec_result: FspecResult,
    },
    WorkUnitsUpdate {
        #[serde(rename = "workUnits")]
        work_units: Vec<WorkUnitInfo>,
    },
    IsolationStateChange {
        #[serde(rename = "isIsolated")]
        is_isolated: bool,
        #[serde(rename = "worktreePath")]
        worktree_path: Option<String>,
        /// RPC-036: the git commit SHA the worktree was forked from.
        /// `None` when the chunk was emitted without baseline info (e.g.
        /// the legacy 2-arg constructor path). The Rust AgentView uses
        /// this to render the isolation diff against the origin commit.
        #[serde(rename = "baseCommit")]
        base_commit: Option<String>,
    },
    FooterStateUpdate {
        cwd: String,
        #[serde(rename = "displayPath")]
        display_path: String,
        #[serde(rename = "isGitRepo")]
        is_git_repo: bool,
        branch: Option<String>,
    },
    DebugStateChange {
        enabled: bool,
    },
    /// CONT-007: state-only live continue/goal counter snapshot —
    /// consumed by the TUI chrome cache, never rendered into the
    /// transcript.
    ContinueStateUpdate {
        #[serde(rename = "continueState")]
        continue_state: ContinueStateInfo,
    },
    /// BUG-171: state-only push of a stored exec-stdin request — the
    /// sessions layer emits it from `BackgroundSession::set_exec_stdin_request`
    /// when a request is stored (detector fire) so the TUI surfaces the
    /// composer overlay WITHOUT a status flip. The TUI folds it into the
    /// exec-stdin slot; it is never rendered into the transcript.
    ExecStdinRequest {
        #[serde(rename = "execStdinRequest")]
        request: ExecStdinRequest,
    },
    /// BUG-171: state-only push of an exec-stdin slot clear — emitted
    /// when the stored request transitions to `None` (child exit
    /// alive-check, successful `write_exec_stdin`, explicit clear) so
    /// the TUI unmounts the overlay. Never rendered into the transcript.
    ExecStdinRequestCleared,
    CompactionProgressUpdate {
        progress: Option<CompactionProgress>,
    },
    InterruptedWithReason {
        #[serde(rename = "queuedInputs")]
        queued_inputs: Vec<String>,
        reason: InterruptionReason,
    },
    ProviderWaiting {
        message: Option<String>,
    },
    /// AMGR-020: a question or approval was raised, answered or cancelled;
    /// sent on every session of its chain.
    InteractionRequestChanged {
        route: RequestRoute,
        status: RequestStatus,
    },
}
