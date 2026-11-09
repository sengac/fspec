macro_rules! compose_rpc_lifecycle_credentials {
    ($($collected:tt)*) => {
        compose_rpc_oauth_worktrees! {
            $($collected)*
    async fn send_fspec_result(
        self,
        _ctx: Context,
        session_id: SessionId,
        result: FspecResult,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.send_fspec_result(&session_id, result),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn create_isolated_session(
        self,
        _ctx: Context,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.create_isolated_session(role),
            None => Err("create_isolated_session requires a SessionManagerHandle".to_string()),
        }
    }

    async fn set_thinking_level_default(
        self,
        _ctx: Context,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_thinking_level_default(&session_id, level),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn destroy_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.destroy_session(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    // ========================================================================
    // RPC-054: Provider credentials surface forwarders.
    // ========================================================================

    async fn list_provider_credentials(self, _ctx: Context) -> Result<Vec<ProviderCredentialInfo>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_provider_credentials(),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_provider_credential(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_provider_credential(&provider_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_provider_credentials(
        self,
        _ctx: Context,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_provider_credentials(&provider_id, creds),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn delete_provider_credentials(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_provider_credentials(&provider_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn test_provider_connection(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<TestConnectionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.test_provider_connection(&provider_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn refresh_models_cache(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<Vec<ModelEntry>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.refresh_models_cache(&provider_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }
        }
    };
}
