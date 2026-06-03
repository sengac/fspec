//! Provider credential detection module
//!
//! Detects available LLM provider credentials from:
//! - Environment variables (ANTHROPIC_API_KEY, CLAUDE_CODE_OAUTH_TOKEN, OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY, ZAI_API_KEY, ZAI_PLAN_API_KEY)
//! - Auth files (~/.codex/auth.json for Codex OAuth, ~/.fspec/credentials/claude_auth.json for Claude OAuth, ~/.fspec/credentials/copilot_auth.json for GitHub Copilot OAuth)
//! - PROV-067: Custom provider definitions discovered via `crate::custom::discover_provider_configs`
//!
//! RPC-108: This module also exposes the prefix-aware credential masking
//! helper [`mask_api_key`] and the four provenance-tag constants
//! ([`SOURCE_EXPLICIT`], [`SOURCE_FILE`], [`SOURCE_ENV`], [`SOURCE_DOTENV`])
//! that mirror the TS `maskApiKey` / `ProviderConfigResult.source` contract
//! at `src/utils/credentials.ts:56-59,277-291`.

use std::collections::HashMap;

/// Provenance tag for caller-supplied credentials (rare; mostly tests).
/// Mirrors TS `ProviderConfigResult.source = 'explicit'`.
pub const SOURCE_EXPLICIT: &str = "explicit";

/// Provenance tag for credentials loaded from
/// `~/.fspec/credentials/credentials.json`.
/// Mirrors TS `ProviderConfigResult.source = 'file'`
/// (`src/utils/credentials.ts:232`).
pub const SOURCE_FILE: &str = "file";

/// Provenance tag for credentials read from `process.env` /
/// `std::env::var`. Mirrors TS `ProviderConfigResult.source = 'env'`
/// (`src/utils/credentials.ts:243`).
pub const SOURCE_ENV: &str = "env";

/// Provenance tag for credentials parsed from a `.env` file in the
/// current working directory. Mirrors TS
/// `ProviderConfigResult.source = 'dotenv'`
/// (`src/utils/credentials.ts:260`).
pub const SOURCE_DOTENV: &str = "dotenv";

/// RPC-108: Mask an API key for safe display.
///
/// Ports the TypeScript helper at `src/utils/credentials.ts:277-291`
/// byte-for-byte:
///
/// - Keys shorter than 12 chars return the literal `••••••••`
///   (8 bullet points, U+2022) — no prefix or suffix is exposed.
/// - Otherwise the helper greedy-matches one of five recognised prefixes
///   in declaration order — `sk-ant-`, `sk-`, `gsk_`, `AIza`, `xai-`.
///   Order matters: `sk-ant-` MUST be tested before `sk-` because both
///   would match an Anthropic key. The matched prefix is preserved
///   verbatim; if none match, the first 6 chars are used as the prefix
///   instead.
/// - Always appends `••••••••` followed by the last 4 chars of the key.
///
/// Examples:
/// ```
/// use codelet_providers::credentials::mask_api_key;
/// assert_eq!(mask_api_key("sk-ant-api03-abcdefghijklmnop"), "sk-ant-••••••••mnop");
/// assert_eq!(mask_api_key("sk-test-1234567890abcdef"),      "sk-••••••••cdef");
/// assert_eq!(mask_api_key("gsk_test_1234567890abcdef"),     "gsk_••••••••cdef");
/// assert_eq!(mask_api_key("AIzaSyABCDEFGH1234IJKLmnop"),    "AIza••••••••mnop");
/// assert_eq!(mask_api_key("xai-test-1234567890abcdef"),     "xai-••••••••cdef");
/// assert_eq!(mask_api_key("short"),                          "••••••••");
/// ```
pub fn mask_api_key(api_key: &str) -> String {
    // TS contract: `apiKey.length < 12` — JavaScript's `.length` counts
    // UTF-16 code units. For the ASCII API keys this matches Rust's
    // `.len()` byte count exactly. Non-ASCII keys are not in scope.
    if api_key.len() < 12 {
        return "••••••••".to_string();
    }

    const PREFIXES: &[&str] = &["sk-ant-", "sk-", "gsk_", "AIza", "xai-"];
    let prefix: &str = PREFIXES
        .iter()
        .find(|p| api_key.starts_with(*p))
        .copied()
        .unwrap_or(&api_key[..6]);
    let suffix = &api_key[api_key.len() - 4..];
    format!("{prefix}••••••••{suffix}")
}

