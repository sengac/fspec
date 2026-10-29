macro_rules! compose_backend_session_history {
    ($($collected:tt)*) => {
        compose_backend_session_context! {
            $($collected)*
    /// CONT-002: set the session's auto-continue state (`/continue`).
    /// Both transports delegate to `FspecService::set_continue_state`.
    /// Default impl is `Ok(())` (silent no-op) so mock/test backends
    /// compile unchanged — mirrors `set_thinking_level_default`.
    async fn set_continue_state(
        &self,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<()> {
        let _ = (session_id, enabled, budget);
        Ok(())
    }

    /// CONT-002: read the session's auto-continue state as
    /// `(enabled, budget)`. Default impl returns `(false, 10)` (off,
    /// default budget) so mock/test backends compile unchanged.
    async fn get_continue_state(&self, session_id: SessionId) -> Result<(bool, u32)> {
        let _ = session_id;
        Ok((false, 10))
    }

    /// CONT-003: set or clear the session's goal chrome state (`/goal`)
    /// as `(text, verify)`. Both transports delegate to
    /// `FspecService::set_goal_state`. Default impl is `Ok(())` (silent
    /// no-op) so mock/test backends compile unchanged.
    async fn set_goal_state(
        &self,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<()> {
        let _ = (session_id, goal);
        Ok(())
    }

    /// CONT-003: read the session's goal chrome state as
    /// `(text, verify)`. Default impl returns `None` so mock/test
    /// backends compile unchanged.
    async fn get_goal_state(
        &self,
        session_id: SessionId,
    ) -> Result<Option<(String, Option<String>)>> {
        let _ = session_id;
        Ok(None)
    }

    /// RPC-022: read the session's current role overlay text. Both
    /// transports delegate to `FspecService::get_session_role`.
    /// Returns `Ok(None)` when no role is active OR when no session
    /// manager is attached.
    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>>;

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears.
    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()>;

    /// RPC-011 rule [4]: trigger the transport's manual-reconnect signal
    /// (resets the backoff schedule + cancels any in-flight backoff
    /// sleep). Wired to the App's `r`-press handler from the
    /// DisconnectDialog so pressing `r` while disconnected immediately
    /// attempts reconnect rather than waiting for the next backoff tick.
    ///
    /// Default impl is a no-op so embedded and other transports without
    /// a reconnect supervisor (where the call has no meaning) don't need
    /// to override.
    fn request_manual_reconnect(&self) {}

    // ========================================================================
    // RPC-037: Widened FspecBackend surface for AgentView parity. Every
    // method below has a peer on `FspecService` (rust/rpc); both
    // backends delegate as one-line wrappers. WebSocket variants follow the
    // existing `client.read().await + BackendError::Disconnected` guard
    // pattern established by the earlier RPC cards.
    // ========================================================================

    /// RPC-037: send user input with optional thinking config.
    /// Default delegates to `send_input` when `thinking` is `None`, and
    /// otherwise to the same delegate (test mocks override only the
    /// fields they care about).
    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        _thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        self.send_input(session_id, text).await
    }

    /// RPC-037: per-session input/output token totals.
    async fn get_session_tokens(&self, _session_id: SessionId) -> Result<SessionTokens> {
        Ok(SessionTokens {
            input_tokens: 0,
            output_tokens: 0,
        })
    }

    /// RPC-037: per-session model binding.
    async fn get_session_model(&self, _session_id: SessionId) -> Result<SessionModel> {
        Ok(SessionModel {
            provider_id: String::new(),
            model_id: String::new(),
            context_window: 0,
            max_output_tokens: 0,
            compaction_threshold: 0,
        })
    }

    /// RPC-037: in-flight compaction progress.
    async fn get_compaction_progress(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        Ok(None)
    }

    async fn get_transcript_page(
        &self,
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> std::result::Result<codelet_rpc_types::TranscriptPage, BackendError> {
        let _ = (cursor, snapshot_end, max_bytes);
        Err(codelet_rpc_types::TranscriptError::SessionUnavailable(session_id.value).into())
    }

    /// RPC-037: replay-buffer of recent stream chunks for a session.
    async fn get_buffered_output(
        &self,
        _session_id: SessionId,
        _limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        Ok(Vec::new())
    }

    /// RPC-037: clear session history.
    async fn clear_history(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: compact session history.
    async fn compact_session(&self, _session_id: SessionId) -> Result<CompactionResult> {
        Ok(CompactionResult {
            original_tokens: 0,
            compacted_tokens: 0,
            // RPC-420: compression_ratio is the percent of tokens
            // removed [0,100]; 0.0 is the valid "nothing removed"
            // sentinel and renders as `COMPACTED 0%` / `0.0% reduction`.
            compression_ratio: 0.0,
            turns_summarized: 0,
            turns_kept: 0,
        })
    }

    /// RPC-037: restore session messages from raw JSONL envelopes.
    async fn restore_session_messages(
        &self,
        _session_id: SessionId,
        _envelopes: Vec<String>,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: restore cumulative-billed counters and cache totals.
    async fn restore_session_token_state(
        &self,
        _session_id: SessionId,
        _state: TokenRestoreState,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-049: durable-restore aggregate used by the TUI `/resume`
    /// flow. Default returns Ok(()) so embedded and other transports
    /// without a real session manager attached don't need to override.
    async fn resume_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }
        }
    };
}
