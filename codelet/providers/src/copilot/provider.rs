//! `CopilotProvider` — top-level composition layer for GitHub Copilot
//! (PROV-053/055).
//!
//! Per-concern logic is extracted into sibling modules:
//!
//! - [`base_url`](super::base_url) — `CopilotBaseUrl` + `base_url_for`
//! - [`system_prompt_facade`](super::system_prompt_facade) — facade selection
//! - [`response`](super::response) — rig → fspec response conversion
//! - [`token_refresh`](super::token_refresh) — refresh decision + pure helpers
//!
//! [`ProviderType::GitHubCopilot`]: crate::ProviderType::GitHubCopilot
//! [`ProviderManager`]: crate::ProviderManager
//! [`LlmProvider`]: crate::LlmProvider

use crate::copilot::auth::{write_copilot_auth, CopilotAuthJson};
use crate::copilot::base_url::{base_url_for, CopilotBaseUrl};
use crate::copilot::endpoint::CopilotEndpoint;
use crate::copilot::models::fetch_models;
use crate::copilot::oauth_types::CopilotDeploymentType;
use crate::copilot::refreshing_client::CopilotHttpClient;
use crate::copilot::response::rig_response_to_completion;
use crate::copilot::system_prompt_facade::system_prompt_facade_for_endpoint;
use crate::copilot::token_exchange::exchange_github_token_for_copilot_token;
use crate::copilot::token_refresh::{
    apply_exchange_response, needs_copilot_token_refresh, unix_timestamp_now,
};
use crate::error::ProviderError;
use crate::models::ModelInfo;
use crate::{
    convert_tools_to_rig, extract_prompt_data, extract_text_from_content, CompletionResponse,
    LlmProvider,
};
use async_trait::async_trait;
use codelet_common::Message;
use codelet_tools::facade::BoxedSystemPromptFacade;
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Top-level Copilot provider façade.
///
/// Holds the state required to issue an API call: the deployment type,
/// the long-lived GitHub OAuth token, a cached short-lived Copilot token,
/// the model id, and a rig OpenAI completions client whose HTTP backend
/// uses [`CopilotHttpClient`] middleware for header injection.
///
/// PROV-057: the `auth` field is shared behind `Arc<RwLock>` so token
/// refresh can update the cache across clones while retaining `&self`.
#[derive(Clone)]
pub struct CopilotProvider {
    deployment: CopilotDeploymentType,
    access_token: std::sync::Arc<str>,
    model_name: String,
    rig_client: openai::CompletionsClient<CopilotHttpClient>,
    completion_model: openai::completion::CompletionModel<CopilotHttpClient>,
    base_url: CopilotBaseUrl,
    auth: Arc<RwLock<CopilotAuthJson>>,
}

impl std::fmt::Debug for CopilotProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopilotProvider")
            .field("deployment", &self.deployment)
            .field("model", &self.model_name)
            .field("base_url", &self.base_url.as_str())
            .finish()
    }
}

impl CopilotProvider {
    /// Re-exported from [`super::base_url::base_url_for`].
    #[must_use]
    pub fn base_url_for(deployment: &CopilotDeploymentType) -> CopilotBaseUrl {
        base_url_for(deployment)
    }

    /// Re-exported from [`super::system_prompt_facade::system_prompt_facade_for_endpoint`].
    #[must_use]
    pub fn system_prompt_facade_for_endpoint(
        endpoint: CopilotEndpoint,
    ) -> BoxedSystemPromptFacade {
        system_prompt_facade_for_endpoint(endpoint)
    }

