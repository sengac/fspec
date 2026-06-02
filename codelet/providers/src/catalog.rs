//! RPC-107: Canonical 17-provider ordered registry.
//!
//! Mirrors `src/utils/provider-registry.ts` (SUPPORTED_PROVIDERS L18-36 +
//! PROVIDER_REGISTRY L43-217) bytes-for-bytes so the Rust ratatui
//! `ProviderSettingsView` shows the SAME 17 rows in the SAME ORDER with
//! the SAME DISPLAY NAMES as the TypeScript Ink reference.
//!
//! This slice is the single source of truth for canonical provider
//! display + ordering. `codelet_providers::custom::list_providers_info`
//! iterates this slice first (canonical order preserved), then appends
//! any custom providers discovered via
//! `codelet_providers::custom::discover_provider_configs`.

/// Authentication style for a canonical provider — mirrors the TS
/// `PROVIDER_REGISTRY[id].authType` discriminant ('api-key' | 'oauth').
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// Uses an env-var-backed API key (e.g. OPENAI_API_KEY).
    ApiKey,
    /// Uses an OAuth flow (anthropic, codex, github-copilot).
    OAuth,
}

/// Static metadata for one canonical provider. All `&'static str` so the
/// slice can live as a `const` with zero runtime allocation.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalProvider {
    /// Stable TS-canonical slug (e.g. "openai", "anthropic",
    /// "github-copilot"). Used as `ProviderCredentialInfo.provider_id`.
    pub id: &'static str,
    /// Human-readable display name from TS PROVIDER_REGISTRY (e.g.
    /// "OpenAI API", "Google Gemini"). Used as
    /// `ProviderCredentialInfo.display_name`.
    pub display_name: &'static str,
    /// Primary env var that, when set, marks the provider configured.
    /// Empty string for OAuth-only providers whose readiness is
    /// determined by an auth file rather than an env var.
    pub env_var: &'static str,
    /// API-key vs OAuth — drives the right-pane editor selection.
    pub auth_type: AuthType,
    /// Default base URL for the provider's REST API. `None` for
    /// providers whose endpoint is resolved dynamically (e.g. Azure
    /// OpenAI per-deployment hosts).
    pub default_base_url: Option<&'static str>,
}

/// Canonical 17-provider ordered registry. Order matches TS
/// `SUPPORTED_PROVIDERS` at `src/utils/provider-registry.ts:18-36`.
pub const CANONICAL_PROVIDERS: &[CanonicalProvider] = &[
    CanonicalProvider {
        id: "openai",
        display_name: "OpenAI API",
        env_var: "OPENAI_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.openai.com/v1"),
    },
    CanonicalProvider {
        id: "anthropic",
        display_name: "Anthropic",
        env_var: "ANTHROPIC_API_KEY",
        auth_type: AuthType::OAuth,
        default_base_url: Some("https://api.anthropic.com/v1"),
    },
    CanonicalProvider {
        id: "cohere",
        display_name: "Cohere",
        env_var: "COHERE_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.cohere.com/v1"),
    },
    CanonicalProvider {
        id: "gemini",
        display_name: "Google Gemini",
        env_var: "GOOGLE_GENERATIVE_AI_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://generativelanguage.googleapis.com/v1beta"),
    },
    CanonicalProvider {
        id: "mistral",
        display_name: "Mistral AI",
        env_var: "MISTRAL_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.mistral.ai/v1"),
    },
    CanonicalProvider {
        id: "xai",
        display_name: "xAI",
        env_var: "XAI_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.x.ai/v1"),
    },
    CanonicalProvider {
        id: "together",
        display_name: "Together AI",
        env_var: "TOGETHER_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.together.xyz/v1"),
    },
    CanonicalProvider {
        id: "huggingface",
        display_name: "Hugging Face",
        env_var: "HF_TOKEN",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api-inference.huggingface.co"),
    },
    CanonicalProvider {
        id: "openrouter",
        display_name: "OpenRouter",
        env_var: "OPENROUTER_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://openrouter.ai/api/v1"),
    },
    CanonicalProvider {
        id: "groq",
        display_name: "Groq",
        env_var: "GROQ_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.groq.com/openai/v1"),
    },
    CanonicalProvider {
        id: "deepseek",
        display_name: "DeepSeek",
        env_var: "DEEPSEEK_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.deepseek.com/v1"),
    },
    CanonicalProvider {
        id: "moonshot",
        display_name: "Moonshot",
        env_var: "MOONSHOT_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.moonshot.cn/v1"),
    },
    CanonicalProvider {
        id: "galadriel",
        display_name: "Galadriel",
        env_var: "GALADRIEL_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.galadriel.com/v1"),
    },
    CanonicalProvider {
        id: "azure",
        display_name: "Azure OpenAI",
        env_var: "AZURE_OPENAI_API_KEY",
        auth_type: AuthType::ApiKey,
        // Azure base URLs are per-deployment; resolved at call time.
        default_base_url: None,
    },
    CanonicalProvider {
        id: "zai",
        display_name: "Z.AI",
        env_var: "ZAI_API_KEY",
        auth_type: AuthType::ApiKey,
        default_base_url: Some("https://api.z.ai/v1"),
    },
    CanonicalProvider {
        id: "codex",
        display_name: "Codex (ChatGPT)",
        // Codex is OAuth-only; readiness comes from ~/.codex/auth.json.
        env_var: "",
        auth_type: AuthType::OAuth,
        default_base_url: Some("https://chatgpt.com/backend-api/codex"),
    },
    CanonicalProvider {
        id: "github-copilot",
        display_name: "GitHub Copilot",
        // GitHub Copilot is OAuth-device-flow; readiness comes from
        // ~/.fspec/credentials/copilot_auth.json.
        env_var: "",
        auth_type: AuthType::OAuth,
        default_base_url: None,
    },
];
