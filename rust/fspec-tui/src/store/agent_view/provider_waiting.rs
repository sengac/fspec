use super::SessionContext;
use crate::terminal::sanitize::sanitize_for_terminal;
use codelet_rpc_types::StreamChunk;

impl SessionContext {
    pub(crate) fn apply_provider_waiting(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::ProviderWaiting { message } => {
                self.provider_waiting = message.as_ref().and_then(|message| {
                    let clean = sanitize_for_terminal(message)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    (!clean.is_empty()).then_some(clean)
                });
            }
            StreamChunk::Text { text, .. } if !text.is_empty() => self.provider_waiting = None,
            StreamChunk::Thinking { thinking, .. } if !thinking.is_empty() => {
                self.provider_waiting = None;
            }
            StreamChunk::ToolCall { .. }
            | StreamChunk::ToolResult { .. }
            | StreamChunk::ToolProgress { .. }
            | StreamChunk::Done
            | StreamChunk::Error { .. }
            | StreamChunk::Interrupted { .. }
            | StreamChunk::InterruptedWithReason { .. }
            | StreamChunk::UserInput { .. }
            | StreamChunk::SessionStateChange { .. }
            | StreamChunk::ResponseAttemptStarted
            | StreamChunk::ResponseAttemptAborted => self.provider_waiting = None,
            _ => {}
        }
    }
}
