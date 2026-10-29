macro_rules! compose_backend_models {
    ($($collected:tt)*) => {
        compose_backend_session_history! {
            $($collected)*
    /// RPC-022: list providers and their models. Both transports
    /// delegate to `FspecService::list_providers` which in turn
    /// delegates to the optional `SessionManagerHandle`. Returns
    /// an empty Vec when no session manager is attached.
    async fn list_providers(&self) -> Result<Vec<ProviderInfo>>;

    /// RPC-022: set the model bound to a session. Both transports
    /// delegate to `FspecService::set_session_model`. Returns
    /// `Ok(())` (silent no-op) when no session manager is attached;
    /// `Err` when the underlying handle reports a failure.
    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()>;

    /// RPC-347: add a NEW custom model to a local-server profile. Delegates
    /// to `FspecService::add_custom_model`. Returns `Ok(())` (silent no-op)
    /// when no session manager is attached. Default body lets test mocks that
    /// do not exercise the custom-model surface compile unchanged (mirrors the
    /// RPC-037 default-impl convention); the embedded + websocket transports
    /// override it with a real delegate.
    async fn add_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        let _ = (provider_id, profile_name, definition);
        Ok(())
    }

    /// RPC-347: UPDATE an existing custom model in place, replacing the entry
    /// named by `original_model_id`. Delegates to
    /// `FspecService::update_custom_model`.
    async fn update_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        let _ = (provider_id, profile_name, original_model_id, definition);
        Ok(())
    }

    /// RPC-347: DELETE a custom model from a local-server profile by id.
    /// Delegates to `FspecService::delete_custom_model`.
    async fn delete_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<()> {
        let _ = (provider_id, profile_name, model_id);
        Ok(())
    }

    /// PROV-109: create or update a local-server profile's connection
    /// settings. Delegates to `FspecService::save_profile` (the PROV-108
    /// read-modify-write that preserves `customModels` + sibling keys).
    /// Returns `Ok(())` (silent no-op) when no session manager is attached;
    /// the default body lets test mocks that do not exercise the profile
    /// surface compile unchanged (mirrors the RPC-347 convention). The
    /// embedded + websocket transports override it with a real delegate.
    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: ProfileDefinition,
    ) -> Result<()> {
        let _ = (provider_id, profile_name, definition);
        Ok(())
    }

    /// PROV-109: delete a local-server profile by name. Delegates to
    /// `FspecService::delete_profile`.
    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        let _ = (provider_id, profile_name);
        Ok(())
    }

    /// PROV-136: rename a local-server profile (or apply an in-place update when
    /// `old_name == new_name`). Delegates to `FspecService::rename_profile`
    /// (the persistence-layer single read-modify-write that moves the old key to
    /// the new name preserving `customModels`, and rejects a collision with an
    /// existing different profile). Returns `Ok(())` (silent no-op) when no
    /// session manager is attached; the default body lets test mocks that do not
    /// exercise the rename surface compile unchanged (mirrors the `save_profile`
    /// convention). The embedded + websocket transports override it with a real
    /// delegate.
    async fn rename_profile(
        &self,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: ProfileDefinition,
    ) -> Result<()> {
        let _ = (provider_id, old_name, new_name, definition);
        Ok(())
    }

    /// RPC-022: set the per-session thinking/reasoning level.
    /// Mirrors `set_session_model` in shape.
    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()>;

    /// RPC-027: set the per-user DEFAULT thinking/reasoning level.
    /// Sister of `set_thinking_level`; persists the level so subsequent
    /// sessions inherit it. Default impl is `Ok(())` (silent no-op)
    /// so embedded transports without a session manager attached
    /// compile unchanged.
    async fn set_thinking_level_default(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        let _ = (session_id, level);
        Ok(())
    }

    /// PROV-118: set the in-process DEFAULT model used by `create_session`
    /// when no per-session model is bound. Used by the no-session `/model`
    /// selection path to break the chicken-and-egg deadlock (no default model
    /// → create_session declined → no session → selection dropped). Both
    /// transports delegate to `FspecService::set_default_model`. Default impl
    /// is `Ok(())` (silent no-op) so mock/test backends compile unchanged —
    /// mirrors `set_thinking_level_default`. PROV-101 no-fallback policy is
    /// preserved downstream (empty strings ignored, no anthropic fallback).
    async fn set_default_model(&self, model: String) -> Result<()> {
        let _ = model;
        Ok(())
    }
        }
    };
}
