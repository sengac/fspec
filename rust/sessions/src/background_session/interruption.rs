use super::BackgroundSession;
use codelet_rpc_types::InterruptionReason;
use std::sync::atomic::Ordering;

impl BackgroundSession {
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
