//! Claude OAuth Core Flow (PROV-020)
//!
//! Implements PKCE-based OAuth primitives for authenticating with
//! Claude Pro/Max subscriptions via Anthropic's OAuth endpoints.
//!
//! This module provides:
//! - OAuth authorize URL construction for Max mode
//! - Authorization code parsing (code#state format)
//! - Token exchange via JSON POST (NOT form-encoded like Codex)
//! - Token refresh via refresh_token grant
//! - OAuth header building with required beta headers
//! - Tool name prefixing/stripping (mcp_ prefix)
//! - URL rewriting for /v1/messages (append ?beta=true)
//! - Token expiry calculation
//!
//! PKCE generation and URL-encoding are in the shared `oauth_crypto` module.
//!
//! Reference: opencode-anthropic-auth npm package (v0.0.13)

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::oauth_crypto::{urlencoded, PkceCodes};

// =========================================================================
// Constants
// =========================================================================

/// OAuth client ID — shared with opencode-anthropic-auth plugin
pub const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Authorize URL base for Claude Max mode
pub const CLAUDE_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";

/// Token endpoint for exchange and refresh
pub const CLAUDE_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";

/// Base URL for token endpoint (without /v1/oauth/token suffix)
/// Used by RefreshingClaudeClient and manager.rs for token refresh
pub const CLAUDE_TOKEN_ENDPOINT_BASE: &str = "https://console.anthropic.com";

/// Redirect URI — Anthropic-hosted callback page (no local server)
pub const CLAUDE_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";

/// OAuth scopes
pub const CLAUDE_SCOPE: &str = "org:create_api_key user:profile user:inference";

/// User-Agent header for OAuth requests
///
/// Must match across all OAuth request paths:
/// - `build_oauth_headers()` (parity reference)
/// - `ClaudeProvider::from_api_key_with_mode_and_model()` (rig static headers)
/// - `ClaudeProvider::from_oauth_tokens()` (rig static headers)
pub const CLAUDE_USER_AGENT: &str = "claude-cli/2.1.3 (external, cli)";

/// Required beta headers for OAuth mode
pub const REQUIRED_BETA_HEADERS: &[&str] =
    &["oauth-2025-04-20", "interleaved-thinking-2025-05-14"];

/// Tool name prefix for OAuth mode
pub const TOOL_NAME_PREFIX: &str = "mcp_";

// =========================================================================
// Types
// =========================================================================

/// Response from the Anthropic OAuth token endpoint
///
/// Unlike Codex, there is no `id_token` — just access/refresh tokens
/// and an expiry duration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClaudeTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

// =========================================================================
// Authorize URL
// =========================================================================

/// Build the OAuth authorize URL for Claude Max mode
///
/// State is set to the PKCE verifier (not a separate random value),
/// which simplifies CSRF validation.
pub fn build_authorize_url(pkce: &PkceCodes) -> String {
    let params = [
        ("code", "true"),
        ("client_id", CLAUDE_CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", CLAUDE_REDIRECT_URI),
        ("scope", CLAUDE_SCOPE),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", &pkce.challenge_method),
        ("state", &pkce.verifier),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{CLAUDE_AUTHORIZE_URL}?{query}")
}

// =========================================================================
// Authorization code parsing
// =========================================================================

/// Parse an authorization code in `code#state` format
///
/// Anthropic's OAuth callback returns the code and state concatenated
/// with a `#` separator. If no `#` is present, the full string is
/// treated as the code with no state.
pub fn parse_authorization_code(raw: &str) -> (String, Option<String>) {
    match raw.split_once('#') {
        Some((code, state)) => (code.to_string(), Some(state.to_string())),
        None => (raw.to_string(), None),
    }
}

// =========================================================================
// Token exchange (JSON POST, not form-encoded)
// =========================================================================

/// Exchange an authorization code for tokens at the Anthropic token endpoint
///
/// Sends a JSON POST (NOT form-encoded like Codex) with:
/// - code, state, grant_type, client_id, redirect_uri, code_verifier
///
/// The `base_url` parameter allows tests to point at a wiremock server.
pub async fn exchange_authorization_code(
    base_url: &str,
    code: &str,
    state: &str,
    code_verifier: &str,
) -> Result<ClaudeTokenResponse> {
    let body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "state": state,
        "client_id": CLAUDE_CLIENT_ID,
        "redirect_uri": CLAUDE_REDIRECT_URI,
        "code_verifier": code_verifier,
    });

    post_json_to_token_endpoint(
        &format!("{base_url}/v1/oauth/token"),
        &body,
        "Token exchange",
    )
    .await
}

// =========================================================================
// Token refresh
// =========================================================================

