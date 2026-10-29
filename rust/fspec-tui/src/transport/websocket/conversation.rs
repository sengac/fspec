macro_rules! compose_websocket_conversation {
    ($($collected:tt)*) => {
        compose_websocket_session_context! {
            $($collected)*
    // ========================================================================
    // RPC-037: Widened FspecBackend surface. Every method follows the
    // existing read-lock + BackendError::Disconnected guard pattern then
    // delegates to the matching tarpc method one line down.
    // ========================================================================

    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_input_with_thinking(context::current(), session_id, text, thinking)
            .await?;
        Ok(())
    }

    async fn get_session_tokens(&self, session_id: SessionId) -> Result<SessionTokens> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_tokens(context::current(), session_id)
            .await?)
    }

    async fn get_session_model(&self, session_id: SessionId) -> Result<SessionModel> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_model(context::current(), session_id)
            .await?)
    }

    async fn get_compaction_progress(
        &self,
        session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_compaction_progress(context::current(), session_id)
            .await?)
    }

    async fn get_transcript_page(
        &self,
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> std::result::Result<codelet_rpc_types::TranscriptPage, BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .get_transcript_page(
                context::current(),
                session_id,
                cursor,
                snapshot_end,
                max_bytes,
            )
            .await
            .map_err(|error| BackendError::TranscriptTransport(error.to_string()))?
            .map_err(BackendError::from)
    }

    async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_buffered_output(context::current(), session_id, limit)
            .await?)
    }

    async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .clear_history(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .compact_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_messages(
        &self,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_session_messages(context::current(), session_id, envelopes)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_token_state(
        &self,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_session_token_state(context::current(), session_id, state)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .resume_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
