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
            None => Ok(()),
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
            None => Ok(()),
        }
    }

    async fn destroy_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.destroy_session(&session_id),
            None => Ok(()),
        }
    }

    // ========================================================================
    // RPC-054: Provider credentials surface forwarders.
    // ========================================================================

    async fn list_provider_credentials(self, _ctx: Context) -> Vec<ProviderCredentialInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_provider_credentials(),
            None => Vec::new(),
        }
    }

    async fn get_provider_credential(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Option<ProviderCredentialInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_provider_credential(&provider_id),
            None => None,
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
            None => Ok(()),
        }
    }

    async fn delete_provider_credentials(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_provider_credentials(&provider_id),
            None => Ok(()),
        }
    }

    async fn test_provider_connection(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<TestConnectionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.test_provider_connection(&provider_id),
            None => Ok(TestConnectionResult {
                success: true,
                error: None,
                latency_ms: 0,
            }),
        }
    }

    async fn refresh_models_cache(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<Vec<ModelEntry>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.refresh_models_cache(&provider_id),
            None => Ok(Vec::new()),
        }
    }
        }
    };
}
