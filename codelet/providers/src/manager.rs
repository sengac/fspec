//! Provider Manager for dynamic provider selection
//!
//! Handles credential detection and provider instantiation based on:
//! - Available credentials (environment variables and auth files)
//! - CLI arguments (--provider and --model flags)
//! - Priority order: Claude API > Claude OAuth > Gemini > Codex > OpenAI
//!
//! MODEL-001: Integrates with ModelCache and ModelRegistry for dynamic model selection

use super::credentials::ProviderCredentials;
use super::models::{ModelCache, ModelInfo, ModelRegistry};
use super::{
    claude, codex, copilot, gemini, openai, zai, ClaudeProvider, CodexProvider, GeminiProvider,
    OpenAIProvider, ProviderError, ZAIProvider,
};
use super::copilot::{CopilotDeploymentType, CopilotProvider};
use std::str::FromStr;

/// Provider type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    /// PROV-053: GitHub Copilot via OAuth device flow
    GitHubCopilot,
}

impl FromStr for ProviderType {
    type Err = ProviderError;

    fn from_str(name: &str) -> Result<Self, ProviderError> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" => Ok(ProviderType::Gemini),
            "zai" => Ok(ProviderType::ZAI),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            _ => Err(ProviderError::config(
                "manager",
                format!("Unknown provider: {name}"),
            )),
        }
    }
}

impl ProviderType {
    /// Get provider name as string
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderType::Claude => "claude",
            ProviderType::OpenAI => "openai",
            ProviderType::Codex => "codex",
            ProviderType::Gemini => "gemini",
            ProviderType::ZAI => "zai",
            ProviderType::GitHubCopilot => "github-copilot",
        }
    }

    /// Check if this provider type has credentials available
    ///
    /// DRY: Centralizes credential checking instead of repeating the match pattern
    pub fn has_credentials(self, credentials: &ProviderCredentials) -> bool {
        match self {
            ProviderType::Claude => credentials.has_claude(),
            ProviderType::OpenAI => credentials.has_openai(),
            ProviderType::Codex => credentials.has_codex(),
            ProviderType::Gemini => credentials.has_gemini(),
            ProviderType::ZAI => credentials.has_zai(),
            ProviderType::GitHubCopilot => credentials.has_github_copilot(),
        }
    }
}

/// Provider Manager for dynamic provider selection
///
/// Includes optional ModelRegistry for dynamic model selection
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    /// Optional model registry for dynamic model selection
    model_registry: Option<ModelRegistry>,
    /// Selected model string (provider/model-id format)
    selected_model: Option<String>,
}

impl std::fmt::Debug for ProviderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderManager")
            .field("current_provider", &self.current_provider)
            .field("selected_model", &self.selected_model)
            .field("has_model_registry", &self.model_registry.is_some())
            .finish()
    }
}

impl ProviderManager {
    /// Create new ProviderManager with automatic provider selection
    ///
    /// Priority order: Claude API > Claude OAuth > Gemini > Codex > OpenAI
    pub fn new() -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();

        if !credentials.has_any() {
            return Err(ProviderError::auth(
                "manager",
                "No provider credentials found. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, \
                 GOOGLE_GENERATIVE_AI_API_KEY, or run 'codex auth login' to authenticate.",
            ));
        }

        let current_provider = Self::detect_default_provider(&credentials)?;

