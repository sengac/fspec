use super::output::{StreamEvent, StreamOutput};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct BufferedText {
    text: String,
    attempt_start: usize,
}

pub(super) struct ProgressiveOutput<'output, Output: StreamOutput> {
    output: &'output Output,
    progressive: bool,
    pending: Mutex<BufferedText>,
}

impl<'output, Output: StreamOutput> ProgressiveOutput<'output, Output> {
    pub(super) fn new(output: &'output Output, progressive: bool) -> Self {
        Self {
            output,
            progressive,
            pending: Mutex::new(BufferedText::default()),
        }
    }

    fn complete_text(&self) {
        let text = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.attempt_start = 0;
            std::mem::take(&mut pending.text)
        };
        if !text.is_empty() {
            self.output.emit_text(&text);
        }
    }
}

impl<Output: StreamOutput> StreamOutput for ProgressiveOutput<'_, Output> {
    fn emit_tool_cancellation(&self, result: super::output::ToolResultEvent) {
        self.complete_text();
        self.output.emit_tool_cancellation(result);
    }

    fn prepare_tool_dispatch(&self, id: &str) -> Result<(), String> {
        self.output.prepare_tool_dispatch(id)
    }

    fn emit(&self, event: StreamEvent) {
        if !self.progressive {
            match &event {
                StreamEvent::Text(text) => {
                    self.pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .text
                        .push_str(text);
                    return;
                }
                StreamEvent::ResponseAttemptStarted => {
                    let mut pending = self
                        .pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    pending.attempt_start = pending.text.len();
                }
                StreamEvent::ResponseAttemptAborted => {
                    let mut pending = self
                        .pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let start = pending.attempt_start;
                    pending.text.truncate(start);
                }
                StreamEvent::ToolCall(_)
                | StreamEvent::Done(_)
                | StreamEvent::Error(_)
                | StreamEvent::Interrupted(_)
                | StreamEvent::CompactionStarted => self.complete_text(),
                _ => {}
            }
        }
        self.output.emit(event);
    }

    fn flush(&self) {
        self.output.flush();
    }

    fn progress_emitter(&self) -> Option<Arc<dyn StreamOutput>> {
        self.output.progress_emitter()
    }

    fn interruption_reason(&self) -> Option<codelet_rpc_types::InterruptionReason> {
        self.output.interruption_reason()
    }

    fn emit_committed_compaction_complete(&self, result: codelet_rpc_types::CompactionResult) {
        self.output.emit_committed_compaction_complete(result);
    }

    fn publish_committed_context(
        &self,
        identity: &codelet_core::persistence::compaction_boundary::CompactionIdentity,
        snapshot: codelet_rpc_types::CommittedContextSnapshot,
        context: codelet_rpc_types::RequestContextSnapshot,
    ) -> Result<(), String> {
        self.output
            .publish_committed_context(identity, snapshot, context)
    }
}

impl<Output: StreamOutput> Drop for ProgressiveOutput<'_, Output> {
    fn drop(&mut self) {
        self.complete_text();
        self.output.flush();
    }
}