    /// Fetch the model catalog from the Copilot `/models` endpoint (PROV-056).
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Api`] for transport, status, or JSON parse failures.
    pub async fn list_models(
        deployment: &CopilotDeploymentType,
        token: &str,
    ) -> Result<Vec<ModelInfo>, ProviderError> {
        let base_url = Self::base_url_for(deployment);
        fetch_models(base_url.as_str(), token).await
    }

    /// Construct a new `CopilotProvider` from a raw GitHub OAuth token.
    ///
    /// Builds a minimal `CopilotAuthJson` and delegates to [`Self::from_auth`].
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Config`] if the rig client builder fails.
    pub fn new(
        deployment: CopilotDeploymentType,
        access_token: String,
        model: &str,
    ) -> Result<Self, ProviderError> {
        let auth =
            CopilotAuthJson::from_github_oauth_token(access_token, match &deployment {
                CopilotDeploymentType::GitHubCom => None,
                CopilotDeploymentType::Enterprise { host } => Some(host.clone()),
            });
        Self::from_auth(deployment, auth, model)
    }

    /// Construct a new `CopilotProvider` from a full `CopilotAuthJson`.
    ///
    /// PROV-057: preserves cached Copilot token + expiry + `endpoints_api`
    /// so repeated process starts avoid a fresh token exchange.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Auth`] if `github_oauth_token` is empty;
    /// returns [`ProviderError::Config`] if the rig client builder fails.
    pub fn from_auth(
        deployment: CopilotDeploymentType,
        auth: CopilotAuthJson,
        model: &str,
    ) -> Result<Self, ProviderError> {
        if auth.github_oauth_token.is_empty() {
            return Err(ProviderError::auth(
                "github-copilot",
                "Copilot github_oauth_token is empty — run `codelet auth login github-copilot` first",
            ));
        }
        if model.is_empty() {
            return Err(ProviderError::config(
                "github-copilot",
                "Model is required. Please select a model before starting a session.",
            ));
        }

        // PROV-057: prefer the endpoints_api URL from the token exchange.
        let base_url = match auth.endpoints_api.as_deref() {
            Some(api) if !api.is_empty() => {
                CopilotBaseUrl::from_string(api.to_string())
            }
            _ => Self::base_url_for(&deployment),
        };

        // Prefer the cached short-lived Copilot token if present.
        let initial_token = auth
            .copilot_token
            .clone()
            .unwrap_or_else(|| auth.github_oauth_token.clone());

        let http_client = CopilotHttpClient::new(initial_token.clone());

        let rig_client = openai::CompletionsClient::<CopilotHttpClient>::builder()
            .api_key(&initial_token)
            .base_url(base_url.as_str())
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config(
                    "github-copilot",
                    format!("Failed to build Copilot completions client: {e}"),
                )
            })?;

        // PROV-057 Layer 2: keep a cloneable handle for the NAPI DeepSearch
        // `build_and_run!` macro path.
        let completion_model =
            openai::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            deployment,
            access_token: std::sync::Arc::from(initial_token),
            model_name: model.to_string(),
            rig_client,
            completion_model,
            base_url,
            auth: Arc::new(RwLock::new(auth)),
        })
    }

    /// Access the deployment this provider was constructed for (tests + diagnostics).
    #[must_use]
    pub fn deployment(&self) -> &CopilotDeploymentType {
        &self.deployment
    }

    /// Access the stored access token (tests + diagnostics).
    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Resolved base URL for this deployment.
    #[must_use]
    pub fn base_url(&self) -> &CopilotBaseUrl {
        &self.base_url
    }

    /// Borrow the underlying rig completions client for DeepSearch sub-agents.
    ///
    /// PROV-057 Layer 2: the NAPI DeepSearch builder calls
    /// `provider.client().agent(provider.model())` to construct agents that
    /// share the same HTTP middleware and Bearer-token refresh behaviour.
    #[must_use]
    pub fn client(&self) -> &openai::CompletionsClient<CopilotHttpClient> {
        &self.rig_client
    }

    /// Snapshot of the current auth state (tests + diagnostics).
    #[must_use]
    pub async fn auth_snapshot(&self) -> CopilotAuthJson {
        self.auth.read().await.clone()
    }

    /// PROV-057: Ensure the cached Copilot token is still valid, refreshing
    /// via the token exchange if needed.
    ///
    /// Returns `true` if a refresh happened.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Api`] if the exchange fails.
    pub async fn ensure_fresh_copilot_token(&self) -> Result<bool, ProviderError> {
        let now = unix_timestamp_now();
        let snapshot = self.auth.read().await.clone();
        if !needs_copilot_token_refresh(&snapshot, now) {
            return Ok(false);
        }

            // Acquire write lock and re-check to avoid double-refresh under race.
        let mut state = self.auth.write().await;
        if !needs_copilot_token_refresh(&state, now) {
            return Ok(false);
        }

        let enterprise_host = state.enterprise_url.clone();
        let gh_token = state.github_oauth_token.clone();
        let exchange =
            exchange_github_token_for_copilot_token(&gh_token, enterprise_host.as_deref())
                .await?;

        apply_exchange_response(&mut state, exchange);
        let persist = state.clone();
        drop(state);

        write_copilot_auth(&persist).await.map_err(|e| {
            ProviderError::api(
                "github-copilot",
                format!("Failed to persist refreshed copilot_auth.json: {e}"),
            )
        })?;

        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// LIMITS-003: ModelLimitsResolver for Copilot
// ---------------------------------------------------------------------------

impl crate::model_limits::ModelLimitsResolver for CopilotProvider {
    /// Copilot trusts real limits from the live `/models` endpoint via registry.
    fn max_context_window(&self) -> Option<usize> {
        None
    }

    /// Copilot trusts real limits from the live `/models` endpoint via registry.
    fn max_output_tokens_limit(&self) -> Option<usize> {
        None
    }

    /// Default context window when no registry data is available.
    fn default_context_window(&self) -> usize {
        crate::copilot::CONTEXT_WINDOW
    }

    /// Default max output tokens when no registry data is available.
    fn default_max_output_tokens(&self) -> usize {
        crate::copilot::MAX_OUTPUT_TOKENS
    }
}

#[async_trait]
impl LlmProvider for CopilotProvider {
    fn name(&self) -> &str {
        "github-copilot"
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    fn context_window(&self) -> usize {
        // Real per-model window lives in the /models response (PROV-056).
        // This fallback is the same value used by the dispatch layer for
        // budget calculations when a model hasn't been selected yet.
        crate::copilot::CONTEXT_WINDOW
    }

    fn max_output_tokens(&self) -> usize {
        crate::copilot::MAX_OUTPUT_TOKENS
    }

    fn supports_caching(&self) -> bool {
        // Copilot does not expose prompt caching primitives the way Claude does.
        false
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError> {
        let response = self.complete_with_tools(messages, &[]).await?;
        Ok(extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // PROV-057: refresh the short-lived Copilot token if needed.
        self.ensure_fresh_copilot_token().await?;

        let (preamble, prompt) = extract_prompt_data(messages);
        let rig_tools = convert_tools_to_rig(tools);

        let mut builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .max_tokens(crate::copilot::MAX_OUTPUT_TOKENS as u64)
            .tools(rig_tools);

        if let Some(preamble_text) = preamble {
            builder = builder.preamble(preamble_text);
        }

        let response = builder.send().await.map_err(|e| {
            ProviderError::api("github-copilot", format!("Rig completion failed: {e}"))
        })?;

        rig_response_to_completion(response)
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
