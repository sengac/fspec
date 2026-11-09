macro_rules! compose_handle_trait_2 {
    ($($collected:tt)*) => {
        compose_handle_trait_3! {
            $($collected)*
    /// CONT-003: set or clear the session's goal chrome state (`/goal`) as
    /// `(text, verify)`. Default returns `Ok(())` (silent no-op) so handles
    /// that have not wired goal state — including the stub — compile without
    /// per-test wiring.
    fn set_goal_state(
        &self,
        session_id: &SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String> {
        let _ = (session_id, goal);
        Ok(())
    }
    /// CONT-003: read the session's goal chrome state as `(text, verify)`.
    /// Default returns `None` so unwired handles compile unchanged.
    fn get_goal_state(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<(String, Option<String>)>, String> {
        let _ = session_id;
        Err("get_goal_state is not implemented by this session manager".to_string())
    }
    /// RPC-022: read the session's current role overlay text. Default
    /// returns `None` so handles that have not yet wired role state —
    /// including the stub — compile without per-test wiring. The
    /// rust/napi override forwards to the existing
    /// `session_get_role` flow (which returns
    /// `Option<SupervisorRoleInfo>` on the JS surface).
    fn get_role(&self, session_id: &SessionId) -> Result<Option<String>, String> {
        let _ = session_id;
        Err("get_role is not implemented by this session manager".to_string())
    }
    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears. Default returns `Ok(())` (silent no-op). The
    /// rust/napi override forwards to the existing
    /// `session_set_role` / `session.clear_role` flow.
    fn set_role(&self, session_id: &SessionId, role: Option<String>) -> Result<(), String> {
        let _ = (session_id, role);
        Err("set_role is not implemented by this session manager".to_string())
    }
    // ========================================================================
    // RPC-037: Widened surface for AgentView parity.
    // Every new method declares a default body so existing handles compile
    // unchanged. The rust/napi `SessionManager` will override these in
    // Phase 4 (RPC-042) when the agent loop is extracted into codelet-sessions.
    // ========================================================================

    /// RPC-037: send user input with an optional provider-specific
    /// thinking config. The default delegates to `send_input` so
    /// implementers only need to override one method.
    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<(), String> {
        let _ = thinking;
        self.send_input(session_id, text)
    }
    /// RPC-037: per-session input + output token totals.
    fn get_session_tokens(&self, session_id: &SessionId) -> Result<SessionTokens, String> {
        let _ = session_id;
        Err("get_session_tokens is not implemented by this session manager".to_string())
    }
    /// RPC-037: per-session model binding (provider + model + limits).
    fn get_session_model(&self, session_id: &SessionId) -> Result<SessionModel, String> {
        let _ = session_id;
        Err("get_session_model is not implemented by this session manager".to_string())
    }
    /// RPC-037: in-flight compaction progress, if any.
    fn get_compaction_progress(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<CompactionProgress>, String> {
        let _ = session_id;
        Err("get_compaction_progress is not implemented by this session manager".to_string())
    }
    fn get_transcript_page(
        &self,
        session_id: &SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<codelet_rpc_types::TranscriptPage, codelet_rpc_types::TranscriptError> {
        let _ = (cursor, snapshot_end, max_bytes);
        Err(codelet_rpc_types::TranscriptError::SessionUnavailable(
            session_id.value.clone(),
        ))
    }
    /// RPC-037: replay-buffer of recent stream chunks for a session.
    fn get_buffered_output(
        &self,
        session_id: &SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>, String> {
        let _ = (session_id, limit);
        Err("get_buffered_output is not implemented by this session manager".to_string())
    }
    /// RPC-037: clear session history.
    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Err("clear_history is not implemented by this session manager".to_string())
    }
    /// RPC-037: compact session history; returns the canned result so
    /// the AgentView can display compression statistics.
    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        let _ = session_id;
        Err("compact_session is not implemented by this session manager".to_string())
    }
    /// RPC-037: restore session messages from raw JSONL envelopes (used by `/resume`).
    fn restore_session_messages(
        &self,
        session_id: &SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        let _ = (session_id, envelopes);
        Err("restore_session_messages is not implemented by this session manager".to_string())
    }
    /// RPC-037: restore the cumulative-billed counters + cache totals (used by `/resume`).
    fn restore_session_token_state(
        &self,
        session_id: &SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String> {
        let _ = (session_id, state);
        Err("restore_session_token_state is not implemented by this session manager".to_string())
    }
    /// RPC-049: durable-restore round-trip used by `/resume`. Loads the
    /// on-disk session manifest + envelopes via
    /// `codelet_core::persistence`, builds a `TokenRestoreState` from
    /// the manifest's `token_usage`, then calls
    /// [`Self::restore_session_messages`] +
    /// [`Self::restore_session_token_state`].
    ///
    /// Errors at any step propagate as `Result<(), String>` so the
    /// caller (TUI `Action::AttachToSession` handler) can surface them
    /// via `Action::EmitSessionNotice`.
    fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(&session_id.value)
            .map_err(|e| format!("invalid session id: {e}"))?;
        let manifest = crate::persistence::load_session(uuid)?;
        let envelopes = crate::persistence::get_session_message_envelopes(uuid)?;
        let state = TokenRestoreState {
            current_context: manifest.token_usage.current_context_tokens as i64,
            cumulative_billed_output: manifest.token_usage.cumulative_billed_output as i64,
            cache_read: manifest.token_usage.cache_read_tokens as i64,
            cache_creation: manifest.token_usage.cache_creation_tokens as i64,
            cumulative_billed_input: manifest.token_usage.cumulative_billed_input as i64,
            cumulative_billed_output_second: manifest.token_usage.cumulative_billed_output as i64,
        };
        self.restore_session_messages(session_id, envelopes)?;
        self.restore_session_token_state(session_id, state)?;
        Ok(())
    }
    fn get_work_unit_focus(&self, session_id: &SessionId) -> Result<WorkUnitFocus, String> {
        Err(format!("Session focus unavailable: {}", session_id.value))
    }
    fn set_work_unit_focus(
        &self,
        session_id: &SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> Result<WorkUnitFocus, String> {
        let _ = (work_unit_id, source);
        Err(format!("Session focus unavailable: {}", session_id.value))
    }
    fn work_unit_focus_rx(&self) -> broadcast::Receiver<(SessionId, WorkUnitFocus)> {
        let (sender, receiver) = broadcast::channel(1);
        drop(sender);
        receiver
    }
    /// RPC-037: read the work-unit context bound to a session.
    fn get_work_unit_context(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<WorkUnitContext>, String> {
        let _ = session_id;
        Err("get_work_unit_context is not implemented by this session manager".to_string())
    }
    /// RPC-037: bind (or detach) a work unit on a session.
    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        let _ = (session_id, ctx);
        Err("set_work_unit_context is not implemented by this session manager".to_string())
    }
    /// RPC-037: read the per-session pending input draft.
    fn get_pending_input(&self, session_id: &SessionId) -> Result<Option<String>, String> {
        let _ = session_id;
        Err("get_pending_input is not implemented by this session manager".to_string())
    }
    /// RPC-037: write the per-session pending input draft.
    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) -> Result<(), String> {
        let _ = (session_id, text);
        Err("set_pending_input is not implemented by this session manager".to_string())
    }
    /// BUG-206: the session IDs of every session that is currently
    /// ALIVE in the handle's in-memory session map. Used by the
    /// TUI's auto-restore path (bootstrap + reconnect) as the liveness
    /// gate for restore candidates — a candidate session that is not
    /// in this set is NOT running in the daemon right now (closed or
    /// manifest-only) and must never be re-opened / resumed.
    ///
    /// The default returns an empty list so handles without a session
    /// table (`StubSessionManagerHandle` overrides; test-only handles)
    /// compile unchanged. Implementations MUST NOT read persisted
    /// manifests — this list is the daemon's live memory, period.
    fn list_live_sessions(&self) -> Result<Vec<SessionId>, String> {
        Err("list_live_sessions is not implemented by this session manager".to_string())
    }
    /// RPC-037: set the active session for the application.
    fn set_active_session(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Err("set_active_session is not implemented by this session manager".to_string())
    }
    /// RPC-037: clear the active session.
    fn clear_active_session(&self) -> Result<(), String> {
        Err("clear_active_session is not implemented by this session manager".to_string())
    }

    /// RPC-037: read the active session, if any.
    fn get_active_session(&self) -> Result<Option<SessionId>, String> {
        Err("get_active_session is not implemented by this session manager".to_string())
    }
    /// RPC-037: effective cwd for a session (worktree-aware). BUG-437: a
    /// handle that cannot answer refuses (never an empty path).
    fn get_effective_cwd(&self, session_id: &SessionId) -> Result<PathBuf, String> {
        Err(format!(
            "the effective cwd of session {} is not available from this session manager handle",
            session_id.value
        ))
    }
    /// RPC-037: list supervisor session ids for a subordinate.
    fn get_supervisors(&self, session_id: &SessionId) -> Result<Vec<SessionId>, String> {
        let _ = session_id;
        Err("get_supervisors is not implemented by this session manager".to_string())
    }
    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. Default is `Ok(())` so handles that don't
    /// track supervisor links (the embedded stub used by some unit
    /// tests, future read-only handles, etc.) compile unchanged.
    /// Production handles (`StubSessionManagerHandle` +
    /// `codelet_sessions::SessionManager`) override this to delegate
    /// into `ChainOfCommand::add_supervisor` and surface its
    /// "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" error strings verbatim.
    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        let _ = (subordinate_id, supervisor_id);
        Ok(())
    }
        }
    };
}
