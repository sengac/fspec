//! Codex OAuth Login Flow (PROV-011)
//!
//! Implements browser OAuth and device auth flows for authenticating
//! with ChatGPT Plus/Pro subscriptions. This module provides:
//!
//! - JWT claims parsing and account ID extraction
//! - OAuth authorize URL construction
//! - OAuth callback state validation
//! - Codex API endpoint URL rewriting and header building
//! - Token refresh via refresh_token grant
//!
//! PKCE generation and URL-encoding are in the shared `oauth_crypto` module
//! and re-exported here for backward compatibility.
//!
//! Reference: OpenCode's codex.ts plugin

use anyhow::{anyhow, Result};
use base64::Engine;
use rand::Rng;
use std::collections::HashMap;

// Re-export shared OAuth crypto primitives from the provider-agnostic module.
// This preserves backward compatibility for existing Codex consumers.
pub use crate::oauth_crypto::{generate_pkce, urlencoded, PkceCodes};

/// OAuth constants from Codex CLI
pub const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_ISSUER: &str = "https://auth.openai.com";
pub const CODEX_API_ENDPOINT: &str = "https://chatgpt.com/backend-api/codex/responses";
pub const OAUTH_PORT: u16 = 1455;
pub const OAUTH_TIMEOUT_MS: u64 = 5 * 60 * 1000; // 5 minutes

/// Generate a random state parameter for CSRF protection
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

/// Parse JWT claims from a token string (no signature verification)
///
/// JWTs are structured as: header.payload.signature
/// We decode the payload (part[1]) as JSON.
pub fn parse_jwt_claims(token: &str) -> Result<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(anyhow!("Invalid JWT: expected 3 parts, got {}", parts.len()));
    }

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| {
            // Try standard base64 as fallback (some JWTs use padding)
            base64::engine::general_purpose::URL_SAFE.decode(parts[1])
        })
        .map_err(|e| anyhow!("Failed to decode JWT payload: {e}"))?;

    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| anyhow!("Failed to parse JWT claims: {e}"))?;

    Ok(claims)
}

/// Extract account ID from JWT claims
///
/// Checks in order (matching OpenCode reference):
/// 1. `chatgpt_account_id` (top-level)
/// 2. `https://api.openai.com/auth.chatgpt_account_id` (nested)
/// 3. `organizations[0].id` (fallback)
pub fn extract_account_id_from_claims(claims: &serde_json::Value) -> Option<String> {
    // 1. Top-level chatgpt_account_id
    if let Some(id) = claims.get("chatgpt_account_id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }

    // 2. Nested under https://api.openai.com/auth
    if let Some(auth) = claims.get("https://api.openai.com/auth") {
        if let Some(id) = auth.get("chatgpt_account_id").and_then(|v| v.as_str()) {
            return Some(id.to_string());
        }
    }

    // 3. organizations[0].id fallback
    if let Some(orgs) = claims.get("organizations").and_then(|v| v.as_array()) {
        if let Some(first_org) = orgs.first() {
            if let Some(id) = first_org.get("id").and_then(|v| v.as_str()) {
                return Some(id.to_string());
            }
        }
    }

    None
}

/// Extract account ID from a token response (tries id_token then access_token)
pub fn extract_account_id(id_token: Option<&str>, access_token: Option<&str>) -> Option<String> {
    // Try id_token first
    if let Some(token) = id_token {
        if let Ok(claims) = parse_jwt_claims(token) {
            if let Some(account_id) = extract_account_id_from_claims(&claims) {
                return Some(account_id);
            }
        }
    }

    // Fallback to access_token
    if let Some(token) = access_token {
        if let Ok(claims) = parse_jwt_claims(token) {
            return extract_account_id_from_claims(&claims);
        }
    }

    None
}

/// Build the OAuth authorize URL for browser-based login
pub fn build_authorize_url(redirect_uri: &str, pkce: &PkceCodes, state: &str) -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", CODEX_CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("scope", "openid profile email offline_access"),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", &pkce.challenge_method),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("state", state),
        ("originator", "codelet"),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{CODEX_ISSUER}/oauth/authorize?{query}")
}

