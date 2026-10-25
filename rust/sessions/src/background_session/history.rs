use super::*;

impl BackgroundSession {
    pub fn clear_history(&self) -> Result<(), codelet_providers::session::ExtensionError> {
        let mut inner = self.inner.blocking_lock();
        inner.reset_provider_state()?;
        self.manifest_restore.store(false, Ordering::Release);
        if let Ok(mut buffer) = self.output_buffer.write() {
            buffer.clear();
        }
        inner.messages.clear();
        inner.turns.clear();
        inner.token_tracker = codelet_core::compaction::TokenTracker::default();
        inner.token_tracker.invalidate_input();
        let isolation = self.build_isolation_context();
        inner.inject_context_reminders_with_isolation(isolation.as_ref());
        drop(inner);
        self.reset_interrupt();
        self.handle_output(StreamChunk::session_state_change(SessionState::Cleared));
        let _ = self
            .session_cleared_tx
            .send(codelet_rpc_types::SessionId::from(self.id.to_string()));
        Ok(())
    }

    pub fn completed_turn_count(&self) -> u32 {
        self.output_buffer
            .read()
            .map(|buffer| buffer.completed_turns())
            .unwrap_or(0) as u32
    }

    pub fn get_info(&self) -> SessionInfo {
        let message_count = self
            .output_buffer
            .read()
            .expect("output buffer lock poisoned")
            .completed_turns() as u32;
        SessionInfo {
            id: self.id.to_string(),
            name: self.name.read().expect("name lock poisoned").clone(),
            status: self.get_status().as_str().to_string(),
            project: self.project.clone(),
            message_count,
            provider_id: self
                .provider_id
                .read()
                .expect("provider_id lock poisoned")
                .clone(),
            model_id: self
                .model_id
                .read()
                .expect("model_id lock poisoned")
                .clone(),
            is_isolated: self.worktree_path().is_some(),
            worktree_path: self
                .worktree_path()
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            role: None,
            updated_at_ms: Some(chrono::Utc::now().timestamp_millis()),
            work_unit_focus: self.activity_focus(),
        }
    }
}
