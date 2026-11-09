//! Provider credential wire types (RPC-054).

use super::*;

// ============================================================================
// RPC-054: Provider credentials surface — the wire shapes used by the
// `list_provider_credentials` / `set_provider_credentials` /
// `delete_provider_credentials` / `test_provider_connection` /
// `refresh_models_cache` RPCs that back the new Rust ratatui
// ProviderSettingsView (`/provider` slash command).
//
// All three types follow the same cfg-gated `napi(object)` + Serialize +
// Deserialize pattern as the surrounding `ProviderInfo` / `ModelEntry`
// shapes so codelet-napi can re-export them verbatim if a future JS
// surface needs them.
// ============================================================================

/// Summary entry per provider returned by
/// `FspecService::list_provider_credentials`. Drives the left-pane
/// provider list in `ProviderSettingsView`.
///
/// `credential_type` is one of `"api_key" | "oauth" | "custom"` and
/// matches the variant tag used in `ProviderCredentialInput::kind`.
/// `configured` reflects [`ProviderCredentials::detect`] (env var set,
/// auth file present, etc.). `model_count` is the number of models the
/// provider's config currently exposes — drives the
/// "(n models)" suffix in each row.
///
/// RPC-108: `masked_key` and `source` extend the wire surface so the
/// TUI can render `'sk-ant-••••mnop [env]'`-style indicators on
/// configured rows. Both stay `Option<String>` (not enums) because
/// `napi(object)` does not support discriminated enums — matches the
/// existing `credential_type` string convention at L407-409. Server-side
/// the masking happens inside `codelet-providers` via
/// `credentials::mask_api_key` BEFORE the data crosses the wire, so
/// raw key bytes never traverse either transport.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialInfo {
    pub provider_id: String,
    pub display_name: String,
    pub configured: bool,
    /// One of "api_key", "oauth", or "custom" — mirrors the variant
    /// discriminant used in [`ProviderCredentialInput::kind`].
    pub credential_type: String,
    pub model_count: u32,
    /// RPC-108: Display-safe masked credential (e.g.
    /// `Some("sk-ant-••••••••mnop")`). `None` for unconfigured rows and
    /// for OAuth-only providers (the view layer renders `'OAuth'`).
    pub masked_key: Option<String>,
    /// RPC-108: Provenance tag — one of `"explicit" | "file" | "env" |
    /// "dotenv"` mirroring TS `ProviderConfigResult.source` at
    /// `src/utils/credentials.ts:56-59`. `None` when unconfigured.
    pub source: Option<String>,
}

/// PROV-113: result of the synchronous-ish headless OAuth start (anthropic).
/// `authorize_url` is shown to the user immediately and `pkce_verifier` is
/// round-tripped back to `oauth_headless_complete` for CSRF validation. Not
/// napi-exposed — the TS TUI has its own napi bindings; this shape only
/// crosses the Rust tarpc boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthHeadlessStart {
    pub authorize_url: String,
    pub pkce_verifier: String,
}

/// PROV-113: result of the codex device-auth start. `user_code` +
/// `verification_url` are displayed immediately; `device_auth_id` + `interval`
/// drive the follow-up poll. Not napi-exposed (Rust tarpc boundary only).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthDeviceStart {
    pub user_code: String,
    pub verification_url: String,
    pub device_auth_id: String,
    pub interval: u64,
}

/// Credential write payload consumed by
/// `FspecService::set_provider_credentials`. Encoded as a struct with a
/// `kind` discriminant (rather than a Rust enum) so that the
/// `napi(object)` derive remains valid — `napi_derive::napi(object)`
/// does not support discriminated enums and the JS surface needs to
/// re-export the same shape.
///
/// `kind` is one of:
///   * `"api_key"`     — `api_key` MUST be Some
///   * `"oauth"`       — `oauth_token` MUST be Some; `oauth_refresh_token` optional
///   * `"custom"`      — `custom_endpoint` MUST be Some; `custom_api_key` optional
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProviderCredentialInput {
    /// "api_key" | "oauth" | "custom"
    pub kind: String,
    pub api_key: Option<String>,
    pub oauth_token: Option<String>,
    pub oauth_refresh_token: Option<String>,
    pub custom_endpoint: Option<String>,
    pub custom_api_key: Option<String>,
}

impl ProviderCredentialInput {
    /// Construct an `api_key` variant. The Rust API stays ergonomic
    /// even though the wire shape is a flat struct.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            kind: "api_key".to_string(),
            api_key: Some(key.into()),
            ..Self::default()
        }
    }

    /// Construct an `oauth` variant.
    pub fn oauth(token: impl Into<String>, refresh: Option<String>) -> Self {
        Self {
            kind: "oauth".to_string(),
            oauth_token: Some(token.into()),
            oauth_refresh_token: refresh,
            ..Self::default()
        }
    }

    /// Construct a `custom` variant.
    pub fn custom(endpoint: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            kind: "custom".to_string(),
            custom_endpoint: Some(endpoint.into()),
            custom_api_key: api_key,
            ..Self::default()
        }
    }
}

/// Result returned by `FspecService::test_provider_connection`. Drives
/// the right-pane status area in `ProviderSettingsView` — a `success:
/// true` value renders "✓ ok (latency_ms ms)" and a `success: false`
/// value renders "✗ <error>".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u32,
}
