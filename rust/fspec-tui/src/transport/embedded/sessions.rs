macro_rules! compose_embedded_sessions {
    ($($collected:tt)*) => {
        compose_embedded_workspace_history! {
            $($collected)*
    async fn list_provider_settings(
        &self,
    ) -> std::result::Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, super::BackendError>
    {
        self.client
            .list_provider_settings(context::current())
            .await
            .map_err(|error| super::BackendError::ProviderSettings(error.to_string()))?
            .map_err(super::BackendError::ProviderSettings)
    }

    async fn save_provider_settings(
        &self,
        provider_id: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> std::result::Result<(), super::BackendError> {
        self.client
            .save_provider_settings(context::current(), provider_id, definition)
            .await
            .map_err(|error| super::BackendError::ProviderSettings(error.to_string()))?
            .map_err(super::BackendError::ProviderSettings)
    }

    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(self.client.list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>> {
        Ok(self
            .client
            .list_sessions(context::current(), project_path)
            .await?)
    }

    /// BUG-206: liveness gate for the auto-restore path — in-memory
    /// session IDs only, never disk.
    async fn list_live_sessions(&self) -> Result<Vec<SessionId>> {
        Ok(self.client.list_live_sessions(context::current()).await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        self.client
            .create_session_checked(context::current(), role)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn session_manifest_issues(
        &self,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>> {
        self.client
            .session_manifest_issues(context::current())
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.client.send_input(context::current(), id, text).await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.client.interrupt(context::current(), id).await?;
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.transport.work_units_rx()
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.transport.chunks_rx()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.transport.logs_rx()
    }
        }
    };
}
