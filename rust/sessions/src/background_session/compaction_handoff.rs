use super::{BackgroundSession, PromptInput};
use codelet_rpc_types::{SessionStatus, StreamChunk};
use std::sync::atomic::Ordering;

impl BackgroundSession {
    pub fn compaction_epoch(&self) -> u64 {
        self.interrupt_epoch.load(Ordering::Acquire)
    }

    pub fn compaction_cancelled(&self, epoch: u64) -> bool {
        self.is_interrupted.load(Ordering::Acquire) || self.compaction_epoch() != epoch
    }

    pub fn send_compaction_continuation(&self, epoch: u64) -> Result<(), String> {
        if self.compaction_cancelled(epoch) {
            return Err("manual compaction cancelled".into());
        }
        self.set_status(SessionStatus::Running);
        self.handle_output(StreamChunk::user_input("Continue".into()));
        self.input_tx
            .try_send(PromptInput {
                input: "Continue".into(),
                thinking_config: None,
                compaction_epoch: Some(epoch),
            })
            .map_err(|error| format!("Failed to send compaction continuation: {error}"))
    }

    pub async fn discard_cancelled_compaction(&self, epoch: u64) -> bool {
        if !self.compaction_cancelled(epoch) {
            return false;
        }
        self.inner.lock().await.discard_compaction_working_view();
        self.compaction_in_progress.store(false, Ordering::Release);
        self.set_compaction_progress(None);
        self.set_status(SessionStatus::Idle);
        true
    }
}
