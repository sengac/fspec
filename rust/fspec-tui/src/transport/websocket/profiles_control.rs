macro_rules! compose_websocket_profiles_control {
    ($($collected:tt)*) => {
        compose_websocket_conversation! {
            $($collected)*
    // PROV-109: profile write surface — guarded delegates.
    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .save_profile(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_profile(context::current(), provider_id, profile_name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-136: rename delegate (guarded like save/delete).
    async fn rename_profile(
        &self,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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

    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_continue_state(context::current(), session_id, enabled, budget)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_continue_state(&self, session_id: SessionId) -> Result<(bool, u32)> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_goal_state(context::current(), session_id, goal)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_goal_state(
        &self,
        session_id: SessionId,
    ) -> Result<Option<(String, Option<String>)>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_goal_state(context::current(), session_id)
            .await?)
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_role(context::current(), session_id)
            .await?)
    }

    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_session_role(context::current(), session_id, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// RPC-011 rule [4]: notify the supervisor task to cancel its
    /// current backoff sleep, attempt connect immediately, and reset
    /// the backoff schedule on its next failure. Idempotent; safe to
    /// call even when the supervisor is not currently sleeping.
    fn request_manual_reconnect(&self) {
        self.manual_reconnect.notify_one();
    }
        }
    };
}
