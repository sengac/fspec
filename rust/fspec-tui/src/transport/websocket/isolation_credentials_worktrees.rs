macro_rules! compose_websocket_isolation_credentials_worktrees {
    ($($collected:tt)*) => {
        compose_websocket_scheduling_loops! {
            $($collected)*
    async fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .create_isolated_session(context::current(), role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .destroy_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ========================================================================
    // RPC-054: Provider credentials surface — one-line forwarders with the
    // standard `Disconnected` guard pattern used by every other WebSocket
    // RPC method.
    // ========================================================================

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_provider_credentials(context::current())
            .await?)
    }

    async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_provider_credential(context::current(), provider_id)
            .await?)
    }

    async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_provider_credentials(context::current(), provider_id, creds)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_provider_credentials(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .test_provider_connection(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .refresh_models_cache(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().blocklist_list(context::current()).await?)
    }

    async fn merge_session_worktree(
        &self,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .merge_session_worktree(context::current(), session_id, strategy)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn discard_session_worktree(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .discard_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .prune_orphaned_worktrees(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_session_worktrees(context::current())
            .await?)
    }

    async fn inspect_session_changes(
        &self,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .inspect_session_changes(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn detach_session_worktree(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .detach_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
