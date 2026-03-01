//! NAPI Bindings for Codex OAuth Flows (PROV-015)
//!
//! Exposes the Rust OAuth flows (browser login, device auth, token refresh,
//! token retrieval) to the TypeScript TUI layer via NAPI bindings.
//!
//! Architecture:
//! - codex_oauth_browser_login() → async, spawns browser_oauth_login()
//! - codex_oauth_device_login_start() → sync, returns user_code + verification_url
//! - codex_oauth_device_login_poll() → async, polls and returns tokens
//! - codex_oauth_refresh_token() → async, refreshes and persists tokens
//! - codex_oauth_get_tokens() → sync, reads auth.json

use codelet_providers::codex::codex_auth::{
    read_codex_auth, write_codex_auth, CodexAuthJson, CodexTokens,
};
use codelet_providers::codex::codex_device_auth::{
    poll_device_token, request_device_code, PollConfig, PollResult,
};
use codelet_providers::codex::codex_oauth::{
    exchange_authorization_code, extract_account_id, refresh_access_token, TokenRefreshResponse,
    CODEX_ISSUER, OAUTH_TIMEOUT_MS,
};
use codelet_providers::codex::codex_oauth_server::browser_oauth_login;
use napi::bindgen_prelude::*;

// ============================================================================
// NAPI Object Structs
// ============================================================================

/// Codex OAuth tokens exposed to TypeScript.
///
/// Rule [5]: Maps 1:1 to the Rust CodexTokens struct.
/// All fields are strings.
#[napi(object)]
pub struct NapiCodexTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

impl From<CodexTokens> for NapiCodexTokens {
    fn from(tokens: CodexTokens) -> Self {
        Self {
            id_token: tokens.id_token,
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            account_id: tokens.account_id,
        }
    }
}

/// Result from device auth login start — returned synchronously so the TUI
/// can display user_code and verification_url immediately.
///
/// Architecture note [3]: Two-phase design avoids the complexity of returning
/// both a sync result and a promise from a single function.
#[napi(object)]
pub struct NapiDeviceAuthStartResult {
    pub user_code: String,
    pub verification_url: String,
    pub device_auth_id: String,
    pub interval: f64,
}

// ============================================================================
// Shared Helpers
// ============================================================================

/// Build CodexTokens from a token endpoint response and persist to auth.json.
///
/// Extracts account_id from the JWT, assembles CodexTokens, writes to
/// auth.json, and converts to NapiCodexTokens. Used by both device auth
/// poll and token refresh to eliminate duplication.
fn build_and_persist_tokens(
    token_response: &TokenRefreshResponse,
    persist_error_context: &str,
) -> Result<NapiCodexTokens> {
    let account_id = extract_account_id(
        Some(&token_response.id_token),
        Some(&token_response.access_token),
    )
    .ok_or_else(|| Error::from_reason("Failed to extract account_id from token response"))?;

    let tokens = CodexTokens {
        id_token: token_response.id_token.clone(),
        access_token: token_response.access_token.clone(),
        refresh_token: token_response.refresh_token.clone(),
        account_id,
    };

    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(tokens.clone()),
        last_refresh: None,
    };
    write_codex_auth(&auth)
        .map_err(|e| Error::from_reason(format!("{persist_error_context}: {e}")))?;

    Ok(NapiCodexTokens::from(tokens))
}

// ============================================================================
// Browser OAuth Login
// ============================================================================

/// Start the browser OAuth login flow.
///
/// Spawns a tokio task that:
/// 1. Binds the local HTTP server on port 1455
/// 2. Opens the browser to the authorize URL
/// 3. Awaits the callback with a 5-minute timeout
/// 4. Exchanges the authorization code for tokens
/// 5. Persists tokens to auth.json
///
/// Returns a Promise<NapiCodexTokens> that resolves when the flow completes.
///
/// Rule [0]: codex_oauth_browser_login() is an async NAPI function that spawns
/// a tokio task to run browser_oauth_login().
#[napi]
pub async fn codex_oauth_browser_login() -> Result<NapiCodexTokens> {
    let tokens = browser_oauth_login()
        .await
        .map_err(|e| Error::from_reason(format!("Browser OAuth login failed: {e}")))?;

    Ok(NapiCodexTokens::from(tokens))
}

// ============================================================================
// Device Auth Login (Two-Phase)
// ============================================================================

