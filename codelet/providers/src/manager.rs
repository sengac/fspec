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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    /// PROV-053: GitHub Copilot via OAuth device flow
    GitHubCopilot,
    /// PROV-067: Custom provider discovered from a JSON config in
    /// `~/.fspec/providers/` or `.fspec/providers/`. The inner `String`
    /// is the provider slug (matching [`crate::custom::ProviderConfig::name`]).
    ///
    /// `Copy` has been removed from the derive list because `String` is
    /// not `Copy`. The enum is only used at session-creation time, not
    /// in hot loops, so this is a non-issue.
    Custom(String),
}

impl FromStr for ProviderType {
    type Err = ProviderError;

    fn from_str(name: &str) -> Result<Self, ProviderError> {
        let lowered = name.to_lowercase();

        // PROV-085: Shadowing precedence. Before matching hardcoded
        // built-in slugs, consult the custom provider registry so that a
        // discovered config named e.g. "claude" wins over
        // `ProviderType::Claude`. The escape hatch
        // `FSPEC_DISABLE_SCRIPT_SHADOWING=1` bypasses the lookup so CI
        // can regression-test the hardcoded path.
        if shadowing_enabled() && custom_provider_registered(&lowered) {
            return Ok(ProviderType::Custom(lowered));
        }

        match lowered.as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" => Ok(ProviderType::Gemini),
            "zai" => Ok(ProviderType::ZAI),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            other => {
                // PROV-067: Before failing with "Unknown provider", consult
                // the custom provider registry. Any config whose `name`
                // equals the requested slug resolves to a Custom variant.
                // PROV-085: When shadowing is disabled the registry still
                // resolves unknown slugs here — only built-in slugs are
                // locked to their hardcoded variants.
                if custom_provider_registered(other) {
                    Ok(ProviderType::Custom(other.to_string()))
                } else {
                    Err(ProviderError::config(
                        "manager",
                        format!("Unknown provider: {name}"),
                    ))
                }
            }
        }
    }
}

impl ProviderType {
    /// Get provider name as string.
    ///
    /// PROV-067: Signature was changed from `(self) -> &'static str` to
    /// `(&self) -> &str` so the `Custom(String)` variant can borrow from
    /// its inner `String` without leaking or allocating.
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::Claude => "claude",
            ProviderType::OpenAI => "openai",
            ProviderType::Codex => "codex",
            ProviderType::Gemini => "gemini",
            ProviderType::ZAI => "zai",
            ProviderType::GitHubCopilot => "github-copilot",
            ProviderType::Custom(name) => name.as_str(),
        }
    }

    /// Check if this provider type has credentials available
    ///
    /// DRY: Centralizes credential checking instead of repeating the match pattern.
    /// PROV-067: Takes `&self` and delegates `Custom` to
    /// [`ProviderCredentials::has_custom`].
    pub fn has_credentials(&self, credentials: &ProviderCredentials) -> bool {
        match self {
            ProviderType::Claude => credentials.has_claude(),
            ProviderType::OpenAI => credentials.has_openai(),
            ProviderType::Codex => credentials.has_codex(),
            ProviderType::Gemini => credentials.has_gemini(),
            ProviderType::ZAI => credentials.has_zai(),
            ProviderType::GitHubCopilot => credentials.has_github_copilot(),
            ProviderType::Custom(name) => credentials.has_custom(name),
        }
    }
}

/// PROV-067: Returns `true` when a custom provider with `slug` is
/// present in the current discovery set. Used by [`FromStr`] and
/// [`ProviderManager::map_provider_id_to_type`] to resolve unknown
/// provider names before falling through to an error. Discovery errors
/// are treated as "not registered" so parsing never panics on a
/// malformed config file.
///
/// PROV-096: Exposed publicly so the NAPI layer can detect custom
/// providers at session-creation time (`create_session_with_id`) and
/// route them through `set_model_direct` — bypassing the models.dev
/// registry lookup that otherwise fails with "Unknown provider:
/// 'claude-rhai'" for any non-builtin provider slug.
pub fn custom_provider_registered(slug: &str) -> bool {
    match crate::custom::discover_provider_configs() {
        Ok(configs) => configs.iter().any(|c| c.name == slug),
        Err(_) => false,
    }
}

