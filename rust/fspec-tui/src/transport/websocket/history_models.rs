macro_rules! compose_websocket_history_models {
    ($($collected:tt)*) => {
        compose_websocket_profiles_control! {
            $($collected)*
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        // RPC-025: route through the shared tarpc method.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_add_history(context::current(), session, text)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_get_history(context::current(), session, limit)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_search_history(context::current(), query)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        // RPC-026: route through the shared tarpc method, returning
        // Disconnected when the supervisor has dropped the client slot.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_delete_session(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        // RPC-022: route through the shared tarpc method.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_providers(context::current()).await?)
    }

    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_session_model(context::current(), session_id, provider_id, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// PROV-118: set the in-process default model — same read().await +
    /// Disconnected guard pattern as the rest of the WebSocket backend.
    async fn set_default_model(&self, model: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_default_model(context::current(), model)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // RPC-347: custom-model write surface — same read().await + Disconnected
    // guard pattern as the rest of the WebSocket backend.
    async fn add_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .add_custom_model(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn update_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .update_custom_model(
                context::current(),
                provider_id,
                profile_name,
                original_model_id,
                definition,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_custom_model(context::current(), provider_id, profile_name, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
