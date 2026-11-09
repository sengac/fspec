//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        self.core
            .compact_session_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_compact_session.lock() = Some(session_id);
        match self.core.compact_session_result.lock().clone() {
            Ok(result) => Ok(result),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }
    pub async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        self.core
            .resume_session_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_resume_session.lock() = Some(session_id);
        if let Some(msg) = self.core.resume_session_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn get_transcript_page(
        &self,
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> std::result::Result<
        codelet_rpc_types::TranscriptPage,
        codelet_fspec_tui::transport::BackendError,
    > {
        self.scripted_transcript_page(session_id, cursor, snapshot_end, max_bytes)
    }
    pub async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        self.core
            .get_buffered_output_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_get_buffered_output.lock() = Some((session_id, limit));
        Ok(self.core.buffered_output.lock().clone())
    }
    /// TUI-113: scripted active session (RPC-037 default is `Ok(None)`).
    pub async fn get_active_session(&self) -> Result<Option<SessionId>> {
        Ok(self.core.scripted_active_session.lock().clone())
    }
    /// TUI-113: capture the active-session pointer writes (R3).
    pub async fn set_active_session(&self, session_id: SessionId) -> Result<()> {
        self.core
            .set_active_session_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_active_session.lock() = Some(session_id);
        Ok(())
    }
    pub async fn get_work_unit_focus(
        &self,
        session: SessionId,
    ) -> std::result::Result<
        codelet_rpc_types::WorkUnitFocus,
        codelet_fspec_tui::transport::BackendError,
    > {
        self.core.focus_state.get(&session)
    }
    pub async fn set_work_unit_focus(
        &self,
        session: SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> std::result::Result<
        codelet_rpc_types::WorkUnitFocus,
        codelet_fspec_tui::transport::BackendError,
    > {
        let context = self
            .core
            .work_unit_contexts
            .lock()
            .get(&session)
            .filter(|context| Some(&context.id) == work_unit_id.as_ref())
            .cloned();
        self.core.focus_state.set(session, context, source)
    }
    pub fn work_unit_focus_rx(
        &self,
    ) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)> {
        self.core.focus_state.subscribe()
    }
    pub async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        self.core
            .get_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .core
            .work_unit_contexts
            .lock()
            .get(&session_id)
            .cloned())
    }
    pub async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        self.core
            .set_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_work_unit_context.lock() = Some((session_id.clone(), context.clone()));
        if let Some(msg) = self.core.set_work_unit_context_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let mut guard = self.core.work_unit_contexts.lock();
        let focus_context = context.clone();
        match context {
            Some(c) => {
                guard.insert(session_id.clone(), c);
            }
            None => {
                guard.remove(&session_id);
            }
        }
        drop(guard);
        self.core.focus_state.bind(session_id, focus_context)?;
        Ok(())
    }
    // ── RPC-052: pending-input draft persistence ─────────────────────────
}
