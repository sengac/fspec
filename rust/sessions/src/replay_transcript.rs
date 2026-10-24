use crate::replay_content::RestoreError;
use codelet_core::persistence::transcript::{JournalEvent, JournalRecord, TerminalOutcome};
use codelet_rpc_types::{InterruptionReason, NotificationSeverity, StreamChunk, ToolResultInfo};
use serde_json::Value;

pub(crate) fn decode(value: &Value) -> Result<Vec<StreamChunk>, RestoreError> {
    let record: JournalRecord = serde_json::from_value(value.clone())?;
    if record.version != 1 {
        return Err(RestoreError::Content(
            "incompatible transcript version".into(),
        ));
    }
    let notice = |message: &str, severity| StreamChunk::user_notification(message.into(), severity);
    let chunk = match record.event {
        JournalEvent::AttemptStarted | JournalEvent::AttemptAborted => return Ok(Vec::new()),
        JournalEvent::Text { text } => StreamChunk::text(text),
        JournalEvent::Thinking { thinking } => StreamChunk::thinking(thinking),
        JournalEvent::ToolRequested { .. } => notice(
            "Tool requested; execution has not started.",
            NotificationSeverity::Info,
        ),
        JournalEvent::ToolStarted { .. } | JournalEvent::ToolOutcomeUnknown { .. } => notice(
            "Tool execution outcome is unknown.",
            NotificationSeverity::Warning,
        ),
        JournalEvent::ToolCompleted {
            id,
            content,
            is_error,
        } => StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: id,
            content,
            is_error,
        }),
        JournalEvent::ToolCancelled { .. } => notice(
            "Tool cancelled before execution.",
            NotificationSeverity::Info,
        ),
        JournalEvent::Terminal { outcome } => match outcome {
            TerminalOutcome::ProviderError => StreamChunk::error(outcome.message().into()),
            TerminalOutcome::UserCancelled | TerminalOutcome::LoopInterrupted => {
                StreamChunk::InterruptedWithReason {
                    queued_inputs: Vec::new(),
                    reason: if outcome == TerminalOutcome::UserCancelled {
                        InterruptionReason::User
                    } else {
                        InterruptionReason::LoopDetector
                    },
                }
            }
            TerminalOutcome::Completed => StreamChunk::done(),
            TerminalOutcome::Interrupted | TerminalOutcome::CompactionFailed => {
                notice(outcome.message(), NotificationSeverity::Warning)
            }
        },
    };
    Ok(vec![chunk])
}