/// Refresh an access token using the refresh_token grant
///
/// The `base_url` parameter allows tests to point at a wiremock server.
pub async fn refresh_access_token_at(
    base_url: &str,
    refresh_token: &str,
) -> Result<ClaudeTokenResponse> {
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": CLAUDE_CLIENT_ID,
    });

    post_json_to_token_endpoint(
        &format!("{base_url}/v1/oauth/token"),
        &body,
        "Token refresh",
    )
    .await
}

// =========================================================================
// Shared HTTP helper
// =========================================================================

/// POST a JSON body to the token endpoint and parse the response
async fn post_json_to_token_endpoint(
    token_url: &str,
    body: &serde_json::Value,
    error_context: &str,
) -> Result<ClaudeTokenResponse> {
    let client = reqwest::Client::new();

    let response = client
        .post(token_url)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "{error_context} failed with status {status}: {body_text}"
        ));
    }

    let token_response: ClaudeTokenResponse = response.json().await?;
    Ok(token_response)
}

// =========================================================================
// OAuth headers
// =========================================================================

/// Build HTTP headers for Claude OAuth API requests
///
/// Returns a HashMap with:
/// - `authorization`: Bearer {access_token}
/// - `anthropic-beta`: merged required + existing beta headers
/// - `user-agent`: claude-cli/2.1.3 (external, cli)
///
/// Does NOT include `x-api-key` — callers must remove it from existing
/// headers when switching to OAuth mode.
///
/// **Note:** This is a parity reference implementation that produces the same
/// header set as opencode's custom fetch. Production requests use rig's
/// `ClientBuilder::http_headers()` set in `ClaudeProvider::from_oauth_tokens()`.
/// Both paths produce identical output — this function is tested directly
/// for parity verification.
pub fn build_oauth_headers(
    access_token: &str,
    existing_beta: Option<&str>,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    // Authorization
    headers.insert(
        "authorization".to_string(),
        format!("Bearer {access_token}"),
    );

    // Merge beta headers: required + existing (deduplicated)
    let mut betas: Vec<&str> = REQUIRED_BETA_HEADERS.to_vec();
    if let Some(existing) = existing_beta {
        for beta in existing.split(',').map(str::trim).filter(|b| !b.is_empty()) {
            if !betas.contains(&beta) {
                betas.push(beta);
            }
        }
    }
    headers.insert("anthropic-beta".to_string(), betas.join(","));

    // User-Agent
    headers.insert("user-agent".to_string(), CLAUDE_USER_AGENT.to_string());

    headers
}

// =========================================================================
// Tool name prefixing
// =========================================================================

/// Add the mcp_ prefix to a tool name for OAuth mode
///
/// Parity reference: opencode's custom fetch prefixes all tool names with `mcp_`
/// because it routes tools through MCP servers. Our tools are native (not MCP),
/// so this function is not in the production request path. It exists for parity
/// verification testing and for future MCP integration.
pub fn prefix_tool_name(name: &str) -> String {
    format!("{TOOL_NAME_PREFIX}{name}")
}

/// Strip the mcp_ prefix from a tool name in a response
///
/// If the name doesn't start with the prefix, it's returned unchanged.
///
/// Parity reference: opencode uses regex to strip `mcp_` from streaming responses.
/// See `prefix_tool_name()` for context on why this isn't in the production path.
pub fn strip_tool_name_prefix(name: &str) -> String {
    name.strip_prefix(TOOL_NAME_PREFIX)
        .unwrap_or(name)
        .to_string()
}

// =========================================================================
// URL rewriting
// =========================================================================

/// Rewrite a Claude API URL for OAuth mode
///
/// If the URL path is /v1/messages, append `?beta=true` (or `&beta=true`
/// if query parameters already exist). Non-messages URLs pass through
/// unchanged.
///
/// **Note:** This is a parity reference implementation. Production URL rewriting
/// is handled by the patched rig `AnthropicExt::build_uri()` in
/// `patches/rig-core/src/providers/anthropic/client.rs`, which detects OAuth
/// mode via `AnthropicKey::is_oauth_token()` and appends `?beta=true` at the
/// rig layer. Both paths produce identical output.
pub fn rewrite_claude_url(url: &str) -> String {
    // Parse to check the path
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.path() == "/v1/messages" {
            // Check if already has beta param
            if parsed.query_pairs().any(|(k, _)| k == "beta") {
                return url.to_string();
            }

            return if parsed.query().is_some() {
                format!("{url}&beta=true")
            } else {
                format!("{url}?beta=true")
            };
        }
    }

    url.to_string()
}

// =========================================================================
// Token expiry
// =========================================================================

/// Calculate the expiry timestamp from expires_in seconds
///
/// Returns milliseconds since Unix epoch: now_ms + expires_in * 1000
pub fn calculate_expiry(expires_in: u64) -> u64 {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    now_ms + expires_in * 1000
}