/// Provider credentials detected from environment variables and auth files
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub claude_available: bool,
    pub openai_available: bool,
    pub codex_available: bool,
    pub gemini_available: bool,
    pub zai_available: bool,
    pub github_copilot_available: bool,
    /// PROV-067: Availability map keyed by custom provider slug.
    ///
    /// Populated by [`ProviderCredentials::detect`] by invoking
    /// `crate::custom::discover_provider_configs()` and checking each
    /// config's `api_key_env_var` (or the env var inside its
    /// [`crate::custom::AuthConfig`]). An unset or empty env var yields
    /// `false`. Unknown slugs return `false` via
    /// [`ProviderCredentials::has_custom`].
    #[doc(hidden)]
    pub custom_available: HashMap<String, bool>,
}

impl ProviderCredentials {
    /// Detect all available provider credentials
    pub fn detect() -> Self {
        Self {
            claude_available: std::env::var("ANTHROPIC_API_KEY").is_ok()
                || std::env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok()
                || has_claude_auth(),
            openai_available: std::env::var("OPENAI_API_KEY").is_ok(),
            codex_available: has_codex_auth(),
            gemini_available: std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").is_ok(),
            // Z.AI: Check both ZAI_PLAN_API_KEY (preferred) and ZAI_API_KEY
            zai_available: std::env::var("ZAI_PLAN_API_KEY").is_ok()
                || std::env::var("ZAI_API_KEY").is_ok(),
            // PROV-053: GitHub Copilot — auth file lives at
            // ~/.fspec/credentials/copilot_auth.json (mode 0600), written by
            // the OAuth device flow in PROV-054.
            github_copilot_available: has_github_copilot_auth(),
            // PROV-067: Scan discovered custom provider configs and
            // probe each one's api_key env var. Errors are swallowed so
            // a malformed config never breaks detection for built-ins.
            custom_available: detect_custom_provider_availability(),
        }
    }

    /// Check if any provider credentials are available
    pub fn has_any(&self) -> bool {
        self.claude_available
            || self.openai_available
            || self.codex_available
            || self.gemini_available
            || self.zai_available
            || self.github_copilot_available
    }

    /// Check if Claude credentials are available
    pub fn has_claude(&self) -> bool {
        self.claude_available
    }

    /// Check if OpenAI credentials are available
    pub fn has_openai(&self) -> bool {
        self.openai_available
    }

    /// Check if Codex credentials are available
    pub fn has_codex(&self) -> bool {
        self.codex_available
    }

    /// Check if Gemini credentials are available
    pub fn has_gemini(&self) -> bool {
        self.gemini_available
    }

    /// Check if Z.AI credentials are available
    pub fn has_zai(&self) -> bool {
        self.zai_available
    }

    /// Check if GitHub Copilot credentials are available (PROV-053)
    pub fn has_github_copilot(&self) -> bool {
        self.github_copilot_available
    }

    /// PROV-067: Check if a discovered custom provider has usable
    /// credentials. Returns `false` for unknown or unregistered slugs.
    pub fn has_custom(&self, name: &str) -> bool {
        self.custom_available.get(name).copied().unwrap_or(false)
    }

    /// List all available provider names
    pub fn available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.claude_available {
            providers.push("claude".to_string());
        }
        if self.gemini_available {
            providers.push("gemini".to_string());
        }
        if self.zai_available {
            providers.push("zai".to_string());
        }
        if self.codex_available {
            providers.push("codex".to_string());
        }
        if self.openai_available {
            providers.push("openai".to_string());
        }
        if self.github_copilot_available {
            providers.push("github-copilot".to_string());
        }
        providers
    }
}

