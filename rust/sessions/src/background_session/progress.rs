use super::*;

impl BackgroundSession {
    pub fn get_compaction_progress(&self) -> Option<CompactionProgress> {
        self.compaction_progress
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set_compaction_progress(&self, progress: Option<CompactionProgress>) {
        tracing::debug!(
            session_id = %self.id,
            progress = ?progress,
            "[compaction-status] set_compaction_progress"
        );
        let mut stored_progress = self
            .compaction_progress
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *stored_progress = progress.clone();
        self.handle_output(StreamChunk::CompactionProgressUpdate {
            progress: progress.map(Into::into),
        });
    }

    pub fn update_compaction_progress(&self, phase: String, current: u32, total: u32) {
        self.set_compaction_progress(Some(CompactionProgress {
            phase,
            current,
            total,
        }));
    }

    pub fn set_pending_observed_correlation_ids(&self, ids: Vec<String>) {
        *self
            .pending_observed_correlation_ids
            .write()
            .expect("pending_observed_correlation_ids lock poisoned") = ids;
    }

    pub fn clear_pending_observed_correlation_ids(&self) {
        self.pending_observed_correlation_ids
            .write()
            .expect("pending_observed_correlation_ids lock poisoned")
            .clear();
    }
}
