//! ProviderConfig JSON schema (PROV-062).
//!
//! Deserializes the custom provider JSON configuration and runs load-time
//! validation: name pattern, script existence, and default-model
//! cross-reference.
//!
//! PROV-085: The built-in-name collision guard has been removed so that
//! custom Rhai provider configs may shadow built-in providers (e.g. ship
//! `claude-code.rhai` as the default Claude path). Precedence is resolved
//! at the manager layer via `custom_provider_registered`, gated by the
//! `FSPEC_DISABLE_SCRIPT_SHADOWING` escape hatch. The
//! [`CustomProviderError::NameConflict`] variant is retained in the
//! public API for binary compatibility; it is no longer produced by the
//! default code path.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::error::CustomProviderError;
use super::tool_facade::RhaiToolDef;

mod auth;
mod styles;

pub use auth::AuthConfig;
pub use styles::{ApiStyle, ToolStyle};

/// Allowed pattern for the `name` field of a custom provider config.
const NAME_PATTERN: &str = "^[a-z][a-z0-9-]*$";

fn default_true() -> bool {
    true
}

fn default_tool_style() -> ToolStyle {
    ToolStyle::Claude
}

fn default_api_style() -> ApiStyle {
    ApiStyle::OpenaiChat
}

/// PROV-067: Default [`AuthConfig`] for custom providers that omit the
/// `auth` block entirely. The custom provider manager layer consults
/// [`ProviderConfig::api_key_env_var`] or delegates to the facade
/// provider, so no credentials are needed at config-load time.
fn default_auth() -> AuthConfig {
    AuthConfig::Custom {
        credential_file: None,
    }
}

/// Per-model definition in a [`ProviderConfig::models`] map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDef {
    /// Model identifier sent to the API.
    pub id: String,
    /// Maximum context window in tokens. PROV-170: `None` when the config
    /// declares none — the limit is unavailable, never guessed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<usize>,
    /// Maximum output tokens per completion. PROV-170: `None` when the
    /// config declares none (see [`Defaults::max_output_tokens`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    /// Whether the model supports tool / function calling.
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    /// Whether the model supports SSE streaming.
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    /// Whether the model supports extended-thinking mode.
    #[serde(default)]
    pub supports_thinking: bool,
    /// Whether the model supports vision / image input.
    #[serde(default)]
    pub supports_vision: bool,
}

/// Default request parameters when the caller does not specify them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Defaults {
    /// Default model alias (must be a key of [`ProviderConfig::models`]).
    #[serde(default)]
    pub model: Option<String>,
    /// Default sampling temperature.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Default max output tokens (overridden by model-specific value).
    #[serde(default)]
    pub max_output_tokens: Option<usize>,
}

/// System-prompt-level configuration (prefix, cache_control).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptConfig {
    /// Text prepended to every system prompt.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Whether to use Anthropic-style cache_control metadata.
    #[serde(default)]
    pub cache_control: bool,
}

/// Full provider configuration loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider identifier (used as the `--provider` CLI flag value).
    pub name: String,
    /// Human-readable provider name.
    pub display_name: String,
    /// Base API URL.
    pub base_url: String,
    /// PROV-067: Optional facade provider name (e.g. `"openai"`, `"claude"`,
    /// `"codex"`). When `Some`, the agent-loop dispatch routes through the
    /// facade provider's match arm via [`crate::ProviderManager::facade_override`]
    /// instead of invoking the custom provider's own `create_rig_agent`.
    /// When `None`, the custom provider uses the generic Rhai-backed agent
    /// construction path (PROV-063/065/066).
    #[serde(default)]
    pub facade: Option<String>,
    /// PROV-067: Optional convenience field duplicating the env var name
    /// from [`AuthConfig::Bearer`] / [`AuthConfig::ApiKeyHeader`]. Used by
    /// the PROV-067 manager-integration layer to discover credential
    /// availability at detection time without unpacking `auth`.
    #[serde(default)]
    pub api_key_env_var: Option<String>,
    /// Path to the `.rhai` script, relative to the config file's directory.
    /// Optional for custom providers that delegate to a facade — the Rhai
    /// runtime is only needed when `facade` is `None`.
    #[serde(default)]
    pub script: String,
    /// Authentication configuration.
    /// Optional for custom providers that delegate fully to a facade; when
    /// omitted the facade provider's own credential resolution path is used.
    #[serde(default = "default_auth")]
    pub auth: AuthConfig,
    /// Map of model aliases to model definitions.
    pub models: HashMap<String, ModelDef>,
    /// Optional request-level defaults.
    #[serde(default)]
    pub defaults: Option<Defaults>,
    /// Optional system-prompt-level configuration.
    #[serde(default)]
    pub system_prompt: Option<SystemPromptConfig>,
    /// Tool-calling convention.
    #[serde(default = "default_tool_style")]
    pub tool_style: ToolStyle,
    /// API request/response shape.
    #[serde(default = "default_api_style")]
    pub api_style: ApiStyle,
    /// Additional static HTTP headers to include on every request.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Environment variable prefix used for credential detection.
    #[serde(default)]
    pub env_prefix: Option<String>,
    /// PROV-066: cached resolved Rhai tool definitions. Populated by
    /// [`crate::custom::tool_resolve::resolve_tools`] so downstream
    /// consumers (system-prompt rendering, request builders) can
    /// introspect without re-running the Rhai `define_tools` script.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resolved_tools: Option<Vec<RhaiToolDef>>,
}

