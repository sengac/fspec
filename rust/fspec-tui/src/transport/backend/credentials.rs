macro_rules! compose_backend_credentials {
    ($($collected:tt)*) => {
        compose_backend_worktrees_jobs! {
            $($collected)*
    // ========================================================================
    // RPC-054: Provider credentials surface. Mirrors the trait additions on
    // `SessionManagerHandle` / `FspecService`. Both backends override these
    // with one-line forwarders to the tarpc client; the default impls here
    // keep test doubles and embedded transports without a session manager
    // attached compiling unchanged.
    // ========================================================================

    /// RPC-054: list provider credential summaries.
    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        Ok(Vec::new())
    }

    /// RPC-054: read a single provider's credential summary.
    async fn get_provider_credential(
        &self,
        _provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        Ok(None)
    }

    /// RPC-054: persist credentials for a provider.
    async fn set_provider_credentials(
        &self,
        _provider_id: String,
        _creds: ProviderCredentialInput,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-054: clear credentials for a provider.
    async fn delete_provider_credentials(&self, _provider_id: String) -> Result<()> {
        Ok(())
    }

    /// RPC-054: round-trip a connection test to the provider.
    async fn test_provider_connection(&self, _provider_id: String) -> Result<TestConnectionResult> {
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        })
    }

    /// RPC-054: refresh the provider's cached model list.
    async fn refresh_models_cache(&self, _provider_id: String) -> Result<Vec<ModelEntry>> {
        Ok(Vec::new())
    }

    /// PROV-112: clear the OAuth tokens for a provider (disconnect/logout).
    /// The embedded transport overrides this to forward to the providers-
    /// direct `FspecService::oauth_clear_tokens`; the websocket transport and
    /// test doubles inherit this no-op default (browser/headless/device OAuth
    /// is only available off the embedded transport — gating consumed by
    /// PROV-113/114). Idempotent; errors are surfaced as `Err` and swallowed
    /// by the dispatch layer so no RPC/method name leaks into the UI.
    async fn oauth_clear_tokens(&self, _provider_id: String) -> Result<()> {
        Ok(())
    }

    /// PROV-112: whether a provider currently has OAuth tokens persisted.
    /// Embedded forwards to `FspecService::oauth_get_tokens`; the websocket
    /// transport and test doubles inherit this `false` default.
    async fn oauth_get_tokens(&self, _provider_id: String) -> Result<bool> {
        Ok(false)
    }

    /// PROV-113: whether this transport can drive the browser OAuth login
    /// (the providers-layer local HTTP server must run on the user's own
    /// machine). The embedded transport overrides this to `true`; the
    /// websocket transport and test doubles inherit `false`, which gates the
    /// browser login rows out of the nav tree (headless/device rows remain).
    fn supports_browser_oauth(&self) -> bool {
        false
    }

    /// PROV-113: run the browser OAuth login to completion. Embedded forwards
    /// to the providers-direct `FspecService::oauth_browser_login`; the default
    /// is the unsupported stub (browser login is embedded-only).
    async fn oauth_browser_login(&self, _provider_id: String) -> Result<()> {
        Err(anyhow::anyhow!("browser OAuth login is not supported"))
    }

    /// PROV-113: phase 1 of the anthropic headless flow (PKCE + authorize URL).
    async fn oauth_headless_start(&self, _provider_id: String) -> Result<OAuthHeadlessStart> {
        Err(anyhow::anyhow!("headless OAuth login is not supported"))
    }

    /// PROV-113: phase 2 of the anthropic headless flow (exchange + persist).
    async fn oauth_headless_complete(
        &self,
        _provider_id: String,
        _code_with_state: String,
        _pkce_verifier: String,
    ) -> Result<()> {
        Err(anyhow::anyhow!("headless OAuth login is not supported"))
    }

    /// PROV-113: phase 1 of the codex device flow (request device code).
    async fn oauth_device_start(&self, _provider_id: String) -> Result<OAuthDeviceStart> {
        Err(anyhow::anyhow!("device OAuth login is not supported"))
    }

    /// PROV-113: phase 2 of the codex device flow (poll + exchange + persist).
    async fn oauth_device_poll(
        &self,
        _provider_id: String,
        _device_auth_id: String,
        _interval: u64,
    ) -> Result<()> {
        Err(anyhow::anyhow!("device OAuth login is not supported"))
    }

    /// PROV-114: phase 1 of the github-copilot device flow. `enterprise_host`
    /// is `None` for GitHub.com or `Some(normalized_host)` for GitHub
    /// Enterprise. Embedded forwards to the providers-direct copilot
    /// device-code request; the default is the unsupported stub.
    async fn oauth_copilot_device_start(
        &self,
        _enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart> {
        Err(anyhow::anyhow!("device OAuth login is not supported"))
    }
        }
    };
}
