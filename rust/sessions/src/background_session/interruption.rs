use super::BackgroundSession;
use codelet_rpc_types::InterruptionReason;
use std::sync::atomic::Ordering;

impl BackgroundSession {
    pub fn begin_settlement(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, String> {
        self.settlement_gate
            .clone()
            .try_lock_owned()
            .map_err(|_| "durable settlement already has an owner".into())
    }

    pub async fn wait_for_settlement(&self) {
        let _ownership = self.settlement_gate.lock().await;
    }

    pub fn settlement_in_progress(&self) -> bool {
        self.settlement_gate.try_lock().is_err()
    }

    pub fn queued_input_epoch(&self) -> Option<u64> {
        let _reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (!self.is_interrupted.load(Ordering::Acquire)).then(|| self.compaction_epoch())
    }

    pub fn admit_input_epoch(&self, epoch: u64) -> bool {
        let mut reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.compaction_epoch() != epoch {
            return false;
        }
        *reason = None;
        let _ = self.take_pending_loop_abort_note();
        self.is_interrupted.store(false, Ordering::Release);
        codelet_tools::clear_bash_abort(self.id);
        true
    }

    pub fn interruption_reason(&self) -> Option<InterruptionReason> {
        *self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub fn interrupt_loop_detector(&self, note: String) {
        let mut reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *reason == Some(InterruptionReason::User) {
            return;
        }
        *reason = Some(InterruptionReason::LoopDetector);
        self.set_pending_loop_abort_note(note);
        self.is_interrupted.store(true, Ordering::Release);
        self.interrupt_epoch.fetch_add(1, Ordering::AcqRel);
        codelet_tools::request_bash_abort(self.id);
        self.interrupt_notify.notify_one();
    }

    pub fn admit_loop_retry(&self, epoch: u64) -> bool {
        let mut reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *reason != Some(InterruptionReason::LoopDetector)
            || self.compaction_epoch() != epoch
            || !self.has_pending_loop_abort_note()
        {
            return false;
        }
        *reason = None;
        self.is_interrupted.store(false, Ordering::Release);
        codelet_tools::clear_bash_abort(self.id);
        true
    }
}