impl ProviderConfig {
    /// Load and validate a provider config from a JSON file on disk.
    ///
    /// Runs all load-time validation: name pattern,
    /// script existence (resolved relative to the config file's directory),
    /// and default-model cross-reference.
    pub fn from_file(path: &Path) -> Result<Self, CustomProviderError> {
        let content = std::fs::read_to_string(path).map_err(|source| CustomProviderError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let cfg: ProviderConfig =
            serde_json::from_str(&content).map_err(|e| CustomProviderError::Parse {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
        cfg.validate(path)?;
        Ok(cfg)
    }

    /// Run all load-time validation for a config that was deserialized
    /// from `path`.
    pub fn validate(&self, path: &Path) -> Result<(), CustomProviderError> {
        // Name pattern. `NAME_PATTERN` is a compile-time constant so
        // constructing the regex cannot fail at runtime; the unwrap is
        // unreachable but avoids `expect()` flagged by clippy.
        let re = match Regex::new(NAME_PATTERN) {
            Ok(re) => re,
            Err(e) => {
                return Err(CustomProviderError::Parse {
                    path: path.to_path_buf(),
                    message: format!("internal: name pattern regex failed: {e}"),
                });
            }
        };
        if !re.is_match(&self.name) {
            return Err(CustomProviderError::InvalidName {
                name: self.name.clone(),
                path: path.to_path_buf(),
            });
        }

        // PROV-085: The BUILTIN_PROVIDER_NAMES collision guard has been
        // removed. Custom configs are now permitted to shadow built-in
        // provider slugs (`claude`, `codex`, `openai`, `gemini`, `zai`,
        // `copilot`, `github-copilot`). Precedence resolution lives in
        // `manager::custom_provider_registered` where the escape-hatch
        // env var `FSPEC_DISABLE_SCRIPT_SHADOWING` can opt back in to
        // the hardcoded built-in path. The `NameConflict` variant is
        // retained in `CustomProviderError` for public-API stability.

        // Script existence (relative to the config file's directory).
        // PROV-067: when a facade is configured, the custom provider
        // delegates fully to a built-in provider and does not require a
        // Rhai script — so only validate script existence when `facade`
        // is `None` and a non-empty `script` path has been provided.
        if self.facade.is_none() && !self.script.is_empty() {
            let config_dir: &Path = path.parent().unwrap_or_else(|| Path::new("."));
            let resolved_script: PathBuf = config_dir.join(&self.script);
            if !resolved_script.exists() {
                return Err(CustomProviderError::ScriptNotFound {
                    resolved_path: resolved_script,
                    config_path: path.to_path_buf(),
                });
            }
        }

        // PROV-170: a declared limit of 0 is not a limit — reject it here,
        // naming the field, instead of letting it reach the model selector.
        self.validate_declared_limits(path)?;

        // Default model must exist.
        if let Some(ref defaults) = self.defaults {
            if let Some(ref default_model) = defaults.model {
                if !self.models.contains_key(default_model) {
                    return Err(CustomProviderError::MissingDefaultModel {
                        provider: self.name.clone(),
                        model: default_model.clone(),
                        path: path.to_path_buf(),
                    });
                }
            }
        }

        Ok(())
    }

    /// PROV-170: every declared token limit must be positive.
    fn validate_declared_limits(&self, path: &Path) -> Result<(), CustomProviderError> {
        let mut models: Vec<(&String, &ModelDef)> = self.models.iter().collect();
        models.sort_by_key(|(alias, _)| *alias);
        let model_limits = models.into_iter().flat_map(|(alias, def)| {
            [
                (format!("models.{alias}.context_window"), def.context_window),
                (
                    format!("models.{alias}.max_output_tokens"),
                    def.max_output_tokens,
                ),
            ]
        });
        let default_limit = self.defaults.as_ref().map(|defaults| {
            (
                "defaults.max_output_tokens".to_string(),
                defaults.max_output_tokens,
            )
        });
        match model_limits
            .chain(default_limit)
            .find(|(_, limit)| *limit == Some(0))
        {
            Some((field, _)) => Err(CustomProviderError::NonPositiveLimit {
                provider: self.name.clone(),
                field,
                path: path.to_path_buf(),
            }),
            None => Ok(()),
        }
    }
}
