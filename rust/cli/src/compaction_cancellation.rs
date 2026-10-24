use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Notify;

use crate::compaction_operation::CompactionFailure;

pub struct CompactionCancellation {
    interrupted: Arc<AtomicBool>,
    notify: Option<Arc<Notify>>,
}

impl CompactionCancellation {
    pub fn new(interrupted: Arc<AtomicBool>, notify: Option<Arc<Notify>>) -> Self {
        Self {
            interrupted,
            notify,
        }
    }

    pub fn check(&self) -> Result<(), CompactionFailure> {
        if self.interrupted.load(Ordering::Acquire) {
            Err(CompactionFailure::Cancelled)
        } else {
            Ok(())
        }
    }

    pub async fn cancelled(&self) {
        loop {
            if let Some(notify) = &self.notify {
                let notified = notify.notified();
                if self.check().is_err() {
                    return;
                }
                notified.await;
            } else {
                if self.check().is_err() {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    }
}