/// Validate an OAuth callback's state parameter against the expected state
///
/// Returns an error if the state doesn't match (CSRF protection).
pub fn validate_oauth_callback(callback_state: &str, expected_state: &str) -> Result<()> {
    if callback_state != expected_state {
        return Err(anyhow!(
            "Invalid state parameter - potential CSRF attack. Expected: {expected_state}, Got: {callback_state}"
        ));
    }
    Ok(())
}

/// Rewrite a standard OpenAI API URL to the Codex endpoint
///
/// If the URL contains /v1/responses, /responses, or /chat/completions,
/// rewrite to the Codex API endpoint. The Responses API client posts to
/// /responses (which rig builds as base_url + /responses).
/// Otherwise return the original URL unchanged.
pub fn rewrite_codex_url(url: &str) -> String {
    if url.contains("/v1/responses") || url.contains("/chat/completions") {
        CODEX_API_ENDPOINT.to_string()
    } else if url.ends_with("/responses") {
        // Catch the bare /responses path from the Responses API client
        CODEX_API_ENDPOINT.to_string()
    } else {
        url.to_string()
    }
}

/// Build HTTP headers for Codex API requests
///
/// Returns a HashMap with:
/// - `authorization`: Bearer {access_token}
/// - `ChatGPT-Account-Id`: {account_id}
/// - `originator`: "codelet"
pub fn build_codex_headers(access_token: &str, account_id: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        "authorization".to_string(),
        format!("Bearer {access_token}"),
    );
    headers.insert("ChatGPT-Account-Id".to_string(), account_id.to_string());
    headers.insert("originator".to_string(), "codelet".to_string());
    headers
}

/// Response from the OAuth token endpoint
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TokenRefreshResponse {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// POST form-encoded parameters to an OAuth token endpoint and parse the response.
///
/// Shared by `refresh_access_token`, `exchange_authorization_code`, and
/// `exchange_device_code` (PROV-014) to eliminate duplicated HTTP +
/// error-handling boilerplate.
pub(crate) async fn post_to_token_endpoint(
    token_url: &str,
    params: &[(&str, &str)],
    error_context: &str,
) -> Result<TokenRefreshResponse> {
    let client = reqwest::Client::new();

    let response = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("{error_context} failed with status {status}: {body}"));
    }

    let token_response: TokenRefreshResponse = response.json().await?;
    Ok(token_response)
}

/// Refresh an access token using the refresh_token grant
///
/// Sends a POST to {CODEX_ISSUER}/oauth/token with:
/// - grant_type: refresh_token
/// - client_id: CODEX_CLIENT_ID
/// - refresh_token: the provided refresh token
pub async fn refresh_access_token(refresh_token: &str) -> Result<TokenRefreshResponse> {
    refresh_access_token_at(CODEX_ISSUER, refresh_token).await
}

/// Refresh an access token at a specific issuer URL.
///
/// Same as `refresh_access_token` but allows callers (and tests) to
/// point at a different issuer (e.g. a wiremock server).
pub async fn refresh_access_token_at(
    issuer_url: &str,
    refresh_token: &str,
) -> Result<TokenRefreshResponse> {
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", CODEX_CLIENT_ID),
        ("refresh_token", refresh_token),
    ];

    post_to_token_endpoint(
        &format!("{issuer_url}/oauth/token"),
        &params,
        "Token refresh",
    )
    .await
}

/// Exchange an authorization code for tokens at the OAuth token endpoint (PROV-013)
///
/// Sends a POST to {issuer_url}/oauth/token with:
/// - grant_type: authorization_code
/// - code: the authorization code from the callback
/// - code_verifier: the PKCE code verifier
/// - client_id: CODEX_CLIENT_ID
/// - redirect_uri: (optional) the redirect URI used during authorization.
///   Required for browser OAuth (where a redirect was used), omitted for
///   device auth (which never redirects).
///
/// The `issuer_url` parameter allows tests to point at a wiremock server instead
/// of the real CODEX_ISSUER.
///
/// Returns a `TokenRefreshResponse` containing id_token, access_token, and refresh_token.
pub async fn exchange_authorization_code(
    issuer_url: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: Option<&str>,
) -> Result<TokenRefreshResponse> {
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("client_id", CODEX_CLIENT_ID),
    ];
    if let Some(uri) = redirect_uri {
        params.push(("redirect_uri", uri));
    }

    post_to_token_endpoint(
        &format!("{issuer_url}/oauth/token"),
        &params,
        "Token exchange",
    )
    .await
}

