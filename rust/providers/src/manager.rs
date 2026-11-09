//! Provider Manager for dynamic provider selection
//!
//! Handles credential detection and provider instantiation based on:
//! - Available credentials (environment variables and auth files)
//! - CLI arguments (--provider and --model flags)
//! - Priority order: Claude API > Claude OAuth > Gemini > Codex > OpenAI
//!
//! MODEL-001: Integrates with ModelCache and ModelRegistry for dynamic model selection

use super::copilot::{CopilotDeploymentType, CopilotProvider};
use super::credentials::ProviderCredentials;
use super::models::{ModelCache, ModelInfo, ModelRegistry};
use super::{
    claude, copilot, gemini, zai, ClaudeProvider, CodexProvider, GeminiProvider, OpenAIProvider,
    ProviderError, ZAIProvider,
};
use crate::model_limits::{resolve_context_window, resolve_max_output_tokens, ModelLimitsResolver};
use std::str::FromStr;

#[path = "manager_runtime.rs"]
mod runtime;

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
            "claude" | "anthropic" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" | "google" => Ok(ProviderType::Gemini),
            "zai" | "z-ai" => Ok(ProviderType::ZAI),
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
    // RPC-066: consult the in-memory stub registry FIRST so the
    // cross-frontend parity test can register its stub provider via
    // `codelet_providers::stub_provider::register_stub_provider()`
    // without touching disk. Architecture note [J].
    #[cfg(feature = "test-support")]
    {
        if crate::stub_provider::is_stub_registered(slug) {
            return true;
        }
    }
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
    /// PROV-165: `None` when the provider declares no context window.
    default_ctx: Option<usize>,
    max_out: Option<usize>,
    /// PROV-165: `None` when the provider declares no output limit.
    default_out: Option<usize>,
}

impl ModelLimitsResolver for ConstantResolver {
    fn max_context_window(&self) -> Option<usize> {
        self.max_ctx
    }
    fn max_output_tokens_limit(&self) -> Option<usize> {
        self.max_out
    }
    fn default_context_window(&self) -> Option<usize> {
        self.default_ctx
    }
    fn default_max_output_tokens(&self) -> Option<usize> {
        self.default_out
    }
}

pub struct ProviderManager {
    runtime_options: crate::ProviderRuntimeOptions,
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
    /// PROV-171: OPENAI_* limit overrides captured when OpenAI is selected.
    pub(crate) openai_env_limits: crate::openai::OpenAIEnvLimits,
    /// MODEL-016: the selected profile model's declaration and live check.
    profile_reasoning: ProfileReasoning,
    /// MODEL-014: the Codex /models response fetched for this selection.
    codex_catalog: Option<Result<crate::codex::models_catalog::CodexCatalog, String>>,
    /// BUG-547: the context window the profile's engine reported, for the
    /// selection (`selected_model_string`) it was asked for. Kept across
    /// re-applying the same selection; ranks below every declared limit.
    engine_context_window: Option<(String, crate::engine_limits::EngineLimit)>,
}

include!("manager/debug.rs");
include!("manager/impl_1.rs");
include!("manager/impl_2.rs");
include!("manager/impl_3.rs");
include!("manager/impl_4.rs");
include!("manager/impl_5.rs");
include!("manager/profile_reasoning.rs");
include!("manager/codex_catalog.rs");
include!("manager/engine_limit.rs");
include!("manager/custom_reasoning.rs");

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    include!("manager/tests_1.rs");
    include!("manager/tests_2.rs");
    include!("manager/tests_3.rs");
    include!("manager/tests_4.rs");
    include!("manager/tests_5.rs");
    include!("manager/tests_6.rs");
    include!("manager/tests_7.rs");
}
