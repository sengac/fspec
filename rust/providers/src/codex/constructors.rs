use super::{
    codex_auth, codex_oauth, CodexAuthMode, CodexProvider, CodexResponsesClient,
    CodexResponsesModel, RefreshingCodexClient,
};
use crate::{validate_api_key_static, ProviderError};

impl CodexProvider {
    /// Create a new CodexProvider using credentials from ~/.codex/auth.json
    ///
    /// Reads credentials from:
    /// 1. macOS keychain (if on macOS)
    /// 2. ~/.codex/auth.json file
    /// 3. $CODEX_HOME/auth.json if CODEX_HOME is set
    ///
    /// Two modes:
    /// 1. **Direct Codex API** (preferred): If OAuth tokens with account_id exist,
    ///    route requests to chatgpt.com/backend-api/codex with Bearer token.
    /// 2. **Legacy token-exchange**: If cached OPENAI_API_KEY exists, use standard
    ///    OpenAI API. If not, refresh tokens and exchange for API key.
    ///
    /// Model MUST be provided via CODEX_MODEL environment variable.
    pub fn new() -> Result<Self, ProviderError> {
        // Model is REQUIRED via CODEX_MODEL env var
        let model_name = std::env::var("CODEX_MODEL").map_err(|_| {
            ProviderError::config(
                "codex",
                "Model is required. Set CODEX_MODEL environment variable.",
            )
        })?;

        // Read existing auth data
        let auth = codex_auth::read_codex_auth()
            .map_err(|e| ProviderError::auth("codex", format!("{e}")))?;

        if let Some(auth_data) = &auth {
            // Mode 1: Direct Codex API - use OAuth tokens with access_token + account_id
            if let Some(tokens) = &auth_data.tokens {
                if !tokens.access_token.is_empty() && !tokens.account_id.is_empty() {
                    let provider = Self::from_oauth_tokens(
                        &tokens.access_token,
                        &tokens.refresh_token,
                        &tokens.account_id,
                        Some(
                            super::auth_refresh::stored_lifetime(&tokens.access_token)
                                .map_err(|error| ProviderError::auth("codex", error.to_string()))?,
                        ),
                        codex_oauth::CODEX_ISSUER,
                        &model_name,
                    )?;
                    provider
                        .client()
                        .http_client()
                        .require_stored_binding()
                        .map_err(|error| ProviderError::auth("codex", error.to_string()))?;
                    return Ok(provider);
                }
            }

            // Mode 2: Legacy - use cached OpenAI API key
            if let Some(api_key) = &auth_data.openai_api_key {
                return Self::from_api_key(api_key, &model_name);
            }
        }

        // Fallback: try the full refresh + exchange flow (legacy mode)
        let api_key = codex_auth::get_codex_api_key_sync()
            .map_err(|e| ProviderError::auth("codex", format!("{e}")))?;

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new CodexProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    /// PROV-016: Uses RefreshingCodexClient in ApiKey pass-through mode.
    /// Uses the Responses API client so requests go to /responses (rewritten
    /// to the Codex endpoint) with `instructions` from preamble.
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("codex", api_key)?;

        // PROV-016: Create RefreshingCodexClient in API key pass-through mode
        let http_client = RefreshingCodexClient::new_api_key();

        // Build rig Responses API client with RefreshingCodexClient as HTTP backend
        let rig_client = CodexResponsesClient::builder()
            .api_key(api_key)
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config("codex", format!("Failed to build Codex client: {e}"))
            })?;

        // Create Responses API completion model using the client
        let completion_model =
            CodexResponsesModel::new(rig_client.clone(), model).with_strict_attribution();

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            auth_mode: CodexAuthMode::ApiKey,
        })
    }

    /// Create a new CodexProvider using OAuth tokens (Direct Codex API mode)
    ///
    /// This mode routes requests directly to chatgpt.com/backend-api/codex/responses
    /// using Bearer access_token and ChatGPT-Account-Id header.
    ///
    /// PROV-016: Uses RefreshingCodexClient in OAuth mode for dynamic token refresh,
    /// URL rewriting, and header injection. No static base_url or headers needed —
    /// RefreshingCodexClient handles everything dynamically per-request.
    ///
    /// Uses the Responses API client so the request body contains `instructions`
    /// (mapped from preamble) and `input` (mapped from chat history), which is
    /// the format required by the Codex endpoint.
    ///
    /// # Arguments
    /// * `access_token` - OAuth access token (used as Bearer token)
    /// * `refresh_token` - OAuth refresh token (used for token refresh)
    /// * `account_id` - ChatGPT account ID (sent as ChatGPT-Account-Id header)
    /// * `expires_in_secs` - Token expiry in seconds (None defaults to 3600s)
    /// * `issuer_url` - OAuth issuer URL for token refresh (e.g., CODEX_ISSUER)
    /// * `model` - Model name (e.g., "gpt-5.1-codex")
    pub fn from_oauth_tokens(
        access_token: &str,
        refresh_token: &str,
        account_id: &str,
        expires_in_secs: Option<u64>,
        issuer_url: &str,
        model: &str,
    ) -> Result<Self, ProviderError> {
        // PROV-016: Create RefreshingCodexClient in OAuth mode with token refresh support
        let http_client = RefreshingCodexClient::new_oauth(
            access_token.to_string(),
            refresh_token.to_string(),
            account_id.to_string(),
            expires_in_secs,
            issuer_url.to_string(),
        );

        // Build rig Responses API client with RefreshingCodexClient as HTTP backend.
        // - .api_key("dummy") is required by rig but RefreshingCodexClient strips and replaces it
        // - No .base_url() needed — RefreshingCodexClient rewrites URLs dynamically
        // - No .http_headers() needed — RefreshingCodexClient injects headers dynamically
        let rig_client = CodexResponsesClient::builder()
            .api_key("dummy")
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config("codex", format!("Failed to build Codex OAuth client: {e}"))
            })?;

        let completion_model = CodexResponsesModel::new(rig_client.clone(), model)
            .with_strict_attribution()
            .with_streaming_completions();

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            auth_mode: CodexAuthMode::OAuthDirect {
                account_id: account_id.to_string(),
            },
        })
    }

    /// Get the configured rig openai::Client (Responses API)
    pub fn client(&self) -> &CodexResponsesClient {
        &self.rig_client
    }

    /// Get the authentication mode (ApiKey or OAuthDirect)
    pub fn auth_mode(&self) -> &CodexAuthMode {
        &self.auth_mode
    }
}
