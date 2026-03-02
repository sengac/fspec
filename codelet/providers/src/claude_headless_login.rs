//! Claude Headless Login Flow (PROV-022)
//!
//! Implements headless/device authentication for environments where a browser
//! can't be opened (SSH, containers, headless servers).
//!
//! Unlike Codex device auth (RFC 8628 with polling), Anthropic has no device
//! authorization endpoints. The headless flow is a standard OAuth authorization
//! code flow with manual code paste instead of browser redirect — matching
//! opencode's anthropic auth plugin behavior.
//!
//! Flow:
//! 1. Generate PKCE codes
//! 2. Build authorize URL (state = verifier)
//! 3. Invoke code-entry callback with the URL (caller decides how to prompt user)
//! 4. Parse code#state from callback result
//! 5. Validate state matches PKCE verifier
//! 6. Exchange authorization code for tokens (JSON POST)
//! 7. Persist tokens to claude_auth.json
//! 8. Return ClaudeAuthJson
//!
//! No HTTP server, no browser opening, no port binding.

use std::future::Future;
use std::pin::Pin;

use anyhow::{anyhow, Result};
use tracing::info;

use crate::claude_auth::{write_claude_auth, ClaudeAuthJson};
use crate::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code, parse_authorization_code,
};
use crate::oauth_crypto::{generate_pkce, PkceCodes};

/// Async callback type for code entry.
///
/// Receives the authorize URL and returns the user-pasted `code#state` string.
/// This allows NAPI/TUI callers to provide their own input mechanism.
pub type CodeEntryFn =
    Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send>;

/// Configuration for `claude_headless_login` to support both production and test use.
///
/// Mirrors the `DeviceAuthConfig` pattern from Codex PROV-014.
pub struct ClaudeHeadlessLoginConfig {
    /// Base URL for the token endpoint.
    /// Production: "https://console.anthropic.com"
    /// Tests: wiremock server URL
    pub token_endpoint_base: String,

    /// Overall timeout in milliseconds for the code-entry callback.
    /// If the callback does not return within this duration, the flow fails.
    pub timeout_ms: u64,

    /// Optional pre-generated PKCE codes.
    /// Tests inject known values; production generates fresh codes.
    pub pkce: Option<PkceCodes>,

    /// Async callback that receives the authorize URL and returns the
    /// user-pasted `code#state` string. This allows NAPI/TUI callers
    /// to provide their own input mechanism (stdin, GUI dialog, etc.).
    pub code_entry_fn: CodeEntryFn,
}

/// Orchestrate the full Claude headless login flow.
///
/// 1. Generate PKCE (or use injected values)
/// 2. Build authorize URL
/// 3. Invoke code-entry callback with the URL
/// 4. Parse code#state from result
/// 5. Validate state matches PKCE verifier
/// 6. Exchange code for tokens
/// 7. Persist tokens
/// 8. Return ClaudeAuthJson
pub async fn claude_headless_login(config: ClaudeHeadlessLoginConfig) -> Result<ClaudeAuthJson> {
    // Step 1: Generate PKCE (or use injected values for testing)
    let pkce = config.pkce.unwrap_or_else(generate_pkce);
    let expected_state = pkce.verifier.clone();

    // Step 2: Build authorize URL
    let auth_url = build_authorize_url(&pkce);

    info!("Claude headless login — authorize URL: {auth_url}");

    // Step 3: Invoke code-entry callback with timeout
    let callback_future = (config.code_entry_fn)(auth_url);

    let raw_code = match tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        callback_future,
    )
    .await
    {
        Ok(Ok(code)) => code,
        Ok(Err(e)) => return Err(anyhow!("Code entry callback failed: {e}")),
        Err(_) => {
            return Err(anyhow!(
                "Claude headless login timed out after {}ms — no code entered",
                config.timeout_ms
            ));
        }
    };

    // Step 4: Validate non-empty
    if raw_code.is_empty() {
        return Err(anyhow!(
            "Empty authorization code submitted — no code to exchange"
        ));
    }

    // Step 5: Parse code#state format
    let (code, maybe_state) = parse_authorization_code(&raw_code);

    let received_state = match maybe_state {
        Some(state) => state,
        None => {
            return Err(anyhow!(
                "Missing state in authorization code — code must be in 'code#state' format"
            ));
        }
    };

    // Step 6: Validate state matches PKCE verifier
    if received_state != expected_state {
        return Err(anyhow!(
            "CSRF validation failed — state mismatch. Expected verifier: {expected_state}, Got: {received_state}"
        ));
    }

    // Step 7: Exchange code for tokens
    let token_response = exchange_authorization_code(
        &config.token_endpoint_base,
        &code,
        &received_state,
        &pkce.verifier,
    )
    .await?;

    // Step 8: Build and persist ClaudeAuthJson
    let expires = calculate_expiry(token_response.expires_in);
    let auth = ClaudeAuthJson {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        expires,
    };

    write_claude_auth(&auth).await?;

    info!("Claude headless login successful — tokens persisted to claude_auth.json");

    Ok(auth)
}