/// Check if Codex auth.json exists with valid credentials
/// Matches codelet's hasCodexCredentials() implementation
fn has_codex_auth() -> bool {
    use crate::codex::codex_auth::read_codex_auth;

    if let Ok(Some(auth)) = read_codex_auth() {
        // Check for either cached API key or OAuth tokens
        if auth.openai_api_key.is_some() {
            return true;
        }
        if let Some(tokens) = auth.tokens {
            return !tokens.refresh_token.is_empty() && !tokens.account_id.is_empty();
        }
    }
    false
}

/// Check if claude_auth.json exists with valid OAuth credentials
/// Mirrors has_codex_auth() — checks claude_auth.json for OAuth tokens.
fn has_claude_auth() -> bool {
    use crate::claude_auth::read_claude_auth_sync;

    if let Ok(Some(auth)) = read_claude_auth_sync() {
        return !auth.access_token.is_empty() && !auth.refresh_token.is_empty();
    }
    false
}

/// Check if copilot_auth.json exists with a valid OAuth access token (PROV-053).
///
/// Mirrors `has_claude_auth()` and `has_codex_auth()`. The Copilot credential
/// file lives at `~/.fspec/credentials/copilot_auth.json` (or under `FSPEC_HOME`
/// if set) and is created by the device authorization flow in PROV-054.
fn has_github_copilot_auth() -> bool {
    use crate::copilot::auth::read_copilot_auth_sync;

    if let Ok(Some(auth)) = read_copilot_auth_sync() {
        return !auth.github_oauth_token.is_empty();
    }
    false
}

/// PROV-067: Scan all discovered custom provider configs and probe their
/// required env var. Returns a map from provider slug -> availability.
///
/// Availability rules:
/// - [`crate::custom::ProviderConfig::api_key_env_var`] takes precedence
///   when `Some`; the env var must be set and non-empty.
/// - Otherwise falls through to the auth block:
///   * [`crate::custom::AuthConfig::Bearer`] / `ApiKeyHeader` → env_var must
///     be set and non-empty
///   * [`crate::custom::AuthConfig::Custom`] → `true` when a
///     `credential_file` is specified and exists inside the fspec
///     credentials dir; `true` unconditionally when a `facade` is set
///     (the facade provider owns credential detection)
///   * OAuth variants → `false` (device-code / PKCE flows have no
///     lightweight availability probe yet)
fn detect_custom_provider_availability() -> std::collections::HashMap<String, bool> {
    use crate::custom::{discover_provider_configs, AuthConfig};

    let mut result = std::collections::HashMap::new();
    let configs = match discover_provider_configs() {
        Ok(cs) => cs,
        Err(e) => {
            tracing::debug!(
                error = %e,
                "custom provider discovery failed during credential detection"
            );
            return result;
        }
    };

    for cfg in configs {
        // Check the explicit api_key_env_var first.
        //
        // PROV-067 semantics (and the
        // `custom_provider_is_unavailable_when_required_env_var_is_unset`
        // regression test): when a custom provider config explicitly
        // declares `api_key_env_var`, that env var is the SOLE
        // availability signal. Whether or not the env var is set
        // overrides the auth-block / facade fallback — declaring the
        // env var in the config is the operator's way of saying "this
        // provider requires this env var to be set", so we must not
        // silently treat the provider as available just because a
        // facade is configured.
        let api_key_explicitly_required = cfg.api_key_env_var.is_some();
        let api_key_env_var_ok = cfg
            .api_key_env_var
            .as_ref()
            .and_then(|v| std::env::var(v).ok())
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        let available = if api_key_explicitly_required {
            api_key_env_var_ok
        } else {
            match &cfg.auth {
                AuthConfig::Bearer { env_var, .. }
                | AuthConfig::ApiKeyHeader { env_var, .. } => std::env::var(env_var)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false),
                // A custom auth provider is available when it has a
                // facade (env-var plumbing handled by the facade) OR a
                // Rhai script (the script handles auth internally, e.g.
                // embedded tokens or custom credential resolution).
                AuthConfig::Custom { .. } => {
                    cfg.facade.is_some() || !cfg.script.is_empty()
                }
                AuthConfig::OauthDeviceCode { .. } | AuthConfig::OauthPkce { .. } => false,
            }
        };
        result.insert(cfg.name, available);
    }

    result
}
