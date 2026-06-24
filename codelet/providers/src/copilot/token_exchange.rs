//! GitHub Copilot token exchange (PROV-057, L2).
//!
//! Implements the missing step in the Copilot two-token model:
//!
//! 1. The OAuth device flow gives the user a long-lived **GitHub** OAuth
//!    token (`gho_*` / `ghu_*`) that is stored in `copilot_auth.json`.
//! 2. That token is then exchanged at
//!    `GET /copilot_internal/v2/token` for a short-lived (~25 min)
//!    **Copilot** API token.
//! 3. Only that short-lived Copilot token is accepted by
//!    `api.githubcopilot.com` as `Authorization: Bearer <token>`.
//!
//! Before PROV-057 fspec skipped step 2 entirely and sent the `gho_*`
//! token straight to `api.githubcopilot.com`, which always returned 401.
//!
//! This module is deliberately transport-thin. The HTTP request is a
//! single `GET` with a handful of headers and the only logic is JSON
//! deserialization. The caller (`CopilotProvider::ensure_fresh_copilot_token`)
//! owns the refresh-decision policy and the persistence side-effects.

use crate::copilot::constants::copilot_user_agent;
use crate::error::ProviderError;
use serde::Deserialize;
use std::time::Duration;

/// Path segment appended to the token-exchange host.
///
/// For `github.com` the full URL is
/// `https://api.github.com/copilot_internal/v2/token`. For GitHub
/// Enterprise it is `https://<host>/api/v3/copilot_internal/v2/token`.
pub const TOKEN_EXCHANGE_PATH: &str = "/copilot_internal/v2/token";

/// Default github.com token-exchange host.
pub const GITHUB_API_HOST: &str = "https://api.github.com";

/// Single-attempt network timeout for the token exchange.
pub const TOKEN_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Header value used for `Editor-Version` on the token exchange.
///
/// GitHub's token-exchange endpoint requires an `Editor-Version` header
/// whose value is an opaque `<editor>/<version>` string. Codelet
/// masquerades as a minimal editor identity so the exchange is accepted.
#[must_use]
pub fn editor_version_header() -> String {
    format!("codelet/{}", env!("CARGO_PKG_VERSION"))
}

/// Header value used for `Editor-Plugin-Version` on the token exchange.
#[must_use]
pub fn editor_plugin_version_header() -> String {
    format!("codelet-copilot/{}", env!("CARGO_PKG_VERSION"))
}

/// Wire-format `endpoints` object on the token-exchange response. We only
/// care about the `api` field — it points at the correct `api.githubcopilot.com`
/// (or `copilot-api.<enterprise-host>`) base URL to use for subsequent
/// chat completion requests.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenExchangeEndpointsWire {
    #[serde(default)]
    pub api: Option<String>,
}

/// Wire-format response body returned by
/// `GET /copilot_internal/v2/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenExchangeWireResponse {
    pub token: String,
    pub expires_at: u64,
    #[serde(default)]
    pub refresh_in: Option<u64>,
    #[serde(default)]
    pub endpoints: Option<TokenExchangeEndpointsWire>,
}

/// Domain-model response from a successful token exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenExchangeResponse {
    /// The short-lived opaque Copilot token (~25 min TTL).
    pub token: String,
    /// Unix seconds at which the token expires.
    pub expires_at: u64,
    /// The `endpoints.api` URL to use for subsequent chat completion
    /// requests. Empty string if the response omitted it — in which case
    /// callers should fall back to the statically computed base URL.
    pub endpoints_api: String,
}

impl From<TokenExchangeWireResponse> for TokenExchangeResponse {
    fn from(wire: TokenExchangeWireResponse) -> Self {
        let endpoints_api = wire.endpoints.and_then(|e| e.api).unwrap_or_default();
        Self {
            token: wire.token,
            expires_at: wire.expires_at,
            endpoints_api,
        }
    }
}

/// Build the full token-exchange URL for a deployment.
///
/// - `github.com` → `https://api.github.com/copilot_internal/v2/token`
/// - Enterprise → `https://<host>/api/v3/copilot_internal/v2/token`
#[must_use]
pub fn build_token_exchange_url(enterprise_host: Option<&str>) -> String {
    match enterprise_host {
        Some(host) if !host.is_empty() => {
            format!("https://{host}/api/v3{TOKEN_EXCHANGE_PATH}")
        }
        _ => format!("{GITHUB_API_HOST}{TOKEN_EXCHANGE_PATH}"),
    }
}

/// Exchange a long-lived GitHub OAuth token for a short-lived Copilot
/// token via `GET /copilot_internal/v2/token`.
///
/// Headers sent:
/// - `Authorization: token <gho_*>` (note: **`token`**, not `Bearer`)
/// - `Editor-Version: codelet/<version>`
/// - `Editor-Plugin-Version: codelet-copilot/<version>`
/// - `User-Agent: codelet/<version>`
/// - `Accept: application/json`
///
/// # Errors
///
/// Returns [`ProviderError::Api`] for transport, non-2xx status, or
/// JSON-parse failures.
pub async fn exchange_github_token_for_copilot_token(
    github_oauth_token: &str,
    enterprise_host: Option<&str>,
) -> Result<TokenExchangeResponse, ProviderError> {
    let url = build_token_exchange_url(enterprise_host);
    exchange_github_token_for_copilot_token_at(&url, github_oauth_token).await
}

/// Test-seam variant of [`exchange_github_token_for_copilot_token`] that
/// accepts a fully-qualified URL. Production callers use the higher-level
/// wrapper; tests point this at a wiremock server.
pub async fn exchange_github_token_for_copilot_token_at(
    url: &str,
    github_oauth_token: &str,
) -> Result<TokenExchangeResponse, ProviderError> {
    if github_oauth_token.is_empty() {
        return Err(ProviderError::auth(
            "github-copilot",
            "token exchange called with empty github_oauth_token",
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(TOKEN_EXCHANGE_TIMEOUT)
        .build()
        .map_err(|e| {
            ProviderError::api(
                "github-copilot",
                format!("failed to build token-exchange HTTP client: {e}"),
            )
        })?;

    let response = client
        .get(url)
        .header("Authorization", format!("token {github_oauth_token}"))
        .header("Editor-Version", editor_version_header())
        .header("Editor-Plugin-Version", editor_plugin_version_header())
        .header("User-Agent", copilot_user_agent())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            ProviderError::api(
                "github-copilot",
                format!("token exchange request failed: {e}"),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::api(
            "github-copilot",
            format!("token exchange returned HTTP {status}: {body}"),
        ));
    }

    let wire: TokenExchangeWireResponse = response.json().await.map_err(|e| {
        ProviderError::api(
            "github-copilot",
            format!("token exchange response parse error: {e}"),
        )
    })?;

    Ok(wire.into())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "token_exchange_tests.rs"]
mod tests;