/// Phase 1: Start device auth flow — request a device code.
///
/// Returns NapiDeviceAuthStartResult with user_code and verification_url
/// so the TUI can display them immediately. The TUI then calls
/// codex_oauth_device_login_poll() to wait for the user to authorize.
///
/// Rule [6]: Two-phase design — first returns user_code and verification_url
/// synchronously, then a separate async function handles the polling.
#[napi]
pub async fn codex_oauth_device_login_start() -> Result<NapiDeviceAuthStartResult> {
    let device_code = request_device_code(CODEX_ISSUER)
        .await
        .map_err(|e| Error::from_reason(format!("Device auth start failed: {e}")))?;

    let verification_url = format!("{CODEX_ISSUER}/codex/device");

    Ok(NapiDeviceAuthStartResult {
        user_code: device_code.user_code,
        verification_url,
        device_auth_id: device_code.device_auth_id,
        interval: device_code.interval as f64,
    })
}

/// Phase 2: Poll for device auth completion.
///
/// Polls the device token endpoint at the given interval until the user
/// authorizes, the code expires, or a terminal error occurs.
///
/// On success: exchanges the authorization code for tokens, extracts
/// account_id from the JWT, persists to auth.json, and returns NapiCodexTokens.
///
/// Rule [1]: Returns a Promise<NapiCodexTokens> that resolves when polling completes.
#[napi]
pub async fn codex_oauth_device_login_poll(
    device_auth_id: String,
    interval: f64,
) -> Result<NapiCodexTokens> {
    use codelet_providers::codex::codex_device_auth::DeviceCodeResponse;

    let device_code = DeviceCodeResponse {
        device_auth_id,
        user_code: String::new(), // Not needed for polling
        interval: interval as u64,
    };

    let poll_config = PollConfig {
        issuer_url: CODEX_ISSUER,
        timeout_ms: OAUTH_TIMEOUT_MS,
        poll_interval_override_ms: None,
        slow_down_increment_override_ms: None,
    };

    let poll_result = poll_device_token(&poll_config, &device_code)
        .await
        .map_err(|e| Error::from_reason(format!("Device auth polling failed: {e}")))?;

    let (authorization_code, code_verifier) = match poll_result {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => (authorization_code, code_verifier),
        PollResult::TerminalError { error } => {
            return Err(Error::from_reason(format!("Device auth failed: {error}")));
        }
    };

    // Exchange authorization_code for tokens (no redirect_uri for device auth)
    let token_response = exchange_authorization_code(
        CODEX_ISSUER,
        &authorization_code,
        &code_verifier,
        None, // Device auth never uses redirect_uri
    )
    .await
    .map_err(|e| Error::from_reason(format!("Device auth token exchange failed: {e}")))?;

    build_and_persist_tokens(&token_response, "Failed to persist device auth tokens")
}

// ============================================================================
// Token Refresh
// ============================================================================

/// Refresh an access token using a refresh_token.
///
/// Calls refresh_access_token(), extracts account_id from the new JWT,
/// persists the refreshed tokens to auth.json, and returns NapiCodexTokens.
///
/// Rule [2]: Async NAPI function that calls refresh_access_token() and
/// returns refreshed NapiCodexTokens.
#[napi]
pub async fn codex_oauth_refresh_token(refresh_token: String) -> Result<NapiCodexTokens> {
    let token_response = refresh_access_token(&refresh_token)
        .await
        .map_err(|e| Error::from_reason(format!("Token refresh failed: {e}")))?;

    build_and_persist_tokens(&token_response, "Failed to persist refreshed tokens")
}

// ============================================================================
// Token Retrieval
// ============================================================================

/// Read stored tokens from auth.json.
///
/// Returns NapiCodexTokens if tokens exist, or null if no auth.json is found.
/// This is a synchronous function — no network calls needed.
///
/// Rule [3]: Synchronous NAPI function that reads auth.json via
/// read_codex_auth() and returns NapiCodexTokens or null.
#[napi]
pub fn codex_oauth_get_tokens() -> Result<Option<NapiCodexTokens>> {
    let auth = read_codex_auth()
        .map_err(|e| Error::from_reason(format!("Failed to read auth.json: {e}")))?;

    match auth {
        Some(auth_json) => match auth_json.tokens {
            Some(tokens) => Ok(Some(NapiCodexTokens::from(tokens))),
            None => Ok(None),
        },
        None => Ok(None),
    }
}

/// Clear stored OAuth tokens from auth.json (disconnect Codex OAuth).
///
/// Removes the `tokens` field from auth.json while preserving any cached
/// OPENAI_API_KEY. Used by the TUI when the user presses 'd' on the
/// Codex provider to disconnect their ChatGPT OAuth session.
#[napi]
pub fn codex_oauth_clear_tokens() -> Result<()> {
    let auth = read_codex_auth()
        .map_err(|e| Error::from_reason(format!("Failed to read auth.json: {e}")))?;

    match auth {
        Some(mut auth_json) => {
            auth_json.tokens = None;
            write_codex_auth(&auth_json)
                .map_err(|e| Error::from_reason(format!("Failed to write auth.json: {e}")))?;
            Ok(())
        }
        None => Ok(()), // No auth file, nothing to clear
    }
}
