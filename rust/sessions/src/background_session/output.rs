use super::*;

impl BackgroundSession {
    pub fn get_status(&self) -> SessionStatus {
        SessionStatus::from(self.status.load(Ordering::Acquire))
    }

    pub fn set_status(&self, status: SessionStatus) {
        if matches!(
            status,
            SessionStatus::Idle | SessionStatus::Interrupted | SessionStatus::Cleared
        ) {
            self.clear_provider_presentation();
        }
        let old_status = self.status.swap(status as u8, Ordering::AcqRel);
        self.notify_status_change(old_status, status);
    }

    pub(super) fn notify_status_change(&self, old_status: u8, status: SessionStatus) {
        if old_status != status as u8 {
            tracing::info!(
                session_id = %self.id,
                from = ?SessionStatus::from(old_status),
                to = ?status,
                "[compaction-status] set_status transition"
            );
            let _ = self.status_changes_tx.send((
                codelet_rpc_types::SessionId::from(self.id.to_string()),
                status,
            ));
            let state = match status {
                SessionStatus::Idle => SessionState::Idle,
                SessionStatus::Running => SessionState::Running,
                SessionStatus::Interrupted => SessionState::Interrupted,
                SessionStatus::Paused => SessionState::Paused,
                SessionStatus::Compacting => SessionState::Compacting,
                SessionStatus::Cleared => SessionState::Cleared,
            };
            self.handle_output(StreamChunk::session_state_change(state));
            codelet_tools::broadcast_metadata_update();
        }
    }

    pub fn handle_output(&self, chunk: StreamChunk) {
        if matches!(chunk, StreamChunk::ProviderWaiting { .. }) {
            let _ = self.chunks_tx.send((
                codelet_rpc_types::SessionId::from(self.id.to_string()),
                chunk,
            ));
            return;
        }
        let mut projection = self
            .committed_context
            .lock()
            .expect("committed context projection lock");
        if !self.project_committed_context(&mut projection, &chunk) {
            return;
        }
        let id = self.correlation_counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id = format!("{}-{}", self.id, id);
        let chunk = chunk.with_correlation_id(correlation_id);
        let chunk = {
            let pending_ids = self
                .pending_observed_correlation_ids
                .read()
                .expect("pending_observed_correlation_ids lock poisoned");
            if !pending_ids.is_empty() {
                chunk.with_observed_correlation_ids(pending_ids.clone())
            } else {
                chunk
            }
        };
        {
            let mut buffer = self
                .output_buffer
                .write()
                .expect("output buffer lock poisoned");
            buffer.push(chunk.clone());
        }
        self.relay_output.append(&chunk);
        let _ = self.supervisor_broadcast.send(chunk.clone());
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(self.id.to_string()),
            chunk.clone(),
        ));
    }
}
