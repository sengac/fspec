macro_rules! compose_embedded_session_state_transcripts {
    ($($collected:tt)*) => {
        compose_embedded_focus_supervision! {
            $($collected)*
    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> {
        self.client
            .set_thinking_level(context::current(), session_id, level)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// RPC-037: per-user default thinking level — routed through the
    /// matching tarpc method now that the gap noted in the attachment
    /// (set_thinking_level_default missing from FspecService) is closed.
    async fn set_thinking_level_default(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        self.client
            .set_thinking_level_default(context::current(), session_id, level)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // CONT-002: auto-continue state — routed through the matching tarpc
    // methods (mirrors set_thinking_level in shape).
    async fn set_continue_state(
        &self,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<()> {
        self.client
            .set_continue_state(context::current(), session_id, enabled, budget)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_continue_state(&self, session_id: SessionId) -> Result<(bool, u32)> {
        Ok(self
            .client
            .get_continue_state(context::current(), session_id)
            .await?)
    }

    // CONT-003: goal chrome state — routed through the matching tarpc
    // methods (mirrors set_continue_state in shape).
    async fn set_goal_state(
        &self,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<()> {
        self.client
            .set_goal_state(context::current(), session_id, goal)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_goal_state(
        &self,
        session_id: SessionId,
    ) -> Result<Option<(String, Option<String>)>> {
        Ok(self
            .client
            .get_goal_state(context::current(), session_id)
            .await?)
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        Ok(self
            .client
            .get_session_role(context::current(), session_id)
            .await?)
    }

    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()> {
        self.client
            .set_session_role(context::current(), session_id, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ========================================================================
    // RPC-037: one-line delegates for the widened tarpc surface.
    // ========================================================================

    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        self.client
            .send_input_with_thinking(context::current(), session_id, text, thinking)
            .await?;
        Ok(())
    }

    async fn get_session_tokens(&self, session_id: SessionId) -> Result<SessionTokens> {
        Ok(self
            .client
            .get_session_tokens(context::current(), session_id)
            .await?)
    }

    async fn get_session_model(&self, session_id: SessionId) -> Result<SessionModel> {
        Ok(self
            .client
            .get_session_model(context::current(), session_id)
            .await?)
    }

    async fn get_compaction_progress(
        &self,
        session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        Ok(self
            .client
            .get_compaction_progress(context::current(), session_id)
            .await?)
    }

    async fn get_transcript_page(
        &self,
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> std::result::Result<codelet_rpc_types::TranscriptPage, super::BackendError> {
        self.client
            .get_transcript_page(
                context::current(),
                session_id,
                cursor,
                snapshot_end,
                max_bytes,
            )
            .await
            .map_err(|error| super::BackendError::TranscriptTransport(error.to_string()))?
            .map_err(super::BackendError::from)
    }

    async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        Ok(self
            .client
            .get_buffered_output(context::current(), session_id, limit)
            .await?)
    }

    async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        self.client
            .clear_history(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        self.client
            .compact_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_messages(
        &self,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<()> {
        self.client
            .restore_session_messages(context::current(), session_id, envelopes)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_token_state(
        &self,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<()> {
        self.client
            .restore_session_token_state(context::current(), session_id, state)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .resume_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
