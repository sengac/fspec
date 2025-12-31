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
    claude, codex, gemini, openai, ClaudeProvider, CodexProvider, GeminiProvider, OpenAIProvider,
    ProviderError,
};
use std::str::FromStr;

/// Provider type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
}

impl FromStr for ProviderType {
    type Err = ProviderError;

    fn from_str(name: &str) -> Result<Self, ProviderError> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" => Ok(ProviderType::Gemini),
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
        }
    }
}

/// Provider Manager for dynamic provider selection
///
/// MODEL-001: Now includes optional ModelRegistry for dynamic model selection
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    /// MODEL-001: Optional model registry for dynamic model selection
    model_registry: Option<ModelRegistry>,
    /// MODEL-001: Selected model string (provider/model-id format)
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
        let has_credentials = match requested_provider {
            ProviderType::Claude => credentials.has_claude(),
            ProviderType::OpenAI => credentials.has_openai(),
            ProviderType::Codex => credentials.has_codex(),
            ProviderType::Gemini => credentials.has_gemini(),
        };

        if !has_credentials {
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
        let cache = ModelCache::new();
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
        let has_credentials = match provider_type {
            ProviderType::Claude => self.credentials.has_claude(),
            ProviderType::OpenAI => self.credentials.has_openai(),
            ProviderType::Codex => self.credentials.has_codex(),
            ProviderType::Gemini => self.credentials.has_gemini(),
        };

        if !has_credentials {
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

    /// MODEL-001: Get the selected model ID (the actual API model ID)
    ///
    /// Returns the model ID to use for API calls. If a model was explicitly selected,
    /// returns that model's ID from the registry. Otherwise returns None (use default).
    pub fn selected_model_id(&self) -> Option<String> {
        let model_string = self.selected_model.as_ref()?;
        let registry = self.model_registry.as_ref()?;

        // Parse and get the model info
        if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
            if let Ok(model_info) = registry.get_model(&provider_id, &model_id) {
                return Some(model_info.id.clone());
            }
        }

        None
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
            // Codex uses OAuth flow, not a models.dev provider
            _ => Err(ProviderError::config(
                "manager",
                format!(
                    "Provider '{provider_id}' is not supported. Supported providers: anthropic, openai, google"
                ),
            )),
        }
    }

    /// Detect default provider based on priority
    fn detect_default_provider(
        credentials: &ProviderCredentials,
    ) -> Result<ProviderType, ProviderError> {
        // Priority: Claude > Gemini > Codex > OpenAI
        if credentials.has_claude() {
            return Ok(ProviderType::Claude);
        }
        if credentials.has_gemini() {
            return Ok(ProviderType::Gemini);
        }
        if credentials.has_codex() {
            return Ok(ProviderType::Codex);
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
    pub fn get_claude(&self) -> Result<ClaudeProvider, ProviderError> {
        if self.current_provider == ProviderType::Claude {
            ClaudeProvider::new_with_model(self.selected_model_id().as_deref())
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not Claude",
            ))
        }
    }

    /// Get OpenAI provider (if selected)
    ///
    /// MODEL-001: Now uses selected_model_id() for dynamic model selection.
    pub fn get_openai(&self) -> Result<OpenAIProvider, ProviderError> {
        if self.current_provider == ProviderType::OpenAI {
            // OpenAI's new() already handles env var detection
            // For MODEL-001, we need to create with explicit model if selected
            if let Some(model_id) = self.selected_model_id() {
                let api_key = std::env::var("OPENAI_API_KEY")
                    .map_err(|_| ProviderError::auth("openai", "OPENAI_API_KEY not set"))?;
                OpenAIProvider::from_api_key(&api_key, &model_id)
            } else {
                OpenAIProvider::new()
            }
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not OpenAI",
            ))
        }
    }

    /// Get Codex provider (if selected)
    pub fn get_codex(&self) -> Result<CodexProvider, ProviderError> {
        if self.current_provider == ProviderType::Codex {
            CodexProvider::new()
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not Codex",
            ))
        }
    }

    /// Get Gemini provider (if selected)
    ///
    /// MODEL-001: Now uses selected_model_id() for dynamic model selection.
    pub fn get_gemini(&self) -> Result<GeminiProvider, ProviderError> {
        if self.current_provider == ProviderType::Gemini {
            // Gemini's new() already handles env var detection
            // For MODEL-001, we need to create with explicit model if selected
            if let Some(model_id) = self.selected_model_id() {
                let api_key = std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").map_err(|_| {
                    ProviderError::auth("gemini", "GOOGLE_GENERATIVE_AI_API_KEY not set")
                })?;
                GeminiProvider::from_api_key(&api_key, &model_id)
            } else {
                GeminiProvider::new()
            }
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not Gemini",
            ))
        }
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
        if self.credentials.has_codex() {
            providers.push("Codex (/codex)".to_string());
        }
        providers
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<(), ProviderError> {
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        let has_credentials = match requested_provider {
            ProviderType::Claude => self.credentials.has_claude(),
            ProviderType::OpenAI => self.credentials.has_openai(),
            ProviderType::Codex => self.credentials.has_codex(),
            ProviderType::Gemini => self.credentials.has_gemini(),
        };

        if !has_credentials {
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
    /// References the canonical CONTEXT_WINDOW constant from each provider module.
    pub fn context_window(&self) -> usize {
        match self.current_provider {
            ProviderType::Claude => claude::CONTEXT_WINDOW,
            ProviderType::OpenAI => openai::CONTEXT_WINDOW,
            ProviderType::Gemini => gemini::CONTEXT_WINDOW,
            ProviderType::Codex => codex::CONTEXT_WINDOW,
        }
    }

    /// Get max output tokens for the current provider (CTX-002)
    ///
    /// Returns the maximum output tokens for the currently selected provider.
    /// Used for calculating usable context in the optimized compaction algorithm.
    pub fn max_output_tokens(&self) -> usize {
        match self.current_provider {
            ProviderType::Claude => claude::MAX_OUTPUT_TOKENS,
            ProviderType::OpenAI => openai::MAX_OUTPUT_TOKENS,
            ProviderType::Gemini => gemini::MAX_OUTPUT_TOKENS,
            ProviderType::Codex => codex::MAX_OUTPUT_TOKENS,
        }
    }
}
