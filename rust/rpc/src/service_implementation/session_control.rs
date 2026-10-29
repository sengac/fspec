macro_rules! compose_rpc_session_control {
    ($($collected:tt)*) => {
        compose_rpc_session_restore_focus! {
            $($collected)*
    async fn set_thinking_level(
        self,
        _ctx: Context,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_thinking_level(&session_id, level),
            None => Ok(()),
        }
    }

    async fn set_continue_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String> {
        // CONT-002: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_continue_state(&session_id, enabled, budget),
            None => Ok(()),
        }
    }

    async fn get_continue_state(self, _ctx: Context, session_id: SessionId) -> (bool, u32) {
        // CONT-002: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.get_continue_state(&session_id),
            None => (false, 10),
        }
    }

    async fn set_goal_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String> {
        // CONT-003: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_goal_state(&session_id, goal),
            None => Ok(()),
        }
    }

    async fn get_goal_state(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<(String, Option<String>)> {
        // CONT-003: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.get_goal_state(&session_id),
            None => None,
        }
    }

    async fn get_session_role(self, _ctx: Context, session_id: SessionId) -> Option<String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        // Default impl returns `None` so callers without an attached
        // handle paint no banner — matching the RPC-018 safe-defaults
        // pattern.
        match self.inner.session_manager() {
            Some(handle) => handle.get_role(&session_id),
            None => None,
        }
    }

    async fn set_session_role(
        self,
        _ctx: Context,
        session_id: SessionId,
        role: Option<String>,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_role(&session_id, role),
            None => Ok(()),
        }
    }

    // ========================================================================
    // RPC-037: Widened tarpc surface implementations. Each delegates through
    // the optional SessionManagerHandle; safe defaults are returned when no
    // handle is attached so existing handle-less tests stay green.
    // ========================================================================

    async fn send_input_with_thinking(
        self,
        _ctx: Context,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) {
        if let Some(handle) = self.inner.session_manager() {
            handle.send_input_with_thinking(&session_id, text, thinking);
        }
    }

    async fn get_session_tokens(self, _ctx: Context, session_id: SessionId) -> SessionTokens {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_tokens(&session_id),
            None => SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            },
        }
    }

    async fn get_session_model(self, _ctx: Context, session_id: SessionId) -> SessionModel {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_model(&session_id),
            None => SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: 0,
                max_output_tokens: 0,
                compaction_threshold: 0,
            },
        }
    }

    async fn get_compaction_progress(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<CompactionProgress> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_compaction_progress(&session_id),
            None => None,
        }
    }

    async fn get_transcript_page(
        self,
        _ctx: Context,
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<codelet_rpc_types::TranscriptPage, codelet_rpc_types::TranscriptError> {
        self.inner
            .session_manager()
            .ok_or_else(|| {
                codelet_rpc_types::TranscriptError::SessionUnavailable(session_id.value.clone())
            })?
            .get_transcript_page(&session_id, cursor, snapshot_end, max_bytes)
    }

    async fn get_buffered_output(
        self,
        _ctx: Context,
        session_id: SessionId,
        limit: u32,
    ) -> Vec<StreamChunk> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_buffered_output(&session_id, limit),
            None => Vec::new(),
        }
    }
        }
    };
}
