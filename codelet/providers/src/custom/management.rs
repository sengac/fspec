//! PROV-067: Custom provider management helpers.
//!
//! These helpers back the NAPI-exposed surface `list_providers`,
//! `show_provider`, `validate_provider`, `test_provider`, and
//! `init_provider` that the TypeScript TUI calls. Each function wraps a
//! deliberately Rust-only slice of logic so it can be unit-tested
//! without a live Node runtime.

use std::path::{Path, PathBuf};

use serde_json::json;

use super::config::{AuthConfig, ProviderConfig};
use super::discovery::discover_provider_configs;
use super::error::CustomProviderError;
use crate::credentials::ProviderCredentials;
use crate::error::ProviderError;

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
    /// [`ModelDef::context_window`].
    pub context_window: usize,
    /// Max output tokens per completion, sourced from
    /// [`ModelDef::max_output_tokens`].
    pub max_output_tokens: usize,
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

    for (name, available) in [
        ("claude", credentials.has_claude()),
        ("openai", credentials.has_openai()),
        ("gemini", credentials.has_gemini()),
        ("zai", credentials.has_zai()),
        ("codex", credentials.has_codex()),
        ("github-copilot", credentials.has_github_copilot()),
    ] {
        list.push(ProviderInfo {
            name: name.to_string(),
            display_name: Some(name.to_string()),
            available,
            is_custom: false,
            facade: None,
            base_url: None,
            api_key_env_var: None,
            models: Vec::new(),
            api_style: None,
        });
    }

    let customs = discover_provider_configs().map_err(CustomErrorExt::to_provider_error)?;
    for cfg in customs {
        let available = credentials.has_custom(&cfg.name);
        let env_var = effective_api_key_env_var(&cfg).map(ToString::to_string);
        // BUG-139: Surface per-model limits and supports_* flags from the
        // JSON `ModelDef` so downstream callers (NAPI -> TUI) do not have
        // to synthesise contextWindow=128000 / maxOutput=8192.
        let models: Vec<ProviderModelInfo> = cfg
            .models
            .iter()
            .map(|(alias, def)| ProviderModelInfo {
                id: alias.clone(),
                context_window: def.context_window,
                max_output_tokens: def.max_output_tokens,
                supports_tools: def.supports_tools,
                supports_streaming: def.supports_streaming,
                supports_thinking: def.supports_thinking,
                supports_vision: def.supports_vision,
            })
            .collect();
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

/// Validate a discovered custom provider's JSON config. Re-parses the
/// config from disk through [`ProviderConfig::from_file`] and performs
/// PROV-067-specific checks (facade OR script required; api_key_env_var
/// or auth must be present when facade is null).
pub fn validate_provider_config(name: &str) -> Result<(), ProviderError> {
    let config_path = find_provider_config_path(name)?;
    // PROV-067: An early structural scan so we can surface the missing
    // `facade` field before the stricter schema validator bails on
    // something else (e.g. a missing script).
    let raw = std::fs::read_to_string(&config_path).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to read {}: {}", config_path.display(), e),
        )
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("invalid JSON in {}: {}", config_path.display(), e),
        )
    })?;
    let has_facade = parsed
        .get("facade")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_script = parsed
        .get("script")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if !has_facade && !has_script {
        return Err(ProviderError::config(
            "manager",
            format!(
                "provider '{name}' is missing the required 'facade' field \
                 (or a 'script' path when facade is null)"
            ),
        ));
    }
    // Fall through to the stricter schema validator.
    let _ = ProviderConfig::from_file(&config_path).map_err(CustomErrorExt::to_provider_error)?;
    Ok(())
}

/// Probe the custom provider's `base_url` by issuing a `GET /v1/models`
/// and comparing the returned model IDs against the config's models.
pub async fn test_provider_connection(name: &str) -> Result<ProviderTestResult, ProviderError> {
    let config_path = find_provider_config_path(name)?;
    let cfg =
        ProviderConfig::from_file(&config_path).map_err(CustomErrorExt::to_provider_error)?;
    let url = format!("{}/models", cfg.base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| ProviderError::api("manager", format!("{url} unreachable: {e}")))?;
    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
    let remote_ids: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    entry
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    let matched_models: Vec<String> = cfg
        .models
        .values()
        .map(|m| m.id.clone())
        .filter(|id| remote_ids.contains(id))
        .collect();
    Ok(ProviderTestResult {
        reachable: (200..300).contains(&status),
        status_code: Some(status),
        matched_models,
    })
}

/// Create a new `.fspec/providers/<name>.json` under `project_root`
/// from a named template. Currently supported template: `"openai-compatible"`.
pub fn init_provider_template(
    project_root: &Path,
    name: &str,
    template: &str,
) -> Result<PathBuf, ProviderError> {
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || !name
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
    {
        return Err(ProviderError::config(
            "manager",
            format!("invalid provider slug '{name}'; must match ^[a-z][a-z0-9-]*$"),
        ));
    }
    let dir = project_root.join(".fspec").join("providers");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to create {}: {}", dir.display(), e),
        )
    })?;
    let body = match template {
        "openai-compatible" => openai_compatible_template(name),
        other => {
            return Err(ProviderError::config(
                "manager",
                format!("unknown template '{other}' (supported: openai-compatible)"),
            ));
        }
    };
    let path = dir.join(format!("{name}.json"));
    let serialized = serde_json::to_string_pretty(&body).map_err(|e| {
        ProviderError::config("manager", format!("failed to serialize template: {e}"))
    })?;
    std::fs::write(&path, serialized).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to write {}: {}", path.display(), e),
        )
    })?;
    Ok(path)
}

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
    let effective_facade = facade
        .or(cfg.facade.as_deref())
        .or(derived_facade);
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

/// Locate the JSON config file for `name` across user-global and
/// project-local search paths.
fn find_provider_config_path(name: &str) -> Result<PathBuf, ProviderError> {
    let project_dir = std::env::current_dir()
        .map(|cwd| cwd.join(".fspec").join("providers").join(format!("{name}.json")))
        .ok();
    if let Some(ref p) = project_dir {
        if p.is_file() {
            return Ok(p.clone());
        }
    }

    let home = std::env::var("FSPEC_HOME")
        .ok()
        .map(PathBuf::from)
        .and_then(|p| {
            if p.file_name().map(|n| n == "credentials").unwrap_or(false) {
                p.parent().map(Path::to_path_buf)
            } else {
                Some(p)
            }
        })
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".fspec")));
    if let Some(base) = home {
        let global = base.join("providers").join(format!("{name}.json"));
        if global.is_file() {
            return Ok(global);
        }
    }
    Err(ProviderError::config(
        "manager",
        format!("custom provider config for '{name}' not found"),
    ))
}

/// Build the JSON skeleton for the `openai-compatible` template.
fn openai_compatible_template(name: &str) -> serde_json::Value {
    let env_var = format!("{}_API_KEY", name.to_uppercase().replace('-', "_"));
    json!({
        "name": name,
        "display_name": format!("Custom {}", name),
        "facade": "openai",
        "base_url": "https://api.example.com/v1",
        "api_key_env_var": env_var,
        "models": {
            "default-model": {
                "id": "default-model",
                "context_window": 200000,
                "max_output_tokens": 4096
            }
        },
        "tool_style": "openai",
        "api_style": "openai_chat"
    })
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
