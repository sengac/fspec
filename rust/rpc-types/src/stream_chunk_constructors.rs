//! StreamChunk constructors and helpers (RPC-007).

use super::*;

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

    /// GIT-029 / RPC-036: Isolation state change - emitted when session
    /// isolation state changes. The optional `base_commit` parameter
    /// carries the git SHA the worktree was forked from; pre-RPC-036
    /// callers using the 2-arg form get `base_commit = None`.
    pub fn isolation_state_change(is_isolated: bool, worktree_path: Option<String>) -> Self {
        Self::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit: None,
        }
    }

    /// RPC-036: Isolation state change including the baseline commit
    /// SHA the worktree was forked from. Use this constructor when
    /// emitting from a code path that has the baseline SHA in hand
    /// (e.g. `SessionManager::create_isolated_session`).
    pub fn isolation_state_change_with_base(
        is_isolated: bool,
        worktree_path: Option<String>,
        base_commit: Option<String>,
    ) -> Self {
        Self::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
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

    /// CONT-007: live continue/goal counter snapshot (state-only).
    pub fn continue_state_update(continue_state: ContinueStateInfo) -> Self {
        Self::ContinueStateUpdate { continue_state }
    }

    /// BUG-171: state-only exec-stdin request push (detector fire).
    pub fn exec_stdin_request(request: ExecStdinRequest) -> Self {
        Self::ExecStdinRequest { request }
    }

    /// BUG-171: state-only exec-stdin cleared push (slot → None).
    pub fn exec_stdin_request_cleared() -> Self {
        Self::ExecStdinRequestCleared
    }
}
