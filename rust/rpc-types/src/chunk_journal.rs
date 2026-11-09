//! BUG-648: which stream chunks a session journals.
//!
//! Feature: spec/features/attaching-to-a-running-session-rebuilds-its-live-state.feature
//!
//! A session's transcript journal is its event log: every chunk the
//! session broadcasts through its output is journaled in broadcast order
//! and read back with `get_transcript_page`. A client applies a journaled
//! chunk from the transcript pages only, so attaching (or reconnecting)
//! replays exactly the events it has not applied yet, with no gap and no
//! duplicate. The sessions layer and the TUI both use this classification.

use super::StreamChunk;

impl StreamChunk {
    /// BUG-648: whether the session journals this chunk. Only the
    /// workspace chrome the daemon sends around a session's output (debug
    /// capture, isolation, footer) is never journaled: a client applies
    /// those when they arrive and reads them again on attach.
    pub fn is_journaled(&self) -> bool {
        match self {
            Self::DebugStateChange { .. }
            | Self::IsolationStateChange { .. }
            | Self::FooterStateUpdate { .. } => false,
            Self::ResponseAttemptStarted
            | Self::ResponseAttemptAborted
            | Self::Text { .. }
            | Self::Thinking { .. }
            | Self::ToolCall { .. }
            | Self::ToolResult { .. }
            | Self::ToolProgress { .. }
            | Self::SessionStateChange { .. }
            | Self::UserNotification { .. }
            | Self::Interrupted { .. }
            | Self::TokenUpdate { .. }
            | Self::ContextFillUpdate { .. }
            | Self::ContextSnapshot { .. }
            | Self::Done
            | Self::Error { .. }
            | Self::UserInput { .. }
            | Self::IncomingMessage { .. }
            | Self::SupervisorPendingInjection { .. }
            | Self::CompactionComplete { .. }
            | Self::CommittedCompaction { .. }
            | Self::FspecCommandRequest { .. }
            | Self::FspecCommandResult { .. }
            | Self::WorkUnitsUpdate { .. }
            | Self::ContinueStateUpdate { .. }
            | Self::ExecStdinRequest { .. }
            | Self::ExecStdinRequestCleared
            | Self::CompactionProgressUpdate { .. }
            | Self::InterruptedWithReason { .. }
            | Self::ProviderWaiting { .. }
            | Self::InteractionRequestChanged { .. } => true,
        }
    }
}
