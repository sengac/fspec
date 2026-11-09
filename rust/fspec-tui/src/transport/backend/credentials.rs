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
        Err(BackendError::Unimplemented("list_provider_credentials").into())
    }

    /// RPC-054: read a single provider's credential summary.
    async fn get_provider_credential(
        &self,
        _provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        Err(BackendError::Unimplemented("get_provider_credential").into())
    }

    /// RPC-054: persist credentials for a provider.
    async fn set_provider_credentials(
        &self,
        _provider_id: String,
        _creds: ProviderCredentialInput,
    ) -> Result<()> {
        Err(BackendError::Unimplemented("set_provider_credentials").into())
    }

    /// RPC-054: clear credentials for a provider.
    async fn delete_provider_credentials(&self, _provider_id: String) -> Result<()> {
        Err(BackendError::Unimplemented("delete_provider_credentials").into())
    }

    /// RPC-054: round-trip a connection test to the provider.
    async fn test_provider_connection(&self, _provider_id: String) -> Result<TestConnectionResult> {
        Err(BackendError::Unimplemented("test_provider_connection").into())
    }

    /// RPC-054: refresh the provider's cached model list.
    async fn refresh_models_cache(&self, _provider_id: String) -> Result<Vec<ModelEntry>> {
        Err(BackendError::Unimplemented("refresh_models_cache").into())
    }

    /// PROV-112: clear the OAuth tokens for a provider (disconnect/logout).
    /// Both production transports forward to the connected service's
    /// provider-owned credential lifecycle. Unimplemented backends refuse.
    async fn oauth_clear_tokens(&self, _provider_id: String) -> Result<()> {
        Err(BackendError::Unimplemented("oauth_clear_tokens").into())
    }

    /// PROV-112: whether a provider currently has OAuth tokens persisted.
    /// Both production transports query the connected service's token store.
    async fn oauth_get_tokens(&self, _provider_id: String) -> Result<bool> {
        Err(BackendError::Unimplemented("oauth_get_tokens").into())
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
