//! Facade derivation, facade env vars and model-id resolution for custom providers (split from management.rs by REFAC-030).

use super::{effective_api_key_env_var, CustomErrorExt};
use crate::custom::discovery::discover_provider_configs;
use crate::error::ProviderError;

/// PROV-067: Set the env vars a facade provider expects so its existing
/// match arm in the agent-loop dispatcher reads them transparently.
///
/// Handles:
/// - `openai` facade (OPENAI_BASE_URL, OPENAI_API_KEY, OPENAI_MODEL)
/// - `claude` facade (ANTHROPIC_API_KEY)
///
/// When no explicit facade is set, derives one from `api_style`:
/// - `anthropic_messages` → `"claude"`
/// - `openai_chat` → `"openai"`
pub fn apply_custom_provider_env_vars(
    name: &str,
    model_id: &str,
    facade: Option<&str>,
) -> Result<(), ProviderError> {
    let configs = discover_provider_configs().map_err(CustomErrorExt::to_provider_error)?;
    let cfg = configs
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| ProviderError::config("manager", format!("Provider '{name}' not found")))?;

    let derived_facade = match cfg.api_style {
        crate::custom::ApiStyle::AnthropicMessages => Some("claude"),
        crate::custom::ApiStyle::OpenaiChat => Some("openai"),
    };
    let effective_facade = facade.or(cfg.facade.as_deref()).or(derived_facade);
    match effective_facade {
        Some("openai") => {
            std::env::set_var("OPENAI_BASE_URL", &cfg.base_url);
            std::env::set_var("OPENAI_MODEL", model_id);
            if let Some(env_var) = effective_api_key_env_var(&cfg) {
                if let Ok(v) = std::env::var(env_var) {
                    if !v.is_empty() {
                        std::env::set_var("OPENAI_API_KEY", v);
                    }
                }
            }
            Ok(())
        }
        Some("claude") => {
            // Set ANTHROPIC_API_KEY so the built-in claude arm picks it up.
            if let Some(env_var) = effective_api_key_env_var(&cfg) {
                if let Ok(v) = std::env::var(env_var) {
                    if !v.is_empty() {
                        std::env::set_var("ANTHROPIC_API_KEY", v);
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Derive the effective facade for a custom provider from its config.
///
/// Priority:
/// 1. Explicit `facade` field → use that facade.
/// 2. No explicit facade AND a Rhai `script` is present → return `None` so
///    the agent-loop dispatches to the custom-provider fallback arm
///    (PROV-092's `CustomProvider::create_rig_agent` / Rhai-native path).
/// 3. No facade AND no script → derive a built-in facade from `api_style`.
///
/// PROV-095: Previously this function unconditionally derived a facade
/// from `api_style` even when a Rhai `script` was present, which
/// short-circuited dispatch to the built-in provider (e.g. `get_claude()`)
/// and raised `Current provider is not Claude` for Rhai-scripted
/// providers like `claude-rhai`.
pub fn derive_facade_for_custom(name: &str) -> Option<String> {
    let configs = discover_provider_configs().ok()?;
    let cfg = configs.into_iter().find(|c| c.name == name)?;

    if let Some(ref f) = cfg.facade {
        return Some(f.clone());
    }
    // PROV-095: A non-empty `script` means the provider runs through the
    // Rhai-native dispatch path — do NOT derive a built-in facade here,
    // otherwise the agent loop short-circuits to the wrong arm.
    if !cfg.script.trim().is_empty() {
        return None;
    }
    match cfg.api_style {
        crate::custom::ApiStyle::AnthropicMessages => Some("claude".to_string()),
        crate::custom::ApiStyle::OpenaiChat => Some("openai".to_string()),
    }
}

/// PROV-100: Resolve a custom-provider model alias (e.g. `"opus-4.7"`,
/// the HashMap key in `ProviderConfig.models`) to the underlying model
/// identifier the provider API expects (e.g. `"claude-opus-4-7"`, the
/// `ModelDef.id` field).
///
/// The session_manager's thinking-config routing is keyed on the real
/// model id — `is_adaptive_thinking_model("claude-opus-4-7")` must be
/// true for Opus 4.7 to receive adaptive thinking. Without this
/// resolution, the alias "opus-4.7" hits none of the Claude/Gemini/
/// Codex provider heuristics and the session falls through to the
/// empty-config branch, meaning thinking is never wired up for any
/// Rhai-scripted provider.
///
/// Returns `None` when the provider name does not match any discovered
/// custom provider or when the alias is not a declared model key. The
/// caller should then fall back to the raw alias.
pub fn resolve_custom_model_id(provider_name: &str, model_alias: &str) -> Option<String> {
    let configs = discover_provider_configs().ok()?;
    let cfg = configs.into_iter().find(|c| c.name == provider_name)?;
    cfg.models.get(model_alias).map(|m| m.id.clone())
}
