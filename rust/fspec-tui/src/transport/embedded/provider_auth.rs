macro_rules! compose_embedded_provider_auth {
    ($($collected:tt)*) => {
        compose_embedded_worktrees_scheduling! {
            $($collected)*
    // ========================================================================
    // RPC-054: Provider credentials surface — one-line forwarders.
    // ========================================================================

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        Ok(self
            .client
            .list_provider_credentials(context::current())
            .await?)
    }

    async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        Ok(self
            .client
            .get_provider_credential(context::current(), provider_id)
            .await?)
    }

    async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        self.client
            .set_provider_credentials(context::current(), provider_id, creds)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        self.client
            .delete_provider_credentials(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult> {
        self.client
            .test_provider_connection(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        self.client
            .refresh_models_cache(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_clear_tokens(&self, provider_id: String) -> Result<()> {
        // PROV-112: napi-direct via the providers-backed RPC method.
        self.client
            .oauth_clear_tokens(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_get_tokens(&self, provider_id: String) -> Result<bool> {
        self.client
            .oauth_get_tokens(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn supports_browser_oauth(&self) -> bool {
        // PROV-113: the providers-layer local HTTP server runs in-process on
        // the embedded transport, so the browser login rows are available.
        true
    }

    async fn oauth_browser_login(&self, provider_id: String) -> Result<()> {
        self.client
            .oauth_browser_login(super::oauth_context::interactive(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_headless_start(&self, provider_id: String) -> Result<OAuthHeadlessStart> {
        self.client
            .oauth_headless_start(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_headless_complete(
        &self,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<()> {
        self.client
            .oauth_headless_complete(
                context::current(),
                provider_id,
                code_with_state,
                pkce_verifier,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_device_start(&self, provider_id: String) -> Result<OAuthDeviceStart> {
        self.client
            .oauth_device_start(super::oauth_context::interactive(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_device_poll(
        &self,
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<()> {
        self.client
            .oauth_device_poll(
                super::oauth_context::interactive(),
                provider_id,
                device_auth_id,
                interval,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_copilot_device_start(
        &self,
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart> {
        self.client
            .oauth_copilot_device_start(context::current(), enterprise_host)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
