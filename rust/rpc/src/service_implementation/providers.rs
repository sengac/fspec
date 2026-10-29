macro_rules! compose_rpc_providers {
    ($($collected:tt)*) => {
        compose_rpc_session_control! {
            $($collected)*
    async fn list_providers(self, _ctx: Context) -> Vec<ProviderInfo> {
        // RPC-022: delegate to the optional SessionManagerHandle. The
        // trait carries a default impl returning `Vec::new()`, so
        // callers without an attached handle get the safe sentinel —
        // matching the same defaulting pattern used by RPC-018
        // `get_model_info`.
        match self.inner.session_manager() {
            Some(handle) => handle.list_providers(),
            None => Vec::new(),
        }
    }

    async fn set_session_model(
        self,
        _ctx: Context,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        // Default impl returns `Ok(())` — silent no-op for the
        // no-handle case.
        match self.inner.session_manager() {
            Some(handle) => handle.set_model(&session_id, &provider_id, &model_id),
            None => Ok(()),
        }
    }

    /// PROV-118: delegate the in-process default-model write through the
    /// optional SessionManagerHandle; no-handle path is a silent no-op.
    async fn set_default_model(self, _ctx: Context, model: String) -> Result<(), String> {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_default_model(&model);
        }
        Ok(())
    }

    // RPC-347: custom-model write surface. Each delegates through the optional
    // SessionManagerHandle; the no-handle path returns `Ok(())` (silent no-op,
    // matching set_session_model).
    async fn add_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.add_custom_model(&provider_id, &profile_name, &definition),
            None => Ok(()),
        }
    }

    async fn update_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.update_custom_model(
                &provider_id,
                &profile_name,
                &original_model_id,
                &definition,
            ),
            None => Ok(()),
        }
    }

    async fn delete_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_custom_model(&provider_id, &profile_name, &model_id),
            None => Ok(()),
        }
    }

    async fn list_provider_settings(
        self,
        _ctx: Context,
    ) -> Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, String> {
        self.inner
            .session_manager()
            .ok_or_else(|| "list_provider_settings: no session manager attached".to_string())?
            .list_provider_settings()
    }

    async fn save_provider_settings(
        self,
        _ctx: Context,
        provider_id: String,
        definition: ProfileDefinition,
    ) -> Result<(), String> {
        self.inner
            .session_manager()
            .ok_or_else(|| "save_provider_settings: no session manager attached".to_string())?
            .save_provider_settings(&provider_id, &definition)
    }

    async fn save_profile(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.save_profile(&provider_id, &profile_name, &definition),
            None => {
                let msg = format!(
                    "save_profile({provider_id}/{profile_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }

    async fn delete_profile(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_profile(&provider_id, &profile_name),
            None => {
                let msg = format!(
                    "delete_profile({provider_id}/{profile_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }

    async fn rename_profile(
        self,
        _ctx: Context,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.rename_profile(&provider_id, &old_name, &new_name, &definition),
            None => {
                let msg = format!(
                    "rename_profile({provider_id}/{old_name}→{new_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }
        }
    };
}