        Ok(Self {
            credentials,
            current_provider,
            model_registry: None,
            selected_model: None,
        })
    }

    /// Create ProviderManager with explicit provider selection
    pub fn with_provider(provider_name: &str) -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        if !requested_provider.has_credentials(&credentials) {
            let available = credentials.available_providers();
            return Err(ProviderError::auth(
                provider_name,
                format!(
                    "Provider {} not available. Available providers: {}",
                    provider_name,
                    available.join(", ")
                ),
            ));
        }

        Ok(Self {
            credentials,
            current_provider: requested_provider,
            model_registry: None,
            selected_model: None,
        })
    }

    /// Create ProviderManager with explicit provider and model selection
    ///
    /// This is used for internal operations (like compaction) where we need to
    /// recreate a provider manager with the same settings without async model registry.
    /// The model_id is stored directly without registry validation.
    pub fn with_provider_and_model(
        provider_name: &str,
        model_id: Option<&str>,
    ) -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        if !requested_provider.has_credentials(&credentials) {
            let available = credentials.available_providers();
            return Err(ProviderError::auth(
                provider_name,
                format!(
                    "Provider {} not available. Available providers: {}",
                    provider_name,
                    available.join(", ")
                ),
            ));
        }

        Ok(Self {
            credentials,
            current_provider: requested_provider,
            model_registry: None,
            selected_model: model_id.map(String::from),
        })
    }

    /// MODEL-001: Create ProviderManager with model registry support
    ///
    /// This async constructor initializes the model cache and registry,
    /// enabling dynamic model selection via `--model provider/model-id`.
    pub async fn with_model_support() -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();

        if !credentials.has_any() {
            return Err(ProviderError::auth(
                "manager",
                "No provider credentials found. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, \
                 GOOGLE_GENERATIVE_AI_API_KEY, or run 'codex auth login' to authenticate.",
            ));
        }

        let current_provider = Self::detect_default_provider(&credentials)?;

        // Initialize model cache and registry
        let cache = ModelCache::new()?;
        let registry = ModelRegistry::new(&cache).await?;

        Ok(Self {
            credentials,
            current_provider,
            model_registry: Some(registry),
            selected_model: None,
        })
    }

    /// MODEL-001: Select a model using provider/model-id format
    ///
    /// Parses the model string, validates the provider exists and has credentials,
    /// validates the model exists in the registry, and ensures tool_call capability.
    ///
    /// # Arguments
    /// * `model_string` - Model in "provider/model-id" format (e.g., "anthropic/claude-sonnet-4")
    ///
    /// # Returns
    /// The validated ModelInfo for the selected model
    pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError> {
        // PROV-057 (stale-cache fix): `ProviderManager::new()` snapshots
        // `ProviderCredentials::detect()` once at construction, which means
        // a credential file (e.g. `copilot_auth.json`) written *after* the
        // manager was built — typically during an in-session OAuth login —
        // remains invisible until process restart. Re-detect before the
        // `has_credentials` check so a freshly-completed Copilot login is
        // honoured by the very next `select_model` call in the same
        // session. See PROV-057 investigation §8 and §10 row 7.
        self.credentials = ProviderCredentials::detect();

        let registry = self.model_registry.as_ref().ok_or_else(|| {
            ProviderError::config(
                "manager",
                "Model registry not initialized. Use with_model_support() for model selection.",
            )
        })?;

        // Parse the model string into provider/model
        let (provider_id, model_id) = registry.parse_model_string(model_string)?;

        // Map models.dev provider ID to our ProviderType
        let provider_type = Self::map_provider_id_to_type(&provider_id)?;

        // Validate we have credentials for this provider
        if !provider_type.has_credentials(&self.credentials) {
            return Err(ProviderError::auth(
                &provider_id,
                format!(
                    "Provider '{}' requires credentials. Available providers: {}",
                    provider_id,
                    self.credentials.available_providers().join(", ")
                ),
            ));
        }

        // Validate model exists and has tool_call capability
        let model_info = registry.validate_model_for_use(&provider_id, &model_id)?;

        // Update state
        self.current_provider = provider_type;
        self.selected_model = Some(model_string.to_string());

        Ok(model_info)
    }

    /// PROV-007: Select a model directly without registry validation
    ///
    /// For profile-based models (vLLM, Ollama, etc.) that aren't in the models.dev registry.
    /// This assumes the caller has already validated the model exists on the remote server
    /// AND has set up the required environment variables (OPENAI_API_KEY, OPENAI_BASE_URL).
    ///
    /// NOTE: This skips credentials validation because profile credentials are passed via
    /// environment variables that were set AFTER the session was created.
    ///
    /// # Arguments
    /// * `provider_id` - The provider ID (e.g., "openai" for OpenAI-compatible APIs)
    /// * `model_id` - The model ID as recognized by the remote server
    pub fn set_model_direct(
        &mut self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), ProviderError> {
        // Map provider ID to our ProviderType
        let provider_type = Self::map_provider_id_to_type(provider_id)?;

        // NOTE: We intentionally skip credentials validation here.
        // For profile-based models, TypeScript sets OPENAI_API_KEY and OPENAI_BASE_URL
        // as environment variables AFTER the session was created. The OpenAIProvider
        // will read these from the environment when get_openai() is called.

        // Update state - store the model_id directly (no provider/ prefix needed for local models)
        self.current_provider = provider_type;
        self.selected_model = Some(model_id.to_string());

        Ok(())
    }

    /// Get the selected model ID (the actual API model ID)
    ///
    /// Returns the model ID to use for API calls. If a model registry is available,
    /// looks up the model to get the actual API model ID. Otherwise, returns the
    /// stored model string directly (for cases like compaction where the model ID
    /// is passed directly without registry lookup).
    pub fn selected_model_id(&self) -> Option<String> {
        let model_string = self.selected_model.as_ref()?;

        // If we have a registry, try to look up the model
        if let Some(registry) = self.model_registry.as_ref() {
            if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
                if let Ok(model_info) = registry.get_model(&provider_id, &model_id) {
                    return Some(model_info.id.clone());
                }
            }
        }

        // No registry or lookup failed - return the stored string directly
        // This handles cases where the model ID is stored directly (e.g., with_provider_and_model)
        Some(model_string.clone())
    }

    /// MODEL-001: Get model info for the selected model
    pub fn selected_model_info(&self) -> Option<&ModelInfo> {
        let model_string = self.selected_model.as_ref()?;
        let registry = self.model_registry.as_ref()?;

        if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
            registry.get_model(&provider_id, &model_id).ok()
        } else {
            None
        }
    }

    /// MODEL-001: Get the original model string (provider/model-id format)
    ///
    /// Returns the model string as originally passed to select_model(),
    /// e.g., "anthropic/claude-sonnet-4".
    pub fn selected_model_string(&self) -> Option<&str> {
        self.selected_model.as_deref()
    }

    /// MODEL-001: Get the model registry (for CLI commands like `codelet models`)
    pub fn model_registry(&self) -> Option<&ModelRegistry> {
        self.model_registry.as_ref()
    }

    /// MODEL-001: Map models.dev provider ID to our ProviderType
    fn map_provider_id_to_type(provider_id: &str) -> Result<ProviderType, ProviderError> {
        match provider_id {
            "anthropic" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "google" => Ok(ProviderType::Gemini),
            "zai" | "z-ai" => Ok(ProviderType::ZAI),
            "codex" => Ok(ProviderType::Codex),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            _ => Err(ProviderError::config(
                "manager",
                format!(
                    "Provider '{provider_id}' is not supported. Supported providers: anthropic, openai, google, zai, codex, github-copilot"
                ),
            )),
        }
    }

    /// Detect default provider based on priority
    fn detect_default_provider(
        credentials: &ProviderCredentials,
    ) -> Result<ProviderType, ProviderError> {
        // Priority: Claude > Gemini > ZAI > Codex > GitHubCopilot > OpenAI
        if credentials.has_claude() {
            return Ok(ProviderType::Claude);
        }
        if credentials.has_gemini() {
            return Ok(ProviderType::Gemini);
        }
        if credentials.has_zai() {
            return Ok(ProviderType::ZAI);
        }
        if credentials.has_codex() {
            return Ok(ProviderType::Codex);
        }
        if credentials.has_github_copilot() {
            return Ok(ProviderType::GitHubCopilot);
        }
        if credentials.has_openai() {
            return Ok(ProviderType::OpenAI);
        }

        Err(ProviderError::auth(
            "manager",
            "No provider credentials available",
        ))
    }

    /// Get current provider name
    pub fn current_provider_name(&self) -> &str {
        self.current_provider.as_str()
    }

    /// Get Claude provider (if selected)
    ///
    /// MODEL-001: Now uses selected_model_id() for dynamic model selection.
    /// PROV-026: Checks claude_auth.json for OAuth tokens first (OAuth takes
    /// precedence over API key). Falls back to new_with_model() if no OAuth.
    pub fn get_claude(&self) -> Result<ClaudeProvider, ProviderError> {
        if self.current_provider == ProviderType::Claude {
            let model_id = self.selected_model_id();

            // PROV-026: Check claude_auth.json for OAuth tokens first
            if let Ok(Some(auth)) = crate::claude_auth::read_claude_auth_sync() {
                if !auth.access_token.is_empty() && !auth.refresh_token.is_empty() {
                    return ClaudeProvider::from_oauth_tokens(
                        &auth.access_token,
                        &auth.refresh_token,
                        Some(0), // Force immediate refresh — tokens from disk are of unknown age
                        crate::claude_oauth::CLAUDE_TOKEN_ENDPOINT_BASE,
                        model_id.as_deref().ok_or_else(|| {
                            ProviderError::config("claude", "Model is required. Please select a model before creating a session.")
                        })?,
                    );
                }
            }

            // Fall back to env var-based authentication
            ClaudeProvider::new_with_model(model_id.as_deref())
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not Claude",
            ))
        }
    }

    /// Get OpenAI provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    /// PROV-051: Accepts session_id for cache optimization (session affinity headers).
    pub fn get_openai(&self, session_id: uuid::Uuid) -> Result<OpenAIProvider, ProviderError> {
        if self.current_provider != ProviderType::OpenAI {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not OpenAI",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "openai",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::auth("openai", "OPENAI_API_KEY not set"))?;
        OpenAIProvider::from_api_key_with_session(&api_key, &model_id, session_id)
    }

    /// Get Codex provider (if selected)
    ///
    /// PROV-018: Sets CODEX_MODEL env var from the selected model so that
    /// CodexProvider::new() can read it. This bridges the TUI model selection
    /// (which stores the model in ProviderManager state) with the CodexProvider
    /// (which reads from the CODEX_MODEL env var).
    pub fn get_codex(&self) -> Result<CodexProvider, ProviderError> {
        if self.current_provider != ProviderType::Codex {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Codex",
            ));
        }

        // Set CODEX_MODEL from selected model for CodexProvider::new() to read
        if let Some(model_id) = self.selected_model_id() {
            std::env::set_var("CODEX_MODEL", &model_id);
        }

        CodexProvider::new()
    }

    /// Get Gemini provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    pub fn get_gemini(&self) -> Result<GeminiProvider, ProviderError> {
        if self.current_provider != ProviderType::Gemini {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Gemini",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "gemini",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let api_key = std::env::var("GOOGLE_GENERATIVE_AI_API_KEY")
            .map_err(|_| ProviderError::auth("gemini", "GOOGLE_GENERATIVE_AI_API_KEY not set"))?;
        GeminiProvider::from_api_key(&api_key, &model_id)
    }

    /// Get GitHub Copilot provider (if selected) — PROV-053 rule 9.
    ///
    /// Reads the persisted credential from `~/.fspec/credentials/copilot_auth.json`
    /// (written by `codelet auth login github-copilot`), determines the
    /// deployment type from the `enterprise_url` field, and constructs a
    /// [`CopilotProvider`] wired through the [`copilot::CopilotHttpClient`]
    /// middleware. The middleware injects the full Copilot header set
    /// (`x-initiator`, `User-Agent`, `Authorization`, `Openai-Intent`, and
    /// conditional `Copilot-Vision-Request`) on every outgoing request via
    /// the shared [`copilot::CopilotHeaderFacade`].
    ///
    /// Requires a model to be selected via `select_model()`.
    ///
    /// # Errors
    ///
    /// - [`ProviderError::Config`] if the current provider is not
    ///   `GitHubCopilot` or no model is selected
    /// - [`ProviderError::Auth`] if the credential file cannot be read or is
    ///   missing the access token
    pub fn get_github_copilot(&self) -> Result<CopilotProvider, ProviderError> {
        if self.current_provider != ProviderType::GitHubCopilot {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not GitHub Copilot",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "github-copilot",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let auth = copilot::read_copilot_auth_sync()
            .map_err(|e| {
                ProviderError::auth(
                    "github-copilot",
                    format!("Failed to read copilot_auth.json: {e}"),
                )
            })?
            .ok_or_else(|| {
                ProviderError::auth(
                    "github-copilot",
                    "No GitHub Copilot credential found. Run `codelet auth login github-copilot`.",
                )
            })?;

        if auth.github_oauth_token.is_empty() {
            return Err(ProviderError::auth(
                "github-copilot",
                "GitHub Copilot credential contains an empty access token. Re-run `codelet auth login github-copilot`.",
            ));
        }

        let deployment = match auth.enterprise_url.clone() {
            Some(host) => CopilotDeploymentType::Enterprise { host },
            None => CopilotDeploymentType::GitHubCom,
        };

        CopilotProvider::from_auth(deployment, auth, &model_id)
    }

    /// Check if any provider is available
    pub fn has_any_provider(&self) -> bool {
        self.credentials.has_any()
    }

    /// List all available providers for display
    pub fn list_available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.credentials.has_claude() {
            providers.push("Claude (/claude)".to_string());
        }
        if self.credentials.has_openai() {
            providers.push("OpenAI (/openai)".to_string());
        }
        if self.credentials.has_gemini() {
            providers.push("Gemini (/gemini)".to_string());
        }
        if self.credentials.has_zai() {
            providers.push("Z.AI (/zai)".to_string());
        }
        if self.credentials.has_codex() {
            providers.push("Codex (/codex)".to_string());
        }
        if self.credentials.has_github_copilot() {
            providers.push("GitHub Copilot (/github-copilot)".to_string());
        }
        providers
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<(), ProviderError> {
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        if !requested_provider.has_credentials(&self.credentials) {
            return Err(ProviderError::auth(
                provider_name,
                format!("Provider {provider_name} not available. No credentials found."),
            ));
        }

        self.current_provider = requested_provider;
        Ok(())
    }

    /// Get prompt prefix for REPL (e.g., "[claude] > ")
    pub fn get_prompt_prefix(&self) -> String {
        format!("[{}] > ", self.current_provider.as_str())
    }

    /// Get context window size for the current provider
    ///
    /// Returns the context window in tokens for the currently selected provider.
    /// CONFIG-007: 1M context opt-in not yet implemented - uses 200k for all Claude models.
    pub fn context_window(&self) -> usize {
        match self.current_provider {
            ProviderType::Claude => claude::CONTEXT_WINDOW,
            ProviderType::OpenAI => openai::CONTEXT_WINDOW,
            ProviderType::Gemini => gemini::CONTEXT_WINDOW,
            ProviderType::Codex => codex::CONTEXT_WINDOW,
            ProviderType::ZAI => zai::CONTEXT_WINDOW,
            ProviderType::GitHubCopilot => copilot::CONTEXT_WINDOW,
        }
    }

    /// Get max output tokens for the current provider (CTX-002)
    ///
    /// Returns the maximum output tokens for the currently selected provider.
    /// Used for calculating usable context in the optimized compaction algorithm.
    /// PROV-039: Reads runtime env vars for OpenAI instead of compile-time constants.
    pub fn max_output_tokens(&self) -> usize {
        match self.current_provider {
            ProviderType::Claude => claude::MAX_OUTPUT_TOKENS,
            ProviderType::OpenAI => {
                // PROV-039: Read OPENAI_MAX_OUTPUT_TOKENS env var at runtime
                std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(openai::MAX_OUTPUT_TOKENS)
            }
            ProviderType::Gemini => gemini::MAX_OUTPUT_TOKENS,
            ProviderType::Codex => codex::MAX_OUTPUT_TOKENS,
            ProviderType::ZAI => zai::MAX_OUTPUT_TOKENS,
            ProviderType::GitHubCopilot => copilot::MAX_OUTPUT_TOKENS,
        }
    }

    /// Test-only constructor that creates a ProviderManager without requiring credentials.
    /// Exposed for workspace-level integration tests that need to call methods like
    /// `max_output_tokens()` without a real provider backend.
    #[doc(hidden)]
    pub fn for_testing(provider: ProviderType) -> Self {
        Self {
            credentials: ProviderCredentials {
                claude_available: false,
                openai_available: false,
                codex_available: false,
                gemini_available: false,
                zai_available: false,
                github_copilot_available: false,
            },
            current_provider: provider,
            model_registry: None,
            selected_model: None,
        }
    }

    /// Get Z.AI provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    /// Checks ZAI_PLAN_API_KEY first (for coding plan endpoint), then ZAI_API_KEY.
    pub fn get_zai(&self) -> Result<ZAIProvider, ProviderError> {
        if self.current_provider != ProviderType::ZAI {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Z.AI",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "zai",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        // Check for plan API key first, then normal API key
        let (api_key, is_plan) = if let Ok(key) = std::env::var("ZAI_PLAN_API_KEY") {
            if !key.is_empty() {
                (key, true)
            } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
                (key, false)
            } else {
                return Err(ProviderError::auth(
                    "zai",
                    "ZAI_API_KEY or ZAI_PLAN_API_KEY not set",
                ));
            }
        } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
            (key, false)
        } else {
            return Err(ProviderError::auth(
                "zai",
                "ZAI_API_KEY or ZAI_PLAN_API_KEY not set",
            ));
        };
        ZAIProvider::from_api_key_with_endpoint(&api_key, &model_id, is_plan)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::copilot::auth::{write_copilot_auth, CopilotAuthJson};
    use crate::models::{LimitInfo, ModelInfo, ModelRegistry, ModelsDevResponse, ProviderInfo};
    use std::collections::HashMap;

    // =========================================================================
    // PROV-039: ProviderManager::max_output_tokens() must read runtime env var
    // Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
    // =========================================================================

    /// Helper to create a ProviderManager for testing without real credentials
    fn test_manager(provider: ProviderType) -> ProviderManager {
        ProviderManager {
            credentials: ProviderCredentials {
                claude_available: false,
                openai_available: false,
                codex_available: false,
                gemini_available: false,
                zai_available: false,
                github_copilot_available: false,
            },
            current_provider: provider,
            model_registry: None,
            selected_model: None,
        }
    }

    /// Per-test guard that redirects `FSPEC_HOME` at a fresh temp directory so
    /// writes never escape into the user's real `~/.fspec`. Restores the
    /// previous value on drop.
    struct FspecHomeGuard {
        _tempdir: tempfile::TempDir,
        original: Option<String>,
    }

    impl FspecHomeGuard {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("tempdir");
            let original = std::env::var("FSPEC_HOME").ok();
            std::env::set_var("FSPEC_HOME", tempdir.path());
            Self {
                _tempdir: tempdir,
                original,
            }
        }
    }

    impl Drop for FspecHomeGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => std::env::set_var("FSPEC_HOME", val),
                None => std::env::remove_var("FSPEC_HOME"),
            }
        }
    }

    /// Build a minimal in-memory `ModelRegistry` containing a single
    /// `github-copilot/gpt-4o` entry with `tool_call = true` so
    /// `select_model` has something to validate against without hitting the
    /// live models.dev cache.
    fn build_github_copilot_registry() -> ModelRegistry {
        let mut models = HashMap::new();
        models.insert(
            "gpt-4o".to_string(),
            ModelInfo {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                family: Some("gpt-4o".to_string()),
                release_date: None,
                attachment: false,
                reasoning: false,
                tool_call: true,
                temperature: true,
                interleaved: None,
                modalities: None,
                cost: None,
                limit: LimitInfo {
                    context: 128_000,
                    output: 16_384,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );
        let mut providers = HashMap::new();
        providers.insert(
            "github-copilot".to_string(),
            ProviderInfo {
                id: "github-copilot".to_string(),
                name: "GitHub Copilot".to_string(),
                env: vec![],
                npm: None,
                api: Some("https://api.githubcopilot.com".to_string()),
                doc: None,
                models,
            },
        );
        ModelRegistry::from_response(ModelsDevResponse { providers })
    }

    /// Build a test `ProviderManager` with the github-copilot registry
    /// entry pre-loaded and **no** credentials detected — simulating the
    /// state where the user hasn't logged in yet.
    fn test_manager_with_copilot_registry() -> ProviderManager {
        ProviderManager {
            credentials: ProviderCredentials {
                claude_available: false,
                openai_available: false,
                codex_available: false,
                gemini_available: false,
                zai_available: false,
                github_copilot_available: false,
            },
            current_provider: ProviderType::Claude,
            model_registry: Some(build_github_copilot_registry()),
            selected_model: None,
        }
    }

    // =========================================================================
    // PROV-057: ProviderManager::select_model must re-detect credentials
    // Feature: spec/features/github-copilot-end-to-end-integration.feature
    //
    // Bug: ProviderManager snapshots ProviderCredentials::detect() once at
    // construction. After OAuth login writes copilot_auth.json the cache
    // still says "no credentials" until process restart.
    // =========================================================================

    /// Scenario: Selecting a github-copilot model right after login succeeds
    /// without restart.
    #[tokio::test]
    #[serial_test::serial]
    async fn select_model_re_detects_copilot_credentials_without_restart() {
        // @step Given ProviderManager was constructed before copilot_auth.json existed
        let _guard = FspecHomeGuard::new();
        let mut manager = test_manager_with_copilot_registry();

        // Sanity: the cached credentials snapshot taken at construction does
        // NOT see any Copilot credential file.
        assert!(
            !manager.credentials.has_github_copilot(),
            "precondition: no Copilot credential should be detected before login"
        );

        // @step And copilot_auth.json has just been written by the OAuth login flow
        let auth = CopilotAuthJson::from_github_oauth_token(
            "gho_prov_057_fresh_login".to_string(),
            None,
        );
        write_copilot_auth(&auth)
            .await
            .expect("write_copilot_auth should succeed in temp HOME");

        // @step When the user calls select_model("github-copilot/gpt-4o") in the same session
        let result = manager.select_model("github-copilot/gpt-4o");

        // @step Then ProviderCredentials::detect() is re-invoked before the has_credentials check
        // @step And the selection succeeds without a "requires credentials" error
        assert!(
            result.is_ok(),
            "select_model must succeed after a fresh copilot_auth.json is written, \
             but it returned: {:?}",
            result.err()
        );

        // And the manager must now treat github-copilot as the current provider.
        assert_eq!(manager.current_provider_name(), "github-copilot");
        assert_eq!(
            manager.selected_model_string(),
            Some("github-copilot/gpt-4o")
        );
    }

    /// Regression guard: without a credential file on disk, `select_model`
    /// must still surface a `requires credentials` auth error for
    /// github-copilot — otherwise the stale-cache fix would silently mask
    /// real missing-credential cases.
    #[tokio::test]
    #[serial_test::serial]
    async fn select_model_still_rejects_github_copilot_when_no_credential_exists() {
        let _guard = FspecHomeGuard::new();
        let mut manager = test_manager_with_copilot_registry();

        let result = manager.select_model("github-copilot/gpt-4o");

        assert!(
            result.is_err(),
            "select_model must still fail when copilot_auth.json does not exist"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("requires credentials") || err.contains("Authentication"),
            "error message should mention credentials, got: {err}"
        );
    }

    /// Scenario: OpenAI max_output_tokens reads runtime environment variable
    ///
    /// This test verifies the BUG: ProviderManager::max_output_tokens() returns
    /// the compile-time constant for OpenAI, ignoring the runtime env var.
    /// The fix should make it read the env var at runtime.
    #[test]
    #[serial_test::serial]
    fn test_provider_manager_openai_max_output_tokens_reads_env_var() {
        // @step Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to "16384"
        std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "16384");

        // @step When ProviderManager::max_output_tokens() is called for the OpenAI provider
        let manager = test_manager(ProviderType::OpenAI);
        let result = manager.max_output_tokens();

        // @step Then the returned value is 16384
        assert_eq!(
            result, 16384,
            "max_output_tokens should read OPENAI_MAX_OUTPUT_TOKENS env var"
        );

        // @step And the returned value is not the compile-time constant 4096
        assert_ne!(result, 4096);

        // Clean up
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    }

    /// Verify default when env var is not set
    #[test]
    #[serial_test::serial]
    fn test_provider_manager_openai_max_output_tokens_default() {
        // Ensure env var is not set
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        let manager = test_manager(ProviderType::OpenAI);
        let result = manager.max_output_tokens();

        // Should return the default when no env var is set
        assert_eq!(result, 4096);
    }
}
