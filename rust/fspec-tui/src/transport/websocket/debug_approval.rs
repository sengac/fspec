macro_rules! compose_websocket_debug_approval {
    ($($collected:tt)*) => {
        compose_websocket_isolation_credentials_worktrees! {
            $($collected)*
    async fn get_debug_enabled(&self, session_id: SessionId) -> Result<bool> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_debug_enabled(context::current(), session_id)
            .await?)
    }

    async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_debug_enabled(context::current(), session_id, enabled)
            .await?;
        Ok(())
    }

    async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .toggle_debug(context::current(), session_id, debug_dir)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_debug_directory(&self, path: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_debug_directory(context::current(), path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_resume(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_resume(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_confirm(&self, session_id: SessionId, accept: bool) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_confirm(context::current(), session_id, accept)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_triple(&self, session_id: SessionId, choice: ApprovalChoice) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_triple(context::current(), session_id, choice)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn send_hitl_response(
        &self,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_hitl_response(context::current(), session_id, response)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PauseState>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_pause_state(context::current(), session_id)
            .await?)
    }

    async fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<HitlRequest>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_hitl_request(context::current(), session_id)
            .await?)
    }

    async fn get_exec_stdin_request(
        &self,
        session_id: SessionId,
    ) -> Result<Option<ExecStdinRequest>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_exec_stdin_request(context::current(), session_id)
            .await?)
    }

    async fn write_exec_stdin(
        &self,
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String> {
        let guard = self.client.read().await;
        // The method's error type is `String` (the backend surfaces its
        // business error verbatim), so the `Disconnected` sentinel maps
        // to its Display string rather than propagating the typed error.
        let client = guard
            .as_ref()
            .ok_or_else(|| BackendError::Disconnected.to_string())?;
        // The tarpc service method returns `Result<(), String>` wrapped
        // in the transport's own `Result<_, tarpc::Error>` — surface the
        // transport error as its Display string, then unwrap the
        // business result (which already names the exec session id).
        client
            .client()
            .write_exec_stdin(context::current(), session_id, exec_session_id, text)
            .await
            .map_err(|e| e.to_string())?
    }

    async fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_fspec_result(context::current(), session_id, result)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