/// Check if the given elapsed time exceeds the OAuth timeout (5 minutes)
pub fn is_oauth_timeout_expired(elapsed_ms: u64) -> bool {
    elapsed_ms > OAUTH_TIMEOUT_MS
}

/// OAuth timeout helper for tracking elapsed time during browser OAuth flow
#[derive(Debug, Clone, Copy)]
pub struct OAuthTimeout {
    timeout_ms: u64,
}

impl OAuthTimeout {
    /// Create a new OAuthTimeout with the default 5-minute timeout
    pub fn default_timeout() -> Self {
        Self {
            timeout_ms: OAUTH_TIMEOUT_MS,
        }
    }

    /// Create a new OAuthTimeout with a custom timeout in milliseconds
    pub fn from_ms(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// Check if the given elapsed time exceeds the timeout
    pub fn is_expired_after_ms(&self, elapsed_ms: u64) -> bool {
        elapsed_ms > self.timeout_ms
    }

    /// Get the timeout duration in milliseconds
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}

/// HTML page shown on successful OAuth callback
pub const HTML_SUCCESS: &str = r#"<!doctype html>
<html>
  <head>
    <title>Codelet - Authorization Successful</title>
    <style>
      body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #f1ecec; margin-bottom: 1rem; }
      p { color: #b7b1b1; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to codelet.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000)</script>
  </body>
</html>"#;

/// HTML page shown when user cancels OAuth flow
pub const HTML_CANCELLED: &str = r#"<!doctype html>
<html>
  <head>
    <title>Codelet - Login Cancelled</title>
    <style>
      body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #b7b1b1; margin-bottom: 1rem; }
      p { color: #888; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Login Cancelled</h1>
      <p>The login was cancelled. You can close this window.</p>
    </div>
  </body>
</html>"#;

/// HTML page shown on failed OAuth callback
pub fn html_error(error: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>Codelet - Authorization Failed</title>
    <style>
      body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #131010; color: #f1ecec; }}
      .container {{ text-align: center; padding: 2rem; }}
      h1 {{ color: #fc533a; margin-bottom: 1rem; }}
      p {{ color: #b7b1b1; }}
      .error {{ color: #ff917b; font-family: monospace; margin-top: 1rem; padding: 1rem; background: #3c140d; border-radius: 0.5rem; }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Failed</h1>
      <p>An error occurred during authorization.</p>
      <div class="error">{error}</div>
    </div>
  </body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_verifier_length() {
        let pkce = generate_pkce();
        assert!(pkce.verifier.len() >= 43);
        assert!(pkce.verifier.len() <= 128);
    }

    #[test]
    fn test_pkce_verifier_charset() {
        let pkce = generate_pkce();
        let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        for c in pkce.verifier.chars() {
            assert!(allowed.contains(c), "Invalid character in verifier: {}", c);
        }
    }

    #[test]
    fn test_pkce_challenge_is_s256() {
        let pkce = generate_pkce();
        assert_eq!(pkce.challenge_method, "S256");
    }

    #[test]
    fn test_pkce_deterministic() {
        let pkce1 = PkceCodes::from_verifier("test_verifier_abc".to_string());
        let pkce2 = PkceCodes::from_verifier("test_verifier_abc".to_string());
        assert_eq!(pkce1.challenge, pkce2.challenge);
    }

    #[test]
    fn test_url_rewrite_v1_responses() {
        assert_eq!(
            rewrite_codex_url("https://api.openai.com/v1/responses"),
            CODEX_API_ENDPOINT
        );
    }

    #[test]
    fn test_url_rewrite_bare_responses() {
        // Catches the case where the URL ends with /responses without /v1/ prefix
        assert_eq!(
            rewrite_codex_url("https://some-host.example.com/responses"),
            CODEX_API_ENDPOINT
        );
    }

    #[test]
    fn test_url_rewrite_chat_completions() {
        assert_eq!(
            rewrite_codex_url("https://api.openai.com/v1/chat/completions"),
            CODEX_API_ENDPOINT
        );
    }

    #[test]
    fn test_url_no_rewrite() {
        let url = "https://api.openai.com/v1/models";
        assert_eq!(rewrite_codex_url(url), url);
    }
}
