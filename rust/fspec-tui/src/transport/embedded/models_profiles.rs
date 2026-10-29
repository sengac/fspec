macro_rules! compose_embedded_models_profiles {
    ($($collected:tt)*) => {
        compose_embedded_session_state_transcripts! {
            $($collected)*
    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        // RPC-022: one-line delegate to the shared tarpc method.
        Ok(self.client.list_providers(context::current()).await?)
    }

    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        // RPC-022: one-line delegate, lifting the String error into anyhow.
        self.client
            .set_session_model(context::current(), session_id, provider_id, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// PROV-118: set the in-process default model — one-line delegate.
    async fn set_default_model(&self, model: String) -> Result<()> {
        self.client
            .set_default_model(context::current(), model)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // RPC-347: custom-model write surface — one-line delegates.
    async fn add_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        self.client
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
        self.client
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
        self.client
            .delete_custom_model(context::current(), provider_id, profile_name, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-109: profile write surface — one-line delegates.
    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.client
            .save_profile(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        self.client
            .delete_profile(context::current(), provider_id, profile_name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-136: rename delegate.
    async fn rename_profile(
        &self,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.client
            .rename_profile(
                context::current(),
                provider_id,
                old_name,
                new_name,
                definition,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
