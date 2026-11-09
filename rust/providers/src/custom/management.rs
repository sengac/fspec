//! PROV-067: Custom provider management helpers.
//!
//! These helpers back the NAPI-exposed surface `list_providers`,
//! `show_provider`, `validate_provider`, `test_provider`, and
//! `init_provider` that the TypeScript TUI calls. Each function wraps a
//! deliberately Rust-only slice of logic so it can be unit-tested
//! without a live Node runtime.

use super::config::{AuthConfig, ProviderConfig};
use super::discovery::discover_provider_configs;
use super::error::CustomProviderError;
use crate::credentials::ProviderCredentials;
use crate::error::ProviderError;

mod facade_env;
mod provider_files;
mod wire_limit;

use wire_limit::wire_limit;

pub use facade_env::{
    apply_custom_provider_env_vars, derive_facade_for_custom, resolve_custom_model_id,
};
pub use provider_files::{
    init_provider_template, test_provider_connection, validate_provider_config,
};

/// BUG-139: Per-model info entry exposed through [`ProviderInfo::models`].
///
/// Prior to BUG-139 `ProviderInfo.models` was `Vec<String>` (just the
/// alias keys of [`ProviderConfig::models`]), which forced the TUI's
/// `customProviderSectionBuilder` to synthesise hardcoded
/// `contextWindow=128000` / `maxOutput=8192` values. That lost any
/// per-model overrides declared in the provider's JSON config (e.g. a
/// 1M context window on `opus-4.7`) and made the SessionHeader badge
/// display stale `[120k]` math.
///
/// The widened shape surfaces the per-model limits and capability flags
/// from [`crate::custom::config::ModelDef`] directly, so the TUI can
/// populate `NapiModelInfo.contextWindow` / `.maxOutput` / `.toolCall`
/// / `.reasoning` from authoritative config values.
#[derive(Debug, Clone)]
pub struct ProviderModelInfo {
    /// Model alias key from [`ProviderConfig::models`] (e.g. `"opus-4.7"`).
    pub id: String,
    /// Context window in tokens, sourced from
    /// [`ModelDef::context_window`]. PROV-170: `None` when undeclared;
    /// PROV-172: `u32`, the RPC wire width (a larger value fails the listing).
    pub context_window: Option<u32>,
    /// Max output tokens per completion, sourced from
    /// [`ModelDef::max_output_tokens`]. PROV-170: `None` when undeclared.
    pub max_output_tokens: Option<u32>,
    /// Whether the model supports tool / function calling.
    pub supports_tools: bool,
    /// Whether the model supports SSE streaming.
    pub supports_streaming: bool,
    /// Whether the model supports extended-thinking mode.
    pub supports_thinking: bool,
    /// Whether the model supports vision / image input.
    pub supports_vision: bool,
}

