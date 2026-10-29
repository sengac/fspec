macro_rules! compose_embedded_interaction {
    ($($collected:tt)*) => {
        compose_embedded_provider_auth! {
            $($collected)*
    async fn get_debug_enabled(&self, session_id: SessionId) -> Result<bool> {
        Ok(self
            .client
            .get_debug_enabled(context::current(), session_id)
            .await?)
    }

    async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        self.client
            .set_debug_enabled(context::current(), session_id, enabled)
            .await?;
        Ok(())
    }

    async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        self.client
            .toggle_debug(context::current(), session_id, debug_dir)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_debug_directory(&self, path: String) -> Result<()> {
        self.client
            .set_debug_directory(context::current(), path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_resume(&self, session_id: SessionId) -> Result<()> {
        self.client
            .pause_resume(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_confirm(&self, session_id: SessionId, accept: bool) -> Result<()> {
        self.client
            .pause_confirm(context::current(), session_id, accept)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_triple(&self, session_id: SessionId, choice: ApprovalChoice) -> Result<()> {
        self.client
            .pause_triple(context::current(), session_id, choice)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn send_hitl_response(
        &self,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<()> {
        self.client
            .send_hitl_response(context::current(), session_id, response)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PauseState>> {
        Ok(self
            .client
            .get_pause_state(context::current(), session_id)
            .await?)
    }

    async fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<HitlRequest>> {
        Ok(self
            .client
            .get_hitl_request(context::current(), session_id)
            .await?)
    }

    async fn get_exec_stdin_request(
        &self,
        session_id: SessionId,
    ) -> Result<Option<ExecStdinRequest>> {
        Ok(self
            .client
            .get_exec_stdin_request(context::current(), session_id)
            .await?)
    }

    async fn write_exec_stdin(
        &self,
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String> {
        // The tarpc `FspecService::write_exec_stdin` returns
        // `Result<(), String>` verbatim — the error already names the
        // offending exec session id (no reaper-race noise). A transport
        // failure is surfaced as its Display string.
        self.client
            .write_exec_stdin(context::current(), session_id, exec_session_id, text)
            .await
            .map_err(|e| e.to_string())?
    }

    async fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()> {
        self.client
            .send_fspec_result(context::current(), session_id, result)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo> {
        self.client
            .create_isolated_session(context::current(), role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .destroy_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