/// PROV-085: Returns `true` when the shadowing precedence rule should
/// apply — i.e. a discovered custom provider config may shadow a
/// hardcoded built-in provider.
///
/// The escape hatch `FSPEC_DISABLE_SCRIPT_SHADOWING=1` disables
/// shadowing so CI can regression-test the hardcoded built-in path
/// even when a `claude.json` / `codex.json` config is installed in
/// `~/.fspec/providers/`.
fn shadowing_enabled() -> bool {
    match std::env::var("FSPEC_DISABLE_SCRIPT_SHADOWING") {
        Ok(value) => value != "1",
        Err(_) => true,
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
    /// The bare API model id as the remote provider knows it — e.g.
    /// "claude-sonnet-4", "gpt-4o", or
    /// "accounts/fireworks/models/kimi-k2-06-instruct".
    ///
    /// This is exactly the value passed to provider-specific API constructors
    /// (`get_claude()`, `get_openai()`, etc.) and may itself contain slashes
    /// for providers like Fireworks AI or OpenRouter that embed a
    /// hierarchical path in the model id.
    selected_model: Option<String>,
    /// BUG-136: Registry-format provider slug associated with
    /// `selected_model` (e.g. "anthropic", "openai", "kimi-rhai"). Stored
    /// separately because model ids may themselves contain slashes, so the
    /// "provider/model" composite returned by `selected_model_string()` is
    /// not generically parseable after the fact. Callers that need the full
    /// registry-format string (notably `AgentManager` / `DeepSearch` handler
    /// registration, which rounds-trips it through
    /// `SessionManager::create_session_with_id()`) consume
    /// `selected_model_string()`; callers that need the bare API id
    /// consume `selected_model_id()`.
    selected_registry_provider_id: Option<String>,
    /// BUG-137: Optional profile name associated with `selected_model`
    /// (e.g. "fireworks" in `openai:fireworks/accounts/fireworks/models/kimi-k2p6`).
    ///
    /// When present, `selected_model_string()` emits the profile-qualified
    /// composite `"{provider}:{profile}/{model}"` instead of the plain
    /// `"{provider}/{model}"`. This preserves the profile selection across
    /// the AgentManager spawn round-trip so subordinate sessions created
    /// via `create_session_with_id()` detect the profile format (colon
    /// before first slash) and route through `set_model_direct` instead
    /// of failing registry validation in `select_model`.
    ///
    /// Set exclusively by `set_model_direct_with_profile()` /
    /// `session_set_model_profile` NAPI. All other entry points leave it
    /// as `None`, which produces the plain composite (unchanged behaviour).
    selected_profile_name: Option<String>,
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
            selected_registry_provider_id: None,
            selected_profile_name: None,
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
            selected_registry_provider_id: None,
            selected_profile_name: None,
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

        // BUG-136: Store the bare model id plus the registry provider slug.
        // `selected_model_string()` rebuilds the composite on demand so
        // model ids containing slashes (Fireworks AI, OpenRouter) can't be
        // mis-parsed by downstream consumers.
        Ok(Self {
            credentials,
            current_provider: requested_provider,
            model_registry: None,
            selected_model: model_id.map(String::from),
            selected_registry_provider_id: model_id.map(|_| provider_name.to_string()),
            selected_profile_name: None,
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
            selected_registry_provider_id: None,
            selected_profile_name: None,
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

        // Update state. BUG-136: store the bare model slug in
        // `selected_model` and the registry provider id in
        // `selected_registry_provider_id`. The `provider/model` composite is
        // rebuilt on demand by `selected_model_string()`. Keeping the two
        // pieces separate is essential for providers whose model ids
        // themselves contain slashes (Fireworks AI, OpenRouter, etc.) where
        // the composite is otherwise ambiguous to parse.
        self.current_provider = provider_type;
        self.selected_model = Some(model_id.to_string());
        self.selected_registry_provider_id = Some(provider_id.to_string());
        // BUG-137: Cloud model selection has no profile. Clear any stale
        // profile name left over from a previous `set_model_direct_with_profile`
        // call so `selected_model_string()` emits the plain
        // `"provider/model"` composite.
        self.selected_profile_name = None;

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
        // BUG-137: Delegate to the profile-aware variant with profile=None.
        // Existing call sites (codex, custom providers, unit tests, session
        // initialization) behave exactly as before; only the new
        // profile-aware NAPI path passes a profile name.
        self.set_model_direct_with_profile(
            provider_id,
            model_id,
            None,
            context_window,
            max_output_tokens,
            facade_override,
        )
    }

    /// BUG-137: Variant of `set_model_direct` that also records a profile
    /// name.
    ///
    /// When `profile_name` is `Some(_)`, `selected_model_string()` emits
    /// the profile-qualified composite `"{provider}:{profile}/{model}"`
    /// (e.g. `"openai:fireworks/accounts/fireworks/models/kimi-k2p6"`).
    /// When `None`, the emitted composite is the plain `"{provider}/{model}"`
    /// — identical to the historical `set_model_direct` behaviour.
    ///
    /// This preserves the profile selection across the AgentManager spawn
    /// round-trip so subordinate sessions created via
    /// `create_session_with_id()` detect the profile format (colon before
    /// first slash) and route through `set_model_direct` again instead
    /// of failing registry validation in `select_model` with
    /// `"Model 'accounts/fireworks/...' not found in provider 'openai'"`.
    pub fn set_model_direct_with_profile(
        &mut self,
        provider_id: &str,
        model_id: &str,
        profile_name: Option<&str>,
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

        // BUG-136: Store the bare model id in `selected_model` and the
        // registry provider id in `selected_registry_provider_id` —
        // `selected_model_string()` rebuilds the composite on demand.
        // Previously this path stored only `model_id`, so
        // `selected_model_string()` returned an incomplete string with no
        // provider prefix. When `AgentManager` captured that string and
        // passed it to `create_session_with_id()` the first slash segment
        // was mis-parsed as the provider name — fine for bare model ids,
        // but broken for Fireworks/OpenRouter-style ids that contain
        // slashes (e.g. "accounts/fireworks/models/kimi-k2-06-instruct").
        self.current_provider = provider_type;
        self.selected_model = Some(model_id.to_string());
        self.selected_registry_provider_id = Some(provider_id.to_string());
        // BUG-137: Store (or clear) the profile name so
        // `selected_model_string()` can emit the profile-qualified
        // composite.
        self.selected_profile_name = profile_name.map(String::from);
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

    /// Get the selected model id (the actual API model id).
    ///
    /// If a model registry is available, a registry lookup is performed so
    /// slug selections (e.g. "claude-sonnet-4") resolve to their versioned
    /// API id (e.g. "claude-sonnet-4-20250514"). Otherwise the bare model id
    /// stored by `select_model()` / `set_model_direct()` /
    /// `with_provider_and_model()` is returned verbatim.
    ///
    /// BUG-136: This method now returns exactly the bare model id — it no
    /// longer needs to strip a provider prefix, because the provider id is
    /// tracked separately in `selected_registry_provider_id`. Model ids
    /// that contain slashes (e.g.
    /// "accounts/fireworks/models/kimi-k2-06-instruct") pass through
    /// unchanged.
    pub fn selected_model_id(&self) -> Option<String> {
        let model_id = self.selected_model.as_ref()?;

        // If we have both a registry and a known provider slug, resolve the
        // slug to the canonical API id (e.g. slug → versioned id).
        if let (Some(registry), Some(provider_id)) = (
            self.model_registry.as_ref(),
            self.selected_registry_provider_id.as_deref(),
        ) {
            if let Ok(model_info) = registry.get_model(provider_id, model_id) {
                return Some(model_info.id.clone());
            }
        }

        Some(model_id.clone())
    }

    /// MODEL-001: Get model info for the selected model, if known to the
    /// registry.
    pub fn selected_model_info(&self) -> Option<&ModelInfo> {
        let registry = self.model_registry.as_ref()?;
        let provider_id = self.selected_registry_provider_id.as_deref()?;
        let model_id = self.selected_model.as_deref()?;
        registry.get_model(provider_id, model_id).ok()
    }

    /// MODEL-001: Get the registry-format model string ("provider/model").
    ///
    /// BUG-136: The composite is rebuilt on demand from
    /// `selected_registry_provider_id` + `selected_model`. Returning an
    /// owned `String` (rather than the previous borrowed `&str`) is
    /// necessary because the composite may not be stored as a single field.
    /// Model ids that contain slashes round-trip cleanly because the
    /// provider slug is tracked independently.
    ///
    /// BUG-137: When a profile name has been recorded (via
    /// `set_model_direct_with_profile`), the composite takes the
    /// profile-qualified form `"{provider}:{profile}/{model}"` — e.g.
    /// `"openai:fireworks/accounts/fireworks/models/kimi-k2p6"`. This
    /// matches the format produced by the TypeScript
    /// `buildModelString()` helper and consumed by
    /// `SessionManager::create_session_with_id()`, whose profile-model
    /// branch is triggered by finding `':'` before the first `'/'`.
    /// Without this, `AgentManager.spawn` captured a plain
    /// `"openai/accounts/..."` string and the subordinate path treated
    /// the model as a cloud model, failing registry validation.
    pub fn selected_model_string(&self) -> Option<String> {
        let provider_id = self.selected_registry_provider_id.as_deref()?;
        let model_id = self.selected_model.as_deref()?;
        match self.selected_profile_name.as_deref() {
            Some(profile) => Some(format!("{provider_id}:{profile}/{model_id}")),
            None => Some(format!("{provider_id}/{model_id}")),
        }
    }

    /// MODEL-001: Get the model registry (for CLI commands like `codelet models`)
    pub fn model_registry(&self) -> Option<&ModelRegistry> {
        self.model_registry.as_ref()
    }

    /// MODEL-001: Map models.dev provider ID to our ProviderType
    fn map_provider_id_to_type(provider_id: &str) -> Result<ProviderType, ProviderError> {
        // PROV-085: Shadowing precedence also applies to models.dev IDs
        // so that `--model <shadowing-slug>/<model>` routes through the
        // custom config. The same escape hatch
        // `FSPEC_DISABLE_SCRIPT_SHADOWING=1` disables this lookup.
        if shadowing_enabled() && custom_provider_registered(provider_id) {
            return Ok(ProviderType::Custom(provider_id.to_string()));
        }

        match provider_id {
            "anthropic" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "google" => Ok(ProviderType::Gemini),
            "zai" | "z-ai" => Ok(ProviderType::ZAI),
            "codex" => Ok(ProviderType::Codex),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            other => {
                // PROV-067: Consult the custom provider registry before
                // failing. A custom provider slug maps 1:1 to
                // `ProviderType::Custom(slug)`.
                if custom_provider_registered(other) {
                    Ok(ProviderType::Custom(other.to_string()))
                } else {
                    Err(ProviderError::config(
                        "manager",
                        format!(
                            "Provider '{provider_id}' is not supported. Supported providers: anthropic, openai, google, zai, codex, github-copilot"
                        ),
                    ))
                }
            }
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

    /// PROV-067: Access the current [`ProviderType`] by reference. The
    /// `Custom(String)` variant is not `Copy`, so callers use this
    /// accessor instead of reading a public field.
    pub fn current_provider_type(&self) -> &ProviderType {
        &self.current_provider
    }

    /// PROV-067: Test-only shim around the private
    /// [`Self::detect_default_provider`] helper, so integration tests
    /// can verify custom providers never auto-select even when they're
    /// the only credentialed provider.
    #[doc(hidden)]
    pub fn detect_default_provider_for_test(
        credentials: &ProviderCredentials,
    ) -> Result<ProviderType, ProviderError> {
        Self::detect_default_provider(credentials)
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
        // PROV-067: Include discovered custom providers — both available
        // and unavailable — so users can see their full registry.
        let mut custom_names: Vec<&String> = self.credentials.custom_available.keys().collect();
        custom_names.sort();
        for name in custom_names {
            let available = self
                .credentials
                .custom_available
                .get(name)
                .copied()
                .unwrap_or(false);
            if available {
                providers.push(format!("{name} (/{name}) (custom)"));
            } else {
                providers.push(format!("{name} (/{name}) (custom, unavailable)"));
            }
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
        match &self.current_provider {
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
            // PROV-067: Custom providers have no hard ceiling — trust
            // user/registry overrides with conservative OpenAI-compatible
            // defaults (128k ctx, 4k out).
            ProviderType::Custom(_) => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: self.user_context_window.unwrap_or(128_000),
                max_out: None,
                default_out: self.user_max_output_tokens.unwrap_or(4096),
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
                custom_available: std::collections::HashMap::new(),
            },
            current_provider: provider,
            model_registry: None,
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
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
                custom_available: std::collections::HashMap::new(),
            },
            current_provider: provider,
            model_registry: None,
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
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
                custom_available: std::collections::HashMap::new(),
            },
            current_provider: ProviderType::Claude,
            model_registry: Some(build_github_copilot_registry()),
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
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
            manager.selected_model_string().as_deref(),
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
                custom_available: std::collections::HashMap::new(),
            },
            current_provider: ProviderType::Claude,
            model_registry: Some(build_multi_provider_registry()),
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
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

    // =========================================================================
    // BUG-136: set_model_direct stores provider-prefixed model string
    // Feature: spec/features/set-model-direct-stores-provider-prefixed-model-string.feature
    // =========================================================================

    /// Scenario: Custom model with slashes in model_id round-trips through
    /// the AgentManager handler capture (i.e. through `selected_model_string`)
    /// without losing the provider prefix.
    ///
    /// This is the regression scenario for the original Fireworks AI
    /// failure. `AgentManager::register_handler` captures
    /// `selected_model_string()` and forwards it to
    /// `create_session_with_id()`, which then splits at the first `/`.
    /// Before BUG-136, `set_model_direct` stored only the bare model id so
    /// `create_session_with_id` mis-interpreted the first slash segment
    /// ("accounts") as the provider name.
    #[test]
    fn test_bug136_agent_manager_round_trips_slashed_model_id() {
        // @step Given a provider manager has been configured via set_model_direct with provider "openai" and model id "accounts/fireworks/models/kimi-k2-06-instruct"
        let mut manager = test_manager(ProviderType::OpenAI);
        manager
            .set_model_direct(
                "openai",
                "accounts/fireworks/models/kimi-k2-06-instruct",
                Some(131_072),
                Some(4_096),
                None,
            )
            .expect("set_model_direct should succeed");

        // @step When AgentManager registers its spawn handler using selected_model_string
        let captured = manager.selected_model_string();

        // @step Then the captured model string is "openai/accounts/fireworks/models/kimi-k2-06-instruct"
        assert_eq!(
            captured.as_deref(),
            Some("openai/accounts/fireworks/models/kimi-k2-06-instruct"),
            "selected_model_string must include the provider prefix so \
             create_session_with_id parses it as provider='openai' + \
             model='accounts/fireworks/...' rather than mis-splitting at \
             the first '/' and treating 'accounts' as the provider name"
        );

        // @step And passing that captured model string to create_session_with_id resolves the provider as "openai"
        let captured = captured.expect("captured string is Some");
        let (resolved_provider, resolved_model) = captured
            .split_once('/')
            .expect("captured string contains at least one '/'");
        assert_eq!(resolved_provider, "openai");
        assert_eq!(
            resolved_model,
            "accounts/fireworks/models/kimi-k2-06-instruct"
        );

        // @step And no "Unknown provider: 'accounts'" error is raised
        // Implicit: if the BUG-136 regression returned — bare model id
        // stored — `split_once('/')` above would have produced
        // ("accounts", "fireworks/...") and `create_session_with_id` would
        // have tried to resolve "accounts" as a provider, producing the
        // original "Unknown provider: 'accounts'" failure.
        assert_ne!(
            resolved_provider, "accounts",
            "BUG-136 regression: model string mis-split and 'accounts' \
             would reach create_session_with_id as the provider name"
        );
    }

    /// Scenario: set_model_direct stores the full provider-prefixed model string
    ///
    /// Uses the `openai` provider slug because that is the code path
    /// Fireworks AI / OpenRouter-style configurations hit in practice
    /// (they're served via OpenAI-compatible endpoints and `set_model_direct`
    /// with `provider_id = "openai"`). The point of this test is the
    /// `selected_model_string()` / `selected_model_id()` round-trip, not
    /// custom-provider registration.
    #[test]
    fn test_bug136_set_model_direct_stores_provider_prefixed_model_string() {
        // @step Given a fresh provider manager
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called with provider "openai" and
        //        model id "llama-3.1-70b"
        manager
            .set_model_direct("openai", "llama-3.1-70b", None, None, None)
            .expect("set_model_direct should succeed");

        // @step Then selected_model_string returns "openai/llama-3.1-70b"
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("openai/llama-3.1-70b")
        );

        // @step And selected_model_id returns "llama-3.1-70b"
        assert_eq!(manager.selected_model_id().as_deref(), Some("llama-3.1-70b"));
    }

    /// Scenario: selected_model_id preserves slashes inside the model id
    ///
    /// Regression guard for the original Fireworks AI / OpenRouter failure:
    /// a model id that itself contains slashes must be handed verbatim to
    /// `get_openai()` / `get_claude()` etc., with only the single
    /// `{provider}/` prefix trimmed off.
    #[test]
    fn test_bug136_selected_model_id_preserves_internal_slashes() {
        // @step Given a fresh provider manager
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called with provider "openai" and
        //        model id "accounts/fireworks/models/kimi-k2-06-instruct"
        manager
            .set_model_direct(
                "openai",
                "accounts/fireworks/models/kimi-k2-06-instruct",
                None,
                None,
                None,
            )
            .expect("set_model_direct should succeed");

        // @step Then selected_model_string returns
        //        "openai/accounts/fireworks/models/kimi-k2-06-instruct"
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("openai/accounts/fireworks/models/kimi-k2-06-instruct")
        );

        // @step And selected_model_id returns
        //        "accounts/fireworks/models/kimi-k2-06-instruct" (unchanged)
        assert_eq!(
            manager.selected_model_id().as_deref(),
            Some("accounts/fireworks/models/kimi-k2-06-instruct")
        );
    }

    /// Scenario: Codex models continue to work after the fix
    #[test]
    fn test_bug136_codex_models_unaffected() {
        // @step Given a fresh provider manager
        let mut manager = test_manager(ProviderType::Codex);

        // @step When set_model_direct is called with provider "codex" and
        //        model id "gpt-5-codex"
        manager
            .set_model_direct("codex", "gpt-5-codex", None, None, None)
            .expect("set_model_direct should succeed");

        // @step Then selected_model_string returns "codex/gpt-5-codex"
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("codex/gpt-5-codex")
        );

        // @step And selected_model_id returns "gpt-5-codex"
        assert_eq!(manager.selected_model_id().as_deref(), Some("gpt-5-codex"));
    }

    /// Scenario: with_provider_and_model also emits a registry-format composite
    ///
    /// The compaction and DeepSearch paths re-hydrate a ProviderManager via
    /// `with_provider_and_model` using the parent's bare model id. This test
    /// confirms those managers also produce a registry-format string — any
    /// path that later feeds the value back into AgentManager /
    /// create_session_with_id must stay consistent with the `select_model`
    /// and `set_model_direct` formats.
    #[test]
    #[serial_test::serial]
    fn test_bug136_with_provider_and_model_emits_composite() {
        // @step Given valid "anthropic" credentials
        std::env::set_var("ANTHROPIC_API_KEY", "bug-136-test-key");

        // @step When with_provider_and_model is called with provider "claude" and model id "claude-opus-4-6"
        let manager = ProviderManager::with_provider_and_model(
            "claude",
            Some("claude-opus-4-6"),
            Some(200_000),
            Some(8_192),
        )
        .expect("with_provider_and_model should succeed with ANTHROPIC_API_KEY");

        // @step Then selected_model_string returns "claude/claude-opus-4-6"
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("claude/claude-opus-4-6")
        );

        // @step And selected_model_id returns "claude-opus-4-6"
        assert_eq!(
            manager.selected_model_id().as_deref(),
            Some("claude-opus-4-6")
        );
    }

    // =========================================================================
    // BUG-137: AgentManager spawn fails for profile-qualified OpenAI Fireworks
    // models (e.g. kimi k2.6). Fix: preserve profile name through
    // set_model_direct so selected_model_string() emits
    // "provider:profile/model".
    // Feature: spec/features/agentmanager-spawn-fails-for-profile-qualified-openai-fireworks-models-e-g-kimi-k2-6.feature
    // =========================================================================

    /// Scenario: set_model_direct with profile_name emits profile-qualified composite
    #[test]
    fn test_bug137_set_model_direct_with_profile_emits_profile_composite() {
        // @step Given a ProviderManager is created with model registry support
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When set_model_direct is called with provider_id "openai", model_id "accounts/fireworks/models/kimi-k2p6", and profile_name "fireworks"
        manager
            .set_model_direct_with_profile(
                "openai",
                "accounts/fireworks/models/kimi-k2p6",
                Some("fireworks"),
                Some(200_000),
                Some(16_384),
                None,
            )
            .expect("set_model_direct_with_profile should succeed");

        // @step Then selected_model_string() returns "openai:fireworks/accounts/fireworks/models/kimi-k2p6"
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("openai:fireworks/accounts/fireworks/models/kimi-k2p6"),
            "selected_model_string must include the profile segment so \
             create_session_with_id detects profile format (':' before '/') \
             and routes subordinate through set_model_direct instead of \
             select_model"
        );

        // @step And selected_model_id() returns "accounts/fireworks/models/kimi-k2p6"
        assert_eq!(
            manager.selected_model_id().as_deref(),
            Some("accounts/fireworks/models/kimi-k2p6")
        );
    }

    /// Scenario: set_model_direct without profile_name emits plain provider/model composite
    #[test]
    fn test_bug137_set_model_direct_without_profile_emits_plain_composite() {
        // @step Given a ProviderManager is created with model registry support
        let mut manager = test_manager(ProviderType::Codex);

        // @step When set_model_direct is called with provider_id "codex", model_id "gpt-5-codex", and no profile_name
        manager
            .set_model_direct_with_profile("codex", "gpt-5-codex", None, None, None, None)
            .expect("set_model_direct_with_profile should succeed");

        // @step Then selected_model_string() returns "codex/gpt-5-codex"
        let composite = manager.selected_model_string();
        assert_eq!(composite.as_deref(), Some("codex/gpt-5-codex"));

        // @step And the composite contains no colon
        assert!(
            !composite.as_ref().expect("composite").contains(':'),
            "composite must not contain ':' when no profile is set"
        );
    }

    /// Scenario: Legacy set_model_direct (no profile) emits plain composite
    /// Regression guard: backward-compatible 5-arg call path must behave as before.
    #[test]
    fn test_bug137_legacy_set_model_direct_is_plain_composite() {
        // @step Given a ProviderManager
        let mut manager = test_manager(ProviderType::OpenAI);

        // @step When legacy set_model_direct(5 args) is called
        manager
            .set_model_direct("openai", "accounts/fireworks/models/kimi-k2p6", None, None, None)
            .expect("legacy set_model_direct should succeed");

        // @step Then the composite has no profile segment (profile name defaults to None)
        assert_eq!(
            manager.selected_model_string().as_deref(),
            Some("openai/accounts/fireworks/models/kimi-k2p6")
        );
    }

    /// Scenario: select_model (cloud) does not inject a profile segment
    #[test]
    #[serial_test::serial]
    fn test_bug137_select_model_emits_no_profile_segment() {
        // @step Given a ProviderManager is created with model registry support
        std::env::set_var("ANTHROPIC_API_KEY", "bug-137-test-key");
        let mut manager = test_manager_with_registry_and_credentials();

        // @step When select_model is called with "anthropic/claude-sonnet-4"
        let result = manager.select_model("anthropic/claude-sonnet-4");
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(result.is_ok(), "select_model should succeed: {:?}", result.err());

        // @step Then selected_model_string() returns a composite without a colon before the first slash
        let composite = manager.selected_model_string().expect("composite");
        let first_slash = composite.find('/').expect("composite has '/'");
        let first_colon = composite.find(':');
        assert!(
            first_colon.is_none() || first_colon.unwrap() > first_slash,
            "cloud select_model must not inject 'provider:profile/' — got {composite:?}"
        );
    }

    /// Scenario: AgentManager spawn round-trips profile-qualified Fireworks model
    ///
    /// Exercises the round-trip that originally failed in the screenshot:
    /// spawner PM → selected_model_string() → create_session_with_id parser.
    /// The subordinate path checks `contains(':') && find(':') < find('/')`
    /// to detect profile models. This test verifies the captured string
    /// satisfies that invariant.
    #[test]
    fn test_bug137_agent_manager_round_trips_profile_qualified_model() {
        // @step Given a spawner session whose ProviderManager was configured via
        //   set_model_direct with profile_name "fireworks" on provider "openai"
        //   and model_id "accounts/fireworks/models/kimi-k2p6"
        let mut manager = test_manager(ProviderType::OpenAI);
        manager
            .set_model_direct_with_profile(
                "openai",
                "accounts/fireworks/models/kimi-k2p6",
                Some("fireworks"),
                Some(200_000),
                Some(16_384),
                None,
            )
            .expect("set_model_direct_with_profile should succeed");

        // @step When AgentManager captures selected_model_string() and passes it to create_session_with_id
        let captured = manager
            .selected_model_string()
            .expect("captured composite");

        // @step Then the subordinate path detects profile format by finding ':' before '/'
        let colon_idx = captured.find(':');
        let slash_idx = captured.find('/');
        assert!(
            colon_idx.is_some() && slash_idx.is_some(),
            "captured composite {captured:?} must contain both ':' and '/'"
        );
        assert!(
            colon_idx.unwrap() < slash_idx.unwrap(),
            "captured composite {captured:?} must have ':' before '/' so \
             create_session_with_id treats it as a profile model"
        );

        // @step And set_model_direct is used for the subordinate instead of select_model
        // Simulate create_session_with_id parsing:
        let is_profile_model = captured.contains(':')
            && captured.find(':') < captured.find('/');
        assert!(
            is_profile_model,
            "captured composite must be classified as profile model by \
             session_manager::create_session_with_id"
        );

        // @step And no "Model '...' not found in provider 'openai'" error is raised
        // (implicit — the profile branch skips registry validation)
        let colon = captured.find(':').unwrap();
        let slash = captured.find('/').unwrap();
        let provider = &captured[..colon];
        let profile = &captured[colon + 1..slash];
        let model = &captured[slash + 1..];
        assert_eq!(provider, "openai");
        assert_eq!(profile, "fireworks");
        assert_eq!(model, "accounts/fireworks/models/kimi-k2p6");
    }
}