/// Lightweight info entry returned by [`list_providers_info`] and
/// [`show_provider_info`]. Mirrors the shape the TUI renders in its
/// provider picker.
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    /// Provider slug (e.g. `"claude"`, `"my-llm"`).
    pub name: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// Whether credentials are present for this provider.
    pub available: bool,
    /// `true` for discovered custom providers, `false` for built-ins.
    pub is_custom: bool,
    /// Facade provider for custom entries (e.g. `Some("openai")`).
    pub facade: Option<String>,
    /// Base URL for custom entries.
    pub base_url: Option<String>,
    /// Env var name for API key (custom entries).
    pub api_key_env_var: Option<String>,
    /// BUG-139: Per-model info (id + limits + supports_* flags) declared
    /// in the custom provider config. Empty for built-in providers.
    pub models: Vec<ProviderModelInfo>,
    /// API style for facade derivation (custom entries).
    pub api_style: Option<String>,
    /// RPC-108 / PROV-099: Display-safe masked API key for the
    /// configured row, e.g. `Some("sk-ant-••••••••mnop")`. Populated
    /// server-side via [`crate::credentials::mask_api_key`] whenever the
    /// provider declares a non-empty `env_var` that is actually set —
    /// independent of [`crate::catalog::AuthType`]. This includes
    /// anthropic (env_var=ANTHROPIC_API_KEY but auth_type=OAuth), whose
    /// env api key is masked here. `None` for unconfigured providers and
    /// for OAuth-only providers that declare an empty `env_var` (codex,
    /// github-copilot) — the TUI view layer substitutes the literal
    /// `'OAuth'` string and no OAuth token bytes traverse this surface.
    pub masked_key: Option<String>,
    /// RPC-108: Provenance tag — one of
    /// [`crate::credentials::SOURCE_EXPLICIT`],
    /// [`crate::credentials::SOURCE_FILE`],
    /// [`crate::credentials::SOURCE_ENV`],
    /// [`crate::credentials::SOURCE_DOTENV`]. Mirrors TS
    /// `ProviderConfigResult.source` at `src/utils/credentials.ts:56-59`.
    /// `None` when the provider is unconfigured.
    pub source: Option<String>,
}

/// Result of [`test_provider_connection`]. `matched_models` lists which
/// of the config-declared model IDs also appeared in the remote
/// `/v1/models` response, so the caller can surface any drift.
#[derive(Debug, Clone)]
pub struct ProviderTestResult {
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub matched_models: Vec<String>,
}

