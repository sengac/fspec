macro_rules! compose_websocket_provider_sessions {
    ($($collected:tt)*) => {
        compose_websocket_subscriptions! {
            $($collected)*
    async fn list_provider_settings(
        &self,
    ) -> std::result::Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .list_provider_settings(context::current())
            .await
            .map_err(|error| BackendError::ProviderSettings(error.to_string()))?
            .map_err(BackendError::ProviderSettings)
    }

    async fn save_provider_settings(
        &self,
        provider_id: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> std::result::Result<(), BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .save_provider_settings(context::current(), provider_id, definition)
            .await
            .map_err(|error| BackendError::ProviderSettings(error.to_string()))?
            .map_err(BackendError::ProviderSettings)
    }

    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_sessions(context::current(), project_path)
            .await?)
    }

    /// BUG-206: liveness gate for the auto-restore path — in-memory
    /// session IDs only, never disk.
    async fn list_live_sessions(&self) -> Result<Vec<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_live_sessions(context::current())
            .await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .create_session_checked(context::current(), role)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn session_manifest_issues(
        &self,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .session_manifest_issues(context::current())
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_input(context::current(), id, text)
            .await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client.client().interrupt(context::current(), id).await?;
        Ok(())
    }
        }
    };
}
