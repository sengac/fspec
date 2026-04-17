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
use crate::model_limits::{resolve_context_window, resolve_max_output_tokens, ModelLimitsResolver};
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
// ---------------------------------------------------------------------------
// LIMITS-004: Lightweight resolver stub for ProviderManager
// ---------------------------------------------------------------------------
/// A lightweight `ModelLimitsResolver` that mirrors provider constants without
/// requiring a full provider instance (which needs credentials / API keys).
///
/// Used by `ProviderManager::provider_limits_resolver()` to build a resolver
/// for the current provider at resolution time.
struct ConstantResolver {
    max_ctx: Option<usize>,
    default_ctx: usize,
    max_out: Option<usize>,
    default_out: usize,
}

impl ModelLimitsResolver for ConstantResolver {
    fn max_context_window(&self) -> Option<usize> {
        self.max_ctx
    }
    fn max_output_tokens_limit(&self) -> Option<usize> {
        self.max_out
    }
    fn default_context_window(&self) -> usize {
        self.default_ctx
    }
    fn default_max_output_tokens(&self) -> usize {
        self.default_out
    }
}

pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    /// Optional model registry for dynamic model selection
    model_registry: Option<ModelRegistry>,
    /// Selected model string (provider/model-id format)
    selected_model: Option<String>,
    /// LIMITS-004: Raw context window from models.dev registry (before clamping).
    /// Stored by `select_model()`. Clamped at resolution time by the provider's
    /// `ModelLimitsResolver`.
    pub(crate) registry_context_window: Option<usize>,
    /// LIMITS-004: Raw max output tokens from models.dev registry (before clamping).
    pub(crate) registry_max_output_tokens: Option<usize>,
    /// LIMITS-004: User-configured context window override (from NAPI).
    /// Takes priority over registry values but is still clamped by provider max.
    user_context_window: Option<usize>,
    /// LIMITS-004: User-configured max output tokens override (from NAPI).
    user_max_output_tokens: Option<usize>,
    /// MODEL-004: Facade override for agent loop dispatch.
    ///
    /// When set, the agent loop dispatches to this provider's get_*() method
    /// instead of the `current_provider`. This enables custom models defined
    /// in profiles to route API calls through a different provider backend
    /// (e.g., a vLLM model using the Codex facade for tool schema).
    facade_override: Option<String>,
    /// CTX-007: Per-model compaction threshold override.
    /// Stored as (type, value) where type is "tokens" or "percentage".
    /// When None, threshold falls through to builtin defaults or legacy formula.
    /// This is a simple data field — the resolution logic lives in codelet-cli's
    /// compaction_threshold module to avoid circular crate dependencies.
    compaction_threshold_override: Option<(String, u64)>,
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
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
        })
    }

    /// Create ProviderManager with explicit provider and model selection
    ///
    /// This is used for internal operations (like compaction) where we need to
    /// recreate a provider manager with the same settings without async model registry.
    /// The model_id is stored directly without registry validation.
    ///
    /// LIMITS-004: `with_provider_and_model` receives already-resolved values
    /// from a parent ProviderManager (via `context_window()` / `max_output_tokens()`).
    /// These are stored as registry values (already clamped by the parent).
    pub fn with_provider_and_model(
        provider_name: &str,
        model_id: Option<&str>,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
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
            registry_context_window: context_window,
            registry_max_output_tokens: max_output_tokens,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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

        // LIMITS-004: Store raw registry values. Clamping happens at resolution time
        // via the provider's ModelLimitsResolver in context_window() / max_output_tokens().
        self.registry_context_window = Some(model_info.limit.context as usize);
        self.registry_max_output_tokens = Some(model_info.limit.output as usize);
        // Clear any previous user overrides — new model selection resets them.
        self.user_context_window = None;
        self.user_max_output_tokens = None;

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
    /// MODEL-005: Accepts optional context_window and max_output_tokens since profile
    /// models have no registry to look up metadata — these come from the NAPI call.
    ///
    /// MODEL-004: Accepts optional facade_override for custom models that need
    /// agent loop dispatch through a different provider (e.g., a model defined in
    /// a vLLM profile that should use the Codex tool schema).
    ///
    /// # Arguments
    /// * `provider_id` - The provider ID (e.g., "openai" for OpenAI-compatible APIs)
    /// * `model_id` - The model ID as recognized by the remote server
    /// * `context_window` - Optional per-model context window size
    /// * `max_output_tokens` - Optional per-model max output tokens
    /// * `facade_override` - Optional provider name to dispatch to instead of provider_id
    pub fn set_model_direct(
        &mut self,
        provider_id: &str,
        model_id: &str,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
        facade_override: Option<String>,
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
        // LIMITS-004: Profile models pass values as user overrides (from NAPI).
        // They have no registry data. The resolver clamps them at resolution time.
        self.user_context_window = context_window;
        self.user_max_output_tokens = max_output_tokens;
        self.registry_context_window = None;
        self.registry_max_output_tokens = None;
        // MODEL-004: Store facade override for agent loop dispatch
        self.facade_override = facade_override;

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

    /// LIMITS-004: Get the `ModelLimitsResolver` for the current provider.
    ///
    /// Returns a boxed resolver that declares the provider's hard limits and
    /// defaults. This avoids constructing a full provider (which requires
    /// credentials / API keys) — instead we build a lightweight resolver stub
    /// that mirrors the constants from each provider's `ModelLimitsResolver`
    /// impl.
    fn provider_limits_resolver(&self) -> Box<dyn ModelLimitsResolver> {
        match self.current_provider {
            ProviderType::Claude => Box::new(ConstantResolver {
                max_ctx: Some(claude::CONTEXT_WINDOW),
                default_ctx: claude::CONTEXT_WINDOW,
                max_out: Some(claude::MAX_OUTPUT_TOKENS),
                default_out: claude::MAX_OUTPUT_TOKENS,
            }),
            ProviderType::OpenAI => {
                // OpenAI trusts registry; defaults come from env vars or constants.
                let ctx = std::env::var("OPENAI_CONTEXT_WINDOW")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(openai::CONTEXT_WINDOW);
                let out = std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(openai::MAX_OUTPUT_TOKENS);
                Box::new(ConstantResolver {
                    max_ctx: None,
                    default_ctx: ctx,
                    max_out: None,
                    default_out: out,
                })
            }
            ProviderType::Gemini => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: gemini::CONTEXT_WINDOW,
                max_out: None,
                default_out: gemini::MAX_OUTPUT_TOKENS,
            }),
            ProviderType::Codex => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: codex::CONTEXT_WINDOW,
                max_out: None,
                default_out: codex::MAX_OUTPUT_TOKENS,
            }),
            ProviderType::ZAI => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: zai::CONTEXT_WINDOW,
                max_out: None,
                default_out: zai::MAX_OUTPUT_TOKENS,
            }),
            ProviderType::GitHubCopilot => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: copilot::CONTEXT_WINDOW,
                max_out: None,
                default_out: copilot::MAX_OUTPUT_TOKENS,
            }),
        }
    }

    /// Get context window size for the current provider.
    ///
    /// LIMITS-004: Resolves through `ModelLimitsResolver` so that provider
    /// hard limits clamp registry and user-override values.
    ///
    /// Priority chain (highest → lowest):
    /// 1. User override — clamped by provider max
    /// 2. Registry value — clamped by provider max
    /// 3. Provider default
    pub fn context_window(&self) -> usize {
        let resolver = self.provider_limits_resolver();
        resolve_context_window(
            self.registry_context_window,
            self.user_context_window,
            resolver.as_ref(),
        )
    }

    /// Get max output tokens for the current provider.
    ///
    /// LIMITS-004: Resolves through `ModelLimitsResolver` so that provider
    /// hard limits clamp registry and user-override values.
    pub fn max_output_tokens(&self) -> usize {
        let resolver = self.provider_limits_resolver();
        resolve_max_output_tokens(
            self.registry_max_output_tokens,
            self.user_max_output_tokens,
            resolver.as_ref(),
        )
    }

    /// LIMITS-004: Override the per-model context window and max output tokens.
    ///
    /// Used by the NAPI layer to apply TypeScript overrides on top of
    /// models.dev registry data. Stored as user overrides so the resolver
    /// can clamp them by the provider's hard maximum.
    pub fn override_model_limits(
        &mut self,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) {
        if let Some(cw) = context_window {
            self.user_context_window = Some(cw);
        }
        if let Some(mot) = max_output_tokens {
            self.user_max_output_tokens = Some(mot);
        }
    }

    /// LIMITS-004: Get the resolved context window for sub-agent propagation.
    ///
    /// Returns `Some(context_window())` when any registry or user data exists,
    /// `None` when no model-specific data is available (sub-agent should use
    /// its own provider defaults).
    ///
    /// The returned value is always clamped by the provider's hard maximum.
    pub fn raw_model_context_window(&self) -> Option<usize> {
        if self.registry_context_window.is_some() || self.user_context_window.is_some() {
            Some(self.context_window())
        } else {
            None
        }
    }

    /// LIMITS-004: Get the resolved max output tokens for sub-agent propagation.
    ///
    /// Same semantics as `raw_model_context_window()` — returns clamped value
    /// or `None` when no model-specific data exists.
    pub fn raw_model_max_output_tokens(&self) -> Option<usize> {
        if self.registry_max_output_tokens.is_some() || self.user_max_output_tokens.is_some() {
            Some(self.max_output_tokens())
        } else {
            None
        }
    }

    /// MODEL-004: Get the facade override for agent loop dispatch.
    ///
    /// When `Some`, the agent loop should dispatch to this provider name
    /// instead of `current_provider_name()`. This allows custom models to
    /// route through a different provider backend.
    pub fn facade_override(&self) -> Option<&str> {
        self.facade_override.as_deref()
    }

    /// MODEL-004: Set or clear the facade override.
    pub fn set_facade_override(&mut self, facade: Option<String>) {
        self.facade_override = facade;
    }

    /// CTX-007: Get the compaction threshold override (type, value).
    /// Returns None if no user-configured override exists.
    pub fn compaction_threshold_override(&self) -> Option<(&str, u64)> {
        self.compaction_threshold_override.as_ref().map(|(t, v)| (t.as_str(), *v))
    }

    /// CTX-007: Set the compaction threshold override.
    /// type_str should be "tokens" or "percentage".
    pub fn set_compaction_threshold_override(&mut self, config: Option<(String, u64)>) {
        self.compaction_threshold_override = config;
    }

    /// Test-only constructor that creates a ProviderManager without requiring credentials.
    /// Exposed for workspace-level integration tests that need to call methods like
    /// `max_output_tokens()` without a real provider backend.
    ///
    /// LIMITS-004: context_window and max_output_tokens are stored as registry values
    /// and resolved through the provider's ModelLimitsResolver.
    #[doc(hidden)]
    pub fn for_testing(
        provider: ProviderType,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) -> Self {
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
            registry_context_window: context_window,
            registry_max_output_tokens: max_output_tokens,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
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

    // =========================================================================
    // MODEL-005: Per-Model Context Window and Max Output Configuration
    // Feature: spec/features/per-model-context-window-and-max-output-configuration.feature
    //
    // Tests for model-specific context_window and max_output_tokens resolution.
    // Priority: model-specific override > models.dev per-model > env var > provider constant.
    // =========================================================================

    /// Build a test registry containing an OpenAI o3 model with 200k context
    /// and 100k max output, plus an anthropic/claude-sonnet-4 model.
    fn build_multi_provider_registry() -> ModelRegistry {
        let mut openai_models = HashMap::new();
        openai_models.insert(
            "o3".to_string(),
            ModelInfo {
                id: "o3".to_string(),
                name: "o3".to_string(),
                family: Some("o3".to_string()),
                release_date: None,
                attachment: false,
                reasoning: true,
                tool_call: true,
                temperature: true,
                interleaved: None,
                modalities: None,
                cost: None,
                limit: LimitInfo {
                    context: 200_000,
                    output: 100_000,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );
        openai_models.insert(
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

        let mut anthropic_models = HashMap::new();
        anthropic_models.insert(
            "claude-sonnet-4".to_string(),
            ModelInfo {
                id: "claude-sonnet-4".to_string(),
                name: "Claude Sonnet 4".to_string(),
                family: Some("claude-sonnet".to_string()),
                release_date: None,
                attachment: false,
                reasoning: false,
                tool_call: true,
                temperature: true,
                interleaved: None,
                modalities: None,
                cost: None,
                limit: LimitInfo {
                    context: 200_000,
                    output: 8_192,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );

        let mut copilot_models = HashMap::new();
        copilot_models.insert(
            "gemini-2.5-pro".to_string(),
            ModelInfo {
                id: "gemini-2.5-pro".to_string(),
                name: "Gemini 2.5 Pro".to_string(),
                family: Some("gemini-2.5".to_string()),
                release_date: None,
                attachment: false,
                reasoning: false,
                tool_call: true,
                temperature: true,
                interleaved: None,
                modalities: None,
                cost: None,
                limit: LimitInfo {
                    context: 1_000_000,
                    output: 8_192,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );

        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                env: vec![],
                npm: None,
                api: Some("https://api.openai.com".to_string()),
                doc: None,
                models: openai_models,
            },
        );
        providers.insert(
            "anthropic".to_string(),
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                env: vec![],
                npm: None,
                api: Some("https://api.anthropic.com".to_string()),
                doc: None,
                models: anthropic_models,
            },
        );
        providers.insert(
            "github-copilot".to_string(),
            ProviderInfo {
                id: "github-copilot".to_string(),
                name: "GitHub Copilot".to_string(),
                env: vec![],
                npm: None,
                api: Some("https://api.githubcopilot.com".to_string()),
                doc: None,
                models: copilot_models,
            },
        );
        ModelRegistry::from_response(ModelsDevResponse { providers })
    }

    /// Build a test ProviderManager with multi-provider registry and
    /// credentials enabled for testing.
    fn test_manager_with_registry_and_credentials() -> ProviderManager {
        ProviderManager {
            credentials: ProviderCredentials {
                claude_available: true,
                openai_available: true,
                codex_available: false,
                gemini_available: false,
                zai_available: false,
                github_copilot_available: true,
            },
            current_provider: ProviderType::Claude,
            model_registry: Some(build_multi_provider_registry()),
            selected_model: None,
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
        }
    }

    // -------------------------------------------------------------------------
    // Scenario: Cloud model gets per-model context window from models.dev registry
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_select_model_stores_model_limits_from_registry() {
        // @step Given the model registry contains "openai/o3" with context=200000 and max_output=100000
        // Need real env var for select_model's re-detect
        std::env::set_var("OPENAI_API_KEY", "fake-key-for-model-005-test");
        let mut manager = test_manager_with_registry_and_credentials();

        // @step And the OpenAI provider-level constant is 128000
        assert_eq!(openai::CONTEXT_WINDOW, 128_000);

        // @step When I call select_model("openai/o3")
        let result = manager.select_model("openai/o3");
        std::env::remove_var("OPENAI_API_KEY");
        assert!(result.is_ok(), "select_model should succeed: {:?}", result.err());

        // @step Then model_context_window should be 200000
        assert_eq!(manager.registry_context_window, Some(200_000));

        // @step And model_max_output_tokens should be 100000
        assert_eq!(manager.registry_max_output_tokens, Some(100_000));

        // @step And context_window() should return 200000
        assert_eq!(manager.context_window(), 200_000);

        // @step And max_output_tokens() should return 100000
        assert_eq!(manager.max_output_tokens(), 100_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: Copilot proxy model gets per-model context from registry
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_copilot_proxy_model_gets_per_model_context_from_registry() {
        // @step Given the model registry contains "github-copilot/gemini-2.5-pro" with context=1000000 and max_output=8192
        let _guard = FspecHomeGuard::new();
        // Install a fake copilot credential so PROV-057 re-detect finds it
        let auth = CopilotAuthJson::from_github_oauth_token(
            "fake-github-oauth-token-for-model-005".to_string(),
            None,
        );
        // Write synchronously using the same approach as FspecHomeGuard tests
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(write_copilot_auth(&auth)).expect("write_copilot_auth should succeed");

        let mut manager = test_manager_with_registry_and_credentials();

        // @step And the Copilot provider-level constant is 200000
        assert_eq!(copilot::CONTEXT_WINDOW, 200_000);

        // @step When I call select_model("github-copilot/gemini-2.5-pro")
        let result = manager.select_model("github-copilot/gemini-2.5-pro");
        assert!(result.is_ok(), "select_model should succeed: {:?}", result.err());

        // @step Then context_window() should return 1000000
        assert_eq!(manager.context_window(), 1_000_000);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), 8_192);
    }

    // -------------------------------------------------------------------------
    // Scenario: Claude model gets per-model context from registry
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_claude_model_gets_per_model_context_from_registry() {
        // @step Given the model registry contains "anthropic/claude-sonnet-4" with context=200000 and max_output=8192
        std::env::set_var("ANTHROPIC_API_KEY", "fake-key-for-model-005-test");
        let mut manager = test_manager_with_registry_and_credentials();

        // @step When I call select_model("anthropic/claude-sonnet-4")
        let result = manager.select_model("anthropic/claude-sonnet-4");
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(result.is_ok(), "select_model should succeed: {:?}", result.err());

        // @step Then model_context_window should be 200000
        assert_eq!(manager.registry_context_window, Some(200_000));

        // @step And context_window() should return 200000
        assert_eq!(manager.context_window(), 200_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: No model selected falls back to provider constant
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_context_window_falls_back_to_provider_constant() {
        // @step Given a fresh ProviderManager with Claude as the current provider
        let manager = test_manager(ProviderType::Claude);

        // @step And no model has been selected
        assert!(manager.selected_model.is_none());

        // @step Then model_context_window should be None
        assert_eq!(manager.registry_context_window, None);

        // @step And context_window() should return 200000
        assert_eq!(manager.context_window(), claude::CONTEXT_WINDOW);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), claude::MAX_OUTPUT_TOKENS);
    }

    // -------------------------------------------------------------------------
    // Scenario: context_window returns model-specific value when set
    // -------------------------------------------------------------------------

    #[test]
    fn test_context_window_returns_model_specific_value() {
        // @step Given a ProviderManager with model_context_window=200000
        let mut manager = test_manager(ProviderType::OpenAI);
        manager.registry_context_window = Some(200_000);

        // @step Then context_window() should return 200000
        assert_eq!(manager.context_window(), 200_000);

        // @step And the value should differ from the OpenAI provider constant (128000)
        assert_ne!(manager.context_window(), openai::CONTEXT_WINDOW);
    }

    // -------------------------------------------------------------------------
    // Scenario: max_output_tokens returns model-specific value when set
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_max_output_tokens_returns_model_specific_value() {
        // @step Given a ProviderManager with model_max_output_tokens=100000
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
        let mut manager = test_manager(ProviderType::OpenAI);
        manager.registry_max_output_tokens = Some(100_000);

        // @step Then max_output_tokens() should return 100000
        assert_eq!(manager.max_output_tokens(), 100_000);

        // @step And the value should differ from the OpenAI provider constant (4096)
        assert_ne!(manager.max_output_tokens(), openai::MAX_OUTPUT_TOKENS);
    }

    // -------------------------------------------------------------------------
    // Scenario: Environment variable override still works when no per-model data
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_env_var_override_works_when_no_per_model_data() {
        // @step Given a fresh ProviderManager with OpenAI as the current provider
        let manager = test_manager(ProviderType::OpenAI);

        // @step And no model has been selected
        assert!(manager.selected_model.is_none());

        // @step And OPENAI_CONTEXT_WINDOW is set to "32000"
        std::env::set_var("OPENAI_CONTEXT_WINDOW", "32000");

        // @step And OPENAI_MAX_OUTPUT_TOKENS is set to "8192"
        std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "8192");

        // @step Then model_context_window should be None
        assert_eq!(manager.registry_context_window, None);

        // @step And context_window() should return 32000
        assert_eq!(manager.context_window(), 32_000);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), 8_192);

        // Clean up
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    }

    // -------------------------------------------------------------------------
    // Scenario: set_model_direct stores optional context params
    // -------------------------------------------------------------------------

    #[test]
    fn test_set_model_direct_stores_optional_context_params() {
        // @step Given a vLLM profile model with ModelSelection.contextWindow=32000 and maxOutput=4096
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096
        let result = manager.set_model_direct(
            "openai",
            "my-local-model",
            Some(32_000),
            Some(4_096),
            None,
        );
        assert!(result.is_ok());

        // @step Then set_model_direct stores model_context_window=32000 and model_max_output_tokens=4096
        assert_eq!(manager.user_context_window, Some(32_000));
        assert_eq!(manager.user_max_output_tokens, Some(4_096));

        // @step And context_window() should return 32000
        assert_eq!(manager.context_window(), 32_000);

        // @step And max_output_tokens() should return 4096
        assert_eq!(manager.max_output_tokens(), 4_096);
    }

    // -------------------------------------------------------------------------
    // Scenario: Codex model gets context window through NAPI parameters
    // -------------------------------------------------------------------------

    #[test]
    fn test_codex_model_gets_context_window_through_napi_params() {
        // @step Given a Codex model with ModelSelection.contextWindow=272000 and maxOutput=4096
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When sessionSetModelProfile is called with context_window=272000 and max_output_tokens=4096
        let result = manager.set_model_direct(
            "openai",
            "codex-model",
            Some(272_000),
            Some(4_096),
            None,
        );
        assert!(result.is_ok());

        // @step Then set_model_direct stores model_context_window=272000 and model_max_output_tokens=4096
        assert_eq!(manager.user_context_window, Some(272_000));
        assert_eq!(manager.user_max_output_tokens, Some(4_096));

        // @step And context_window() should return 272000
        assert_eq!(manager.context_window(), 272_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: set_model_direct without context params leaves None
    // -------------------------------------------------------------------------

    #[test]
    fn test_set_model_direct_without_context_params_leaves_none() {
        // @step Given a ProviderManager
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called without context params
        let result = manager.set_model_direct(
            "openai",
            "my-local-model",
            None,
            None,
            None,
        );
        assert!(result.is_ok());

        // @step Then model_context_window should remain None
        assert_eq!(manager.user_context_window, None);

        // @step And model_max_output_tokens should remain None
        assert_eq!(manager.user_max_output_tokens, None);
    }

    // -------------------------------------------------------------------------
    // Scenario: for_testing constructor with custom context window
    // -------------------------------------------------------------------------

    #[test]
    fn test_for_testing_with_custom_context() {
        // @step Given I create a test ProviderManager via for_testing(OpenAI, context_window=200000, max_output_tokens=100000)
        let manager = ProviderManager::for_testing(
            ProviderType::OpenAI,
            Some(200_000),
            Some(100_000),
        );

        // @step Then context_window() should return 200000
        assert_eq!(manager.context_window(), 200_000);

        // @step And max_output_tokens() should return 100000
        assert_eq!(manager.max_output_tokens(), 100_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: with_provider_and_model accepts optional context window parameters
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_with_provider_and_model_accepts_context_params() {
        // @step Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4", context_window=200000, max_output_tokens=8192)
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-for-provider-manager");
        let result = ProviderManager::with_provider_and_model(
            "claude",
            Some("claude-sonnet-4"),
            Some(200_000),
            Some(8_192),
        );
        std::env::remove_var("ANTHROPIC_API_KEY");

        assert!(result.is_ok(), "with_provider_and_model should succeed: {:?}", result.err());
        let manager = result.unwrap();

        // @step Then context_window() should return 200000
        assert_eq!(manager.context_window(), 200_000);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), 8_192);
    }

    // -------------------------------------------------------------------------
    // Scenario: with_provider_and_model without context params falls back to provider constant
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_with_provider_and_model_without_context_falls_back() {
        // @step Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4") with no context params
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-for-provider-manager");
        let result = ProviderManager::with_provider_and_model(
            "claude",
            Some("claude-sonnet-4"),
            None,
            None,
        );
        std::env::remove_var("ANTHROPIC_API_KEY");

        assert!(result.is_ok());
        let manager = result.unwrap();

        // @step Then context_window() should return 200000
        assert_eq!(manager.context_window(), claude::CONTEXT_WINDOW);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), claude::MAX_OUTPUT_TOKENS);
    }

    // -------------------------------------------------------------------------
    // Scenario: NAPI override takes priority over models.dev metadata
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_napi_override_takes_priority_over_registry() {
        // @step Given the model registry contains "openai/gpt-4o" with context=128000 and max_output=16384
        std::env::set_var("OPENAI_API_KEY", "fake-key-for-model-005-test");
        let mut manager = test_manager_with_registry_and_credentials();

        // @step When session_set_model is called with context_window=64000 and max_output_tokens=8192
        // First call select_model to store registry values, then override with NAPI params
        let result = manager.select_model("openai/gpt-4o");
        std::env::remove_var("OPENAI_API_KEY");
        assert!(result.is_ok());

        // Verify registry values are stored
        assert_eq!(manager.registry_context_window, Some(128_000));
        assert_eq!(manager.registry_max_output_tokens, Some(16_384));

        // Simulate NAPI override (the NAPI layer calls select_model first,
        // then overwrites with NAPI params if Some)
        manager.override_model_limits(Some(64_000), Some(8_192));

        // @step Then context_window() should return 64000
        assert_eq!(manager.context_window(), 64_000);

        // @step And max_output_tokens() should return 8192
        assert_eq!(manager.max_output_tokens(), 8_192);
    }

    // -------------------------------------------------------------------------
    // Scenario: Provider-level compile-time constants remain unchanged
    // -------------------------------------------------------------------------

    #[test]
    fn test_provider_constants_remain_unchanged() {
        // @step Then claude::CONTEXT_WINDOW should be 200000
        assert_eq!(claude::CONTEXT_WINDOW, 200_000);

        // @step And openai::CONTEXT_WINDOW should be 128000
        assert_eq!(openai::CONTEXT_WINDOW, 128_000);

        // @step And gemini::CONTEXT_WINDOW should be 1000000
        assert_eq!(gemini::CONTEXT_WINDOW, 1_000_000);

        // @step And codex::CONTEXT_WINDOW should be 272000
        assert_eq!(codex::CONTEXT_WINDOW, 272_000);

        // @step And zai::CONTEXT_WINDOW should be 128000
        assert_eq!(zai::CONTEXT_WINDOW, 128_000);

        // @step And copilot::CONTEXT_WINDOW should be 200000
        assert_eq!(copilot::CONTEXT_WINDOW, 200_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: Compaction threshold uses per-model context window for large-context model
    // (Validates that ProviderManager values flow correctly into the compaction calc)
    // -------------------------------------------------------------------------

    #[test]
    fn test_compaction_threshold_large_context_model() {
        // @step Given a ProviderManager with model_context_window=200000 and model_max_output_tokens=100000
        let mut manager = test_manager(ProviderType::OpenAI);
        manager.registry_context_window = Some(200_000);
        manager.registry_max_output_tokens = Some(100_000);

        // @step When the compaction threshold is calculated
        let cw = manager.context_window() as u64;
        let mot = manager.max_output_tokens() as u64;
        // calculate_usable_context logic: context_window - min(max_output, 32000)
        let output_reservation = mot.min(32_000);
        let usable = cw.saturating_sub(output_reservation);

        // @step Then calculate_usable_context(200000, 100000) should return 168000
        assert_eq!(usable, 168_000);

        // @step And compaction triggers when effective tokens exceed 168000
        assert!(168_001 > usable, "tokens exceeding threshold should trigger compaction");
    }

    // -------------------------------------------------------------------------
    // Scenario: Compaction threshold uses per-model context window for small-context model
    // -------------------------------------------------------------------------

    #[test]
    fn test_compaction_threshold_small_context_model() {
        // @step Given a ProviderManager with model_context_window=32000 and model_max_output_tokens=4096
        let mut manager = test_manager(ProviderType::OpenAI);
        manager.registry_context_window = Some(32_000);
        manager.registry_max_output_tokens = Some(4_096);

        // @step When the compaction threshold is calculated
        let cw = manager.context_window() as u64;
        let mot = manager.max_output_tokens() as u64;
        let output_reservation = mot.min(32_000);
        let usable = cw.saturating_sub(output_reservation);

        // @step Then calculate_usable_context(32000, 4096) should return 27904
        assert_eq!(usable, 27_904);

        // @step And compaction triggers when effective tokens exceed 27904
        assert!(27_905 > usable, "tokens exceeding threshold should trigger compaction");
    }

    // -------------------------------------------------------------------------
    // Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModel
    // (Rust-side test validates the ProviderManager stores values correctly from NAPI)
    // -------------------------------------------------------------------------

    #[test]
    #[serial_test::serial]
    fn test_session_set_model_passes_context_params() {
        // @step Given a ModelSelection with providerId="openai" and modelId="o3" and contextWindow=200000 and maxOutput=100000
        std::env::set_var("OPENAI_API_KEY", "fake-key-for-model-005-test");
        let mut manager = test_manager_with_registry_and_credentials();

        // @step And an active session exists
        // (Simulated: we use select_model + override, matching what NAPI does)

        // @step When selectModel is called
        let result = manager.select_model("openai/o3");
        std::env::remove_var("OPENAI_API_KEY");
        assert!(result.is_ok());

        // Simulate NAPI override
        manager.registry_context_window = Some(200_000);
        manager.registry_max_output_tokens = Some(100_000);

        // @step Then sessionSetModel is called with context_window=200000 and max_output_tokens=100000
        assert_eq!(manager.context_window(), 200_000);
        assert_eq!(manager.max_output_tokens(), 100_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModelProfile
    // -------------------------------------------------------------------------

    #[test]
    fn test_session_set_model_profile_passes_context_params() {
        // @step Given a ModelSelection with profileConfig and contextWindow=32000 and maxOutput=4096
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step And an active session exists
        // (Simulated: direct call to set_model_direct with context params)

        // @step When selectModel is called
        let result = manager.set_model_direct(
            "openai",
            "local-model",
            Some(32_000),
            Some(4_096),
            None,
        );
        assert!(result.is_ok());

        // @step Then sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096
        assert_eq!(manager.context_window(), 32_000);
        assert_eq!(manager.max_output_tokens(), 4_096);
    }

    // -------------------------------------------------------------------------
    // MODEL-004: Facade Override Tests
    // Feature: spec/features/custom-model-registration-and-facade-override-in-model-selector.feature
    // -------------------------------------------------------------------------

    #[test]
    fn test_set_model_direct_stores_facade_override() {
        // @step Given a custom model with facade="codex"
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called with facade_override=Some("codex")
        let result = manager.set_model_direct(
            "openai",
            "my-custom-model",
            Some(32_000),
            Some(4_096),
            Some("codex".to_string()),
        );
        assert!(result.is_ok());

        // @step Then facade_override() should return Some("codex")
        assert_eq!(manager.facade_override(), Some("codex"));
    }

    #[test]
    fn test_set_model_direct_without_facade_leaves_none() {
        // @step Given a profile model without facade override
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called with facade_override=None
        let result = manager.set_model_direct(
            "openai",
            "my-local-model",
            None,
            None,
            None,
        );
        assert!(result.is_ok());

        // @step Then facade_override() should return None
        assert_eq!(manager.facade_override(), None);
    }

    #[test]
    fn test_set_facade_override_setter() {
        // @step Given a ProviderManager with no facade override
        let mut manager = test_manager(ProviderType::OpenAI);
        assert_eq!(manager.facade_override(), None);

        // @step When set_facade_override is called with Some("gemini")
        manager.set_facade_override(Some("gemini".to_string()));

        // @step Then facade_override() should return Some("gemini")
        assert_eq!(manager.facade_override(), Some("gemini"));

        // @step When set_facade_override is called with None
        manager.set_facade_override(None);

        // @step Then facade_override() should return None
        assert_eq!(manager.facade_override(), None);
    }

    #[test]
    fn test_facade_override_initialized_none_in_all_constructors() {
        // @step Given ProviderManagers created via different constructors
        // test_manager uses direct construction
        let mgr = test_manager(ProviderType::Claude);
        assert_eq!(mgr.facade_override(), None);

        // for_testing constructor
        let mgr = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
        assert_eq!(mgr.facade_override(), None);
    }

    // =========================================================================
    // LIMITS-004: ProviderManager resolves through ModelLimitsResolver
    // Feature: spec/features/fix-providermanager-resolution-chain-use-modellimitsresolver.feature
    // =========================================================================

    /// Scenario: Claude context window is clamped from 1M registry to 200k
    #[test]
    fn test_claude_context_window_clamped_from_1m_to_200k() {
        // @step Given a ProviderManager configured for the Claude provider
        let mut manager = test_manager(ProviderType::Claude);

        // @step And select_model stores a registry context window of 1000000
        manager.registry_context_window = Some(1_000_000);

        // @step And the Claude resolver declares max_context_window as 200000
        // (built into the resolver — ClaudeProvider::max_context_window() returns Some(200_000))

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 200000
        assert_eq!(result, 200_000);
    }

    /// Scenario: Claude max output tokens clamped from 128k registry to 8192
    #[test]
    fn test_claude_max_output_clamped_from_128k_to_8192() {
        // @step Given a ProviderManager configured for the Claude provider
        let mut manager = test_manager(ProviderType::Claude);

        // @step And select_model stores a registry max output of 128000
        manager.registry_max_output_tokens = Some(128_000);

        // @step And the Claude resolver declares max_output_tokens_limit as 8192
        // (built into the resolver)

        // @step When max_output_tokens() is called
        let result = manager.max_output_tokens();

        // @step Then the result should be 8192
        assert_eq!(result, 8_192);
    }

    /// Scenario: OpenAI context window passes through unclamped
    #[test]
    fn test_openai_context_window_passes_through_unclamped() {
        // @step Given a ProviderManager configured for the OpenAI provider
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step And select_model stores a registry context window of 128000
        manager.registry_context_window = Some(128_000);

        // @step And the OpenAI resolver declares max_context_window as None
        // (built into the resolver — OpenAI trusts registry)

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 128000
        assert_eq!(result, 128_000);
    }

    /// Scenario: Codex with no registry data returns provider default
    #[test]
    fn test_codex_no_registry_returns_default() {
        // @step Given a ProviderManager configured for the Codex provider
        let manager = test_manager(ProviderType::Codex);

        // @step And no registry context window is set
        assert_eq!(manager.registry_context_window, None);

        // @step And no user context window override is set
        assert_eq!(manager.user_context_window, None);

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 272000
        assert_eq!(result, 272_000);
    }

    /// Scenario: User override is clamped by provider max
    #[test]
    fn test_user_override_clamped_by_provider_max() {
        // @step Given a ProviderManager configured for the Claude provider
        let mut manager = test_manager(ProviderType::Claude);

        // @step And the user overrides context window to 500000 via NAPI
        manager.override_model_limits(Some(500_000), None);

        // @step And the Claude resolver declares max_context_window as 200000
        // (built into the resolver)

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 200000
        assert_eq!(result, 200_000);
    }

    /// Scenario: User override takes priority over registry value
    #[test]
    #[serial_test::serial]
    fn test_user_override_takes_priority_over_registry() {
        // @step Given a ProviderManager configured for the OpenAI provider
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step And select_model stores a registry context window of 128000
        manager.registry_context_window = Some(128_000);

        // @step And the user overrides context window to 64000 via NAPI
        manager.override_model_limits(Some(64_000), None);

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 64000
        assert_eq!(result, 64_000);
    }

    /// Scenario: Sub-agent propagation returns clamped values
    #[test]
    fn test_sub_agent_propagation_returns_clamped_values() {
        // @step Given a ProviderManager configured for the Claude provider
        let mut manager = test_manager(ProviderType::Claude);

        // @step And select_model stores a registry context window of 1000000
        manager.registry_context_window = Some(1_000_000);

        // @step When raw_model_context_window() is called for sub-agent propagation
        let raw = manager.raw_model_context_window();

        // @step Then the result should be 200000
        assert_eq!(raw, Some(200_000));

        // @step And it should equal the value from context_window()
        assert_eq!(raw, Some(manager.context_window()));
    }

    /// Scenario: OpenAI env var fallback when no registry data
    #[test]
    #[serial_test::serial]
    fn test_openai_env_var_fallback_when_no_registry() {
        // @step Given a ProviderManager configured for the OpenAI provider
        let manager = test_manager(ProviderType::OpenAI);

        // @step And no registry context window is set
        assert_eq!(manager.registry_context_window, None);

        // @step And no user context window override is set
        assert_eq!(manager.user_context_window, None);

        // @step And the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
        std::env::set_var("OPENAI_CONTEXT_WINDOW", "256000");

        // @step When context_window() is called
        let result = manager.context_window();

        // @step Then the result should be 256000
        assert_eq!(result, 256_000);

        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    }
}
