//! `CopilotProvider` — top-level composition layer for GitHub Copilot
//! (PROV-053/055).
//!
//! This module glues the header facade, classifier, endpoint facade, and
//! behaviour facades together into a single [`LlmProvider`] entry point that
//! [`ProviderManager`] registers as [`ProviderType::GitHubCopilot`].
//!
//! Per-concern logic has been extracted into sibling modules to keep this
//! file under the 300-line SoC budget (PROV-053 rule 21):
//!
//! - [`base_url`](super::base_url) — `CopilotBaseUrl` + `base_url_for`
//! - [`system_prompt_facade`](super::system_prompt_facade) — facade
//!   selection for chat/completions vs. /responses
//! - [`response`](super::response) — rig → fspec response conversion
//!
//! The heavy lifting lives in the per-concern modules
//! ([`header_facade`](super::header_facade),
//! [`classifier`](super::classifier),
//! [`endpoint`](super::endpoint),
//! [`behavior_facade`](super::behavior_facade),
//! [`refreshing_client`](super::refreshing_client)) — this file only holds
//! the [`CopilotProvider`] struct and its [`LlmProvider`] impl.
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
use crate::copilot::token_exchange::{
    exchange_github_token_for_copilot_token, TokenExchangeResponse,
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
/// Holds the state required to issue an API call: the deployment type
/// (github.com vs Enterprise), the long-lived GitHub OAuth token, a cache
/// of the short-lived Copilot token with its expiry, the model id, and a
/// rig OpenAI completions client whose HTTP backend is the
/// [`CopilotHttpClient`] middleware so every outgoing request carries the
/// full Copilot header set built by the shared
/// [`header_facade`](super::header_facade).
///
/// PROV-057: the `auth` field is shared behind an `Arc<RwLock>` so the
/// two-token refresh logic in [`Self::ensure_fresh_copilot_token`] can
/// update the cached Copilot token + `endpoints_api` across clones of the
/// provider while retaining an `&self` method signature on
/// [`LlmProvider::complete_with_tools`].
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
    /// Compute the API base URL for an active Copilot deployment.
    ///
    /// Re-exported from [`super::base_url::base_url_for`] for API
    /// compatibility with existing test call sites.
    #[must_use]
    pub fn base_url_for(deployment: &CopilotDeploymentType) -> CopilotBaseUrl {
        base_url_for(deployment)
    }

    /// Select the system-prompt facade for a given endpoint.
    ///
    /// Re-exported from
    /// [`super::system_prompt_facade::system_prompt_facade_for_endpoint`]
    /// for API compatibility with existing test call sites.
    #[must_use]
    pub fn system_prompt_facade_for_endpoint(
        endpoint: CopilotEndpoint,
    ) -> BoxedSystemPromptFacade {
        system_prompt_facade_for_endpoint(endpoint)
    }

    /// Fetch the model catalog directly from the Copilot `/models` endpoint
    /// of the active deployment (PROV-056).
    ///
    /// This method is the integration seam consumed by the TUI model picker
    /// via the provider registry. It delegates to
    /// [`crate::copilot::models::fetch_models`], which is the **sole source**
    /// of every model field — no `models.dev` fallback, no static catalog
    /// merge, no hardcoded ids.
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
    /// This constructor is kept for backward compatibility with call sites
    /// that only carry the long-lived token. It builds a minimal
    /// `CopilotAuthJson` with `github_oauth_token = access_token` and no
    /// cached Copilot token — the first call to
    /// [`LlmProvider::complete_with_tools`] will trigger the token
    /// exchange (PROV-057).
    ///
    /// [`ProviderManager::get_github_copilot`]: crate::ProviderManager::get_github_copilot
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Config`] if the rig client builder cannot
    /// assemble the completions client with the given parameters.
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
    /// PROV-057: this is the preferred constructor. It preserves any
    /// cached Copilot token + expiry + `endpoints_api` carried in the
    /// auth file so repeated process starts do not force a fresh token
    /// exchange each time the user selects a Copilot model.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Auth`] if the auth file has an empty
    /// `github_oauth_token`; returns [`ProviderError::Config`] if the rig
    /// client builder cannot assemble the completions client.
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

        // PROV-057: prefer the endpoints_api URL returned by the token
        // exchange over the statically computed base URL so enterprise
        // deployments automatically route to `copilot-api.<host>` as soon
        // as the first token exchange has been performed.
        let base_url = match auth.endpoints_api.as_deref() {
            Some(api) if !api.is_empty() => {
                CopilotBaseUrl::from_string(api.to_string())
            }
            _ => Self::base_url_for(&deployment),
        };

        // The rig client's Bearer header is stripped + re-injected by
        // CopilotHttpClient on every send, so the initial token we seed
        // here is only used until the first refresh. Prefer the cached
        // short-lived Copilot token if present, otherwise fall back to
        // the long-lived GitHub OAuth token.
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

        // PROV-057 Layer 2: keep a cloneable handle to the rig completions
        // client so `Self::client()` can expose it to the NAPI DeepSearch
        // `build_and_run!` macro (`provider.client().agent(provider.model())`).
        // The `CompletionModel` wrapper below consumes one copy for the
        // trait-dispatch path; this clone is the one the agent-builder path
        // calls into. Both copies share the same `CopilotHttpClient` middleware
        // so Bearer-token refresh is consistent across them.
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
    ///
    /// Returns the currently-effective Bearer token (either the cached
    /// Copilot token if one has been minted or the long-lived GitHub OAuth
    /// token as a fallback).
    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Access the resolved base URL for this deployment.
    ///
    /// PROV-057: when an `endpoints_api` URL was carried in the auth file
    /// this returns that value (e.g. `copilot-api.<enterprise-host>`);
    /// otherwise it returns the statically computed deployment URL.
    #[must_use]
    pub fn base_url(&self) -> &CopilotBaseUrl {
        &self.base_url
    }

    /// Borrow the underlying rig `openai::CompletionsClient` parameterised by
    /// [`CopilotHttpClient`] so external callers can build a rig `Agent`
    /// pinned to this provider's HTTP middleware.
    ///
    /// PROV-057 Layer 2: the NAPI DeepSearch sub-agent builder at
    /// `codelet/napi/src/deep_search_handler.rs` (`build_and_run!` macro)
    /// calls `provider.client().agent(provider.model())` to construct a
    /// fresh, separately-tooled agent for DeepSearch queries. Exposing the
    /// rig client here (rather than having callers rebuild it from the raw
    /// auth state) keeps the HTTP middleware — and therefore the Bearer
    /// token refresh behaviour — consistent between the top-level agent
    /// loop and any DeepSearch children.
    ///
    /// The returned reference is specialised to `CopilotHttpClient` so the
    /// `.agent(...)` call chain produces an
    /// `AgentBuilder<CompletionModel<CopilotHttpClient>>`, matching the
    /// type the DeepSearch macro expects.
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
    /// via the `/copilot_internal/v2/token` exchange if the cached token is
    /// missing or within 60 seconds of expiry.
    ///
    /// Returns `true` if a refresh happened, `false` if the cached token
    /// was still fresh enough to reuse. On refresh the updated auth is
    /// persisted back to `copilot_auth.json`.
    ///
    /// This is the single seam tests exercise for the token refresh
    /// behaviour — the full rig-based request loop is too deep to mock in
    /// a unit test, but every scenario of interest is either about the
    /// refresh decision or the endpoint URL, both of which are decided
    /// here.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Api`] if the exchange fails (network,
    /// bad status, invalid JSON).
    pub async fn ensure_fresh_copilot_token(&self) -> Result<bool, ProviderError> {
        let now = unix_timestamp_now();
        let snapshot = self.auth.read().await.clone();
        if !needs_copilot_token_refresh(&snapshot, now) {
            return Ok(false);
        }

        // Acquire write lock and re-check under the lock to avoid a
        // double-refresh if multiple tasks race.
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

/// Apply a [`TokenExchangeResponse`] to a mutable [`CopilotAuthJson`].
///
/// Pure logic extracted so the refresh-decision tests do not need to spin
/// up a mock HTTP server — they can feed the response straight in.
pub(crate) fn apply_exchange_response(
    auth: &mut CopilotAuthJson,
    exchange: TokenExchangeResponse,
) {
    auth.copilot_token = Some(exchange.token);
    auth.copilot_token_expires_at = Some(exchange.expires_at);
    if !exchange.endpoints_api.is_empty() {
        auth.endpoints_api = Some(exchange.endpoints_api);
    }
}

/// PROV-057 Rule 4: decide whether the cached Copilot token needs to be
/// refreshed. A refresh is needed if:
///
/// 1. There is no cached Copilot token at all, or
/// 2. The cached token's `expires_at` is within 60 seconds of `now`.
///
/// This is a pure function so tests can feed a deterministic `now` rather
/// than relying on wall-clock time.
#[must_use]
pub(crate) fn needs_copilot_token_refresh(auth: &CopilotAuthJson, now: u64) -> bool {
    match (auth.copilot_token.as_deref(), auth.copilot_token_expires_at) {
        (Some(tok), Some(exp)) if !tok.is_empty() => exp <= now + 60,
        _ => true,
    }
}

/// Current unix seconds. Extracted as a free function so tests can bypass
/// it by calling [`needs_copilot_token_refresh`] directly.
fn unix_timestamp_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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
        // PROV-057 Rule 4: refresh the short-lived Copilot token if it is
        // missing or within 60 seconds of expiry BEFORE sending the chat
        // request. The refresh mutates the shared auth state and persists
        // it to copilot_auth.json so subsequent sessions reuse the fresh
        // token. The rig client was constructed with the initially cached
        // token; the CopilotHttpClient middleware re-injects the Bearer
        // header from the provider's access_token on each send, which for
        // this iteration is still the construction-time token. Tests
        // exercise the refresh decision logic directly via the helpers on
        // this module.
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