/// List all providers (built-in + custom) along with their availability.
///
/// Built-ins always appear (claude, openai, gemini, zai, codex,
/// github-copilot); each entry's `available` reflects
/// [`ProviderCredentials::detect`]. Custom entries come from
/// [`discover_provider_configs`] with their env var probed.
pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError> {
    let credentials = ProviderCredentials::detect();
    let mut list: Vec<ProviderInfo> = Vec::new();

    // RPC-107: iterate the canonical 17-provider ordered registry so the
    // Rust ratatui `ProviderSettingsView` matches the TS Ink reference
    // byte-for-byte on order + display names. Pre-RPC-107 this loop
    // hard-coded 6 entries using the legacy slug `claude`; the canonical
    // surface uses `anthropic`.
    for entry in crate::catalog::CANONICAL_PROVIDERS {
        let available = match entry.id {
            "openai" => credentials.has_openai(),
            "anthropic" => credentials.has_claude(),
            "gemini" => credentials.has_gemini(),
            "zai" => credentials.has_zai(),
            "codex" => credentials.has_codex(),
            "github-copilot" => credentials.has_github_copilot(),
            // Net-new canonical providers (cohere, mistral, xai,
            // together, huggingface, openrouter, groq, deepseek,
            // moonshot, galadriel, azure) probe their declared env var
            // directly. OAuth-only entries declare an empty env_var and
            // therefore remain unconfigured until a sibling card wires
            // their auth-file probes.
            _ => {
                !entry.env_var.is_empty()
                    && std::env::var(entry.env_var)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
            }
        };
        // PROV-099: derive masked_key + source for ANY provider whose
        // declared env_var is actually set to a non-empty value,
        // independent of AuthType. Pre-PROV-099 this matched only
        // `AuthType::ApiKey`, so anthropic (env_var=ANTHROPIC_API_KEY but
        // auth_type=OAuth) always fell to (None, None) even when the env
        // key was present. OAuth-only providers (codex, github-copilot)
        // declare an empty env_var, so they still carry (None, None) and
        // no OAuth token bytes ever traverse this surface.
        let (masked_key, source) = if available && !entry.env_var.is_empty() {
            match std::env::var(entry.env_var) {
                Ok(raw) if !raw.is_empty() => (
                    Some(crate::credentials::mask_api_key(&raw)),
                    Some(crate::credentials::SOURCE_ENV.to_string()),
                ),
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        list.push(ProviderInfo {
            name: entry.id.to_string(),
            display_name: Some(entry.display_name.to_string()),
            available,
            is_custom: false,
            facade: None,
            base_url: entry.default_base_url.map(ToString::to_string),
            api_key_env_var: if entry.env_var.is_empty() {
                None
            } else {
                Some(entry.env_var.to_string())
            },
            models: Vec::new(),
            api_style: None,
            masked_key,
            source,
        });
    }

    let customs = discover_provider_configs().map_err(CustomErrorExt::to_provider_error)?;
    for cfg in customs {
        let available = credentials.has_custom(&cfg.name);
        let env_var = effective_api_key_env_var(&cfg).map(ToString::to_string);
        // BUG-139: Surface per-model limits and supports_* flags from the
        // JSON `ModelDef` so downstream callers (NAPI -> TUI) do not have
        // to synthesise contextWindow=128000 / maxOutput=8192.
        let models = cfg
            .models
            .iter()
            .map(|(alias, def)| {
                Ok(ProviderModelInfo {
                    id: alias.clone(),
                    context_window: wire_limit(
                        &cfg.name,
                        alias,
                        "context_window",
                        def.context_window,
                    )?,
                    max_output_tokens: wire_limit(
                        &cfg.name,
                        alias,
                        "max_output_tokens",
                        def.max_output_tokens,
                    )?,
                    supports_tools: def.supports_tools,
                    supports_streaming: def.supports_streaming,
                    supports_thinking: def.supports_thinking,
                    supports_vision: def.supports_vision,
                })
            })
            .collect::<Result<Vec<ProviderModelInfo>, CustomProviderError>>()
            .map_err(CustomErrorExt::to_provider_error)?;
        let api_style_str = match cfg.api_style {
            crate::custom::ApiStyle::AnthropicMessages => "anthropic_messages",
            crate::custom::ApiStyle::OpenaiChat => "openai_chat",
        };
        list.push(ProviderInfo {
            name: cfg.name.clone(),
            display_name: Some(cfg.display_name.clone()),
            available,
            is_custom: true,
            facade: cfg.facade.clone(),
            base_url: Some(cfg.base_url.clone()),
            api_key_env_var: env_var,
            models,
            api_style: Some(api_style_str.to_string()),
            // RPC-108: custom providers don't carry a masked-key on
            // this surface — `source: 'config'` tagging is PROV-067
            // scope tracked separately. Leaving both fields None keeps
            // the wire shape stable.
            masked_key: None,
            source: None,
        });
    }

    Ok(list)
}

/// Return the full [`ProviderInfo`] for a single custom provider slug.
/// Built-in providers are *not* accepted here — use [`list_providers_info`]
/// for those.
pub fn show_provider_info(name: &str) -> Result<ProviderInfo, ProviderError> {
    let list = list_providers_info()?;
    list.into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| ProviderError::config("manager", format!("Provider '{name}' not found")))
}

/// Return the effective API-key env var for `cfg`: the `api_key_env_var`
/// field wins, falling back to the `env_var` inside
/// [`AuthConfig::Bearer`] / [`AuthConfig::ApiKeyHeader`].
fn effective_api_key_env_var(cfg: &ProviderConfig) -> Option<&str> {
    if let Some(ref v) = cfg.api_key_env_var {
        return Some(v.as_str());
    }
    match &cfg.auth {
        AuthConfig::Bearer { env_var, .. } | AuthConfig::ApiKeyHeader { env_var, .. } => {
            Some(env_var.as_str())
        }
        _ => None,
    }
}

/// Tiny helper so [`CustomProviderError`] can be routed into
/// [`ProviderError`] without the module reaching into
/// `crate::custom::error_mapping` internals.
trait CustomErrorExt {
    fn to_provider_error(self) -> ProviderError;
}

impl CustomErrorExt for CustomProviderError {
    fn to_provider_error(self) -> ProviderError {
        ProviderError::from(self)
    }
}
