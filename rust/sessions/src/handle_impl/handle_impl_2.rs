macro_rules! compose_handle_impl_2 {
    ($($collected:tt)*) => {
        compose_handle_impl_3! {
            $($collected)*
    fn get_compaction_progress(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<CompactionProgress>, String> {
        Ok(self
            .get_session(&session_id.value)?
            .get_compaction_progress()
            .map(CompactionProgress::from))
    }
    fn get_transcript_page(
        &self,
        session_id: &SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<codelet_rpc_types::TranscriptPage, codelet_rpc_types::TranscriptError> {
        self.get_session(&session_id.value)
            .map_err(|_| {
                codelet_rpc_types::TranscriptError::SessionUnavailable(session_id.value.clone())
            })?
            .get_transcript_page(cursor, snapshot_end, max_bytes)
    }
    fn get_buffered_output(
        &self,
        session_id: &SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>, String> {
        Ok(self
            .get_session(&session_id.value)?
            .get_buffered_output(limit as usize))
    }
    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                // RPC-073: BackgroundSession::clear_history calls
                // `self.inner.blocking_lock()` (see
                // rust/sessions/src/background_session.rs:1156),
                // which panics with "Cannot block the current thread
                // from within a runtime" when invoked from inside a
                // tokio multi-thread worker (the tarpc dispatcher).
                // Wrap in `tokio::task::block_in_place` so the worker
                // is temporarily detached from the scheduler — same
                // pattern as the other sync→async bridges in this
                // file (`create_session`, `create_isolated_session`,
                // `test_provider_connection`, the three `loop_*`
                // methods).
                tokio::task::block_in_place(|| session.clear_history())
                    .map_err(|error| error.to_string())
            }
            Err(error) => Err(error),
        }
    }
    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        // RPC-418 / BUG-516: manual /compact queues the compactor sub-agent
        // round that automatic compaction uses (see
        // BackgroundSession::start_manual_compaction).
        self.get_session(&session_id.value)?.start_manual_compaction()
    }
    fn restore_session_messages(
        &self,
        session_id: &SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        self.restore_messages(session_id, envelopes)
            .map_err(|error| error.to_string())
    }
    fn restore_session_token_state(
        &self,
        session_id: &SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String> {
        self.restore_tokens(session_id, state)
            .map_err(|error| error.to_string())
    }
    fn get_work_unit_context(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<WorkUnitContext>, String> {
        Ok(self
            .get_session(&session_id.value)?
            .get_work_unit_context()
            .map(WorkUnitContext::from))
    }
    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                self.set_session_focus(
                    session_id,
                    ctx.as_ref().map(|context| context.id.as_str()),
                    "board",
                )
                .map_err(|error| error.to_string())?;
                match ctx {
                    Some(c) => {
                        session.set_work_unit_context(Some(c.id), Some(c.title), Some(c.status))
                    }
                    None => session.set_work_unit_context(None, None, None),
                }
            }
            Err(error) => Err(error),
        }
    }
    fn get_work_unit_focus(
        &self,
        session_id: &SessionId,
    ) -> Result<codelet_rpc_types::WorkUnitFocus, String> {
        self.get_session(&session_id.value)
            .map(|session| session.activity_focus())
    }
    fn set_work_unit_focus(
        &self,
        session_id: &SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> Result<codelet_rpc_types::WorkUnitFocus, String> {
        self.set_session_focus(session_id, work_unit_id.as_deref(), &source)
            .map_err(|error| error.to_string())
    }
    fn work_unit_focus_rx(
        &self,
    ) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)> {
        self.work_unit_focus_tx.subscribe()
    }
    fn get_pending_input(&self, session_id: &SessionId) -> Result<Option<String>, String> {
        Ok(self.get_session(&session_id.value)?.get_pending_input())
    }
    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) -> Result<(), String> {
        // BUG-464: an unknown session is the lookup error, not a no-op.
        self.get_session(&session_id.value)?.set_pending_input(text);
        Ok(())
    }
    fn set_active_session(&self, session_id: &SessionId) -> Result<(), String> {
        // Active-session tracking works without a real session row (RPC-042
        // "Active session tracking is manager-scoped"); BUG-465: an invalid
        // id is an error, not a silent no-op.
        let uuid = uuid::Uuid::parse_str(&session_id.value)
            .map_err(|error| format!("Invalid session ID {}: {error}", session_id.value))?;
        SessionManager::set_active_session(self, uuid);
        Ok(())
    }
        }
    };
}
