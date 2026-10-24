use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use codelet_core::persistence::AssistantContent;

use super::{BackgroundOutput, LoopEscalationPolicy, StreamLoopDetector};

pub(super) struct ResponseAttemptCheckpoint {
    content_len: usize,
    thinking: StreamLoopDetector,
    text: StreamLoopDetector,
    escalation: LoopEscalationPolicy,
    last_trigger: Option<Instant>,
    abort_fired: bool,
}

fn snapshot<T: Clone>(state: &Mutex<T>) -> T {
    state.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn restore<T>(state: &Mutex<T>, value: T) {
    *state.lock().unwrap_or_else(PoisonError::into_inner) = value;
}

impl BackgroundOutput {
    pub(super) fn start_response_attempt(&self) {
        let checkpoint = ResponseAttemptCheckpoint {
            content_len: self
                .assistant_content
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len(),
            thinking: snapshot(&self.thinking_loop_detector),
            text: snapshot(&self.text_loop_detector),
            escalation: snapshot(&self.loop_escalation_policy),
            last_trigger: snapshot(&self.loop_last_trigger_at),
            abort_fired: snapshot(&self.loop_abort_fired),
        };
        restore(&self.response_attempt, Some(checkpoint));
    }

    pub(super) fn abort_response_attempt(&self) {
        let Some(checkpoint) = self
            .response_attempt
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        else {
            return;
        };
        let mut content = self
            .assistant_content
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let boundary = checkpoint.content_len.min(content.len());
        let abandoned = content.split_off(boundary);
        content.extend(abandoned.into_iter().filter(|block| {
            !matches!(
                block,
                AssistantContent::Text { .. } | AssistantContent::Thinking { .. }
            )
        }));
        restore(&self.thinking_loop_detector, checkpoint.thinking);
        restore(&self.text_loop_detector, checkpoint.text);
        restore(&self.loop_escalation_policy, checkpoint.escalation);
        restore(&self.loop_last_trigger_at, checkpoint.last_trigger);
        restore(&self.loop_abort_fired, checkpoint.abort_fired);
    }
}
