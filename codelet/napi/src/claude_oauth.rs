//! NAPI Bindings for Claude OAuth Flows (PROV-024)
//!
//! Exposes the Rust OAuth flows (browser login, headless login, token refresh,
//! token retrieval, token clearing) to the TypeScript TUI layer via NAPI bindings.
//!
//! Architecture:
//! - claude_oauth_browser_login() → async, spawns claude_browser_oauth_login()
//! - claude_oauth_headless_start() → sync, returns authorize_url + pkce_verifier
//! - claude_oauth_headless_complete() → async, validates state, exchanges code, persists tokens
//! - claude_oauth_refresh_token() → async, refreshes and persists tokens
//! - claude_oauth_get_tokens() → async (tokio::fs), reads claude_auth.json
//! - claude_oauth_clear_tokens() → async (tokio::fs), deletes claude_auth.json
//!
//! Key differences from codex_oauth.rs (PROV-015):
//! - No id_token, no account_id, no JWT extraction — simpler token struct
//! - claude_auth is async (tokio::fs) → get_tokens and clear_tokens are async
//! - No device polling — headless is start+complete instead of start+poll
//! - Token exchange uses JSON POST (not form-encoded)

use codelet_providers::claude_auth::{
    get_claude_auth_path, read_claude_auth, write_claude_auth, ClaudeAuthJson,
};
use codelet_providers::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code,
    parse_authorization_code, refresh_access_token_at, ClaudeTokenResponse, CLAUDE_TOKEN_ENDPOINT,
};
use codelet_providers::claude_oauth_server::claude_browser_oauth_login;
use codelet_providers::oauth_crypto::generate_pkce;
use napi::bindgen_prelude::*;

// ============================================================================
// NAPI Object Structs
// ============================================================================

/// Claude OAuth tokens exposed to TypeScript.
///
/// Rule [6]: Maps to ClaudeAuthJson from claude_auth.rs.
/// Fields: access_token, refresh_token, expires (f64 for JS compatibility).
///
/// Unlike Codex, there is no id_token or account_id.
#[napi(object)]
pub struct NapiClaudeTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiry timestamp in milliseconds since Unix epoch (f64 for JS Number)
    pub expires: f64,
}

impl From<ClaudeAuthJson> for NapiClaudeTokens {
    fn from(auth: ClaudeAuthJson) -> Self {
        Self {
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
            expires: auth.expires as f64,
        }
    }
}

/// Result from headless login start — returned synchronously so the TUI
/// can display the authorize URL immediately.
///
/// Architecture note [1]: authorize_url and pkce_verifier. The verifier is
/// returned to TypeScript so it can be passed back to headless_complete().
#[napi(object)]
pub struct NapiClaudeHeadlessStartResult {
    pub authorize_url: String,
    pub pkce_verifier: String,
}

// ============================================================================
// Browser OAuth Login
// ============================================================================

/// Start the Claude browser OAuth login flow.
///
/// Spawns a tokio task that:
/// 1. Binds a local HTTP server on an ephemeral port
/// 2. Opens the browser to the authorize URL
/// 3. Shows a form for the user to paste their code#state
/// 4. Validates state, exchanges the authorization code for tokens
/// 5. Persists tokens to claude_auth.json
///
/// Returns a Promise<NapiClaudeTokens> that resolves when the flow completes.
///
/// Rule [0]: claude_oauth_browser_login() is an async NAPI function that spawns
/// a tokio task to run claude_browser_oauth_login().
#[napi]
pub async fn claude_oauth_browser_login() -> Result<NapiClaudeTokens> {
    let auth = claude_browser_oauth_login()
        .await
        .map_err(|e| Error::from_reason(format!("Claude browser OAuth login failed: {e}")))?;

    Ok(NapiClaudeTokens::from(auth))
}

// ============================================================================
// Shared Helpers
// ============================================================================

/// Derive the base URL for the token endpoint from the full constant.
///
/// `exchange_authorization_code` and `refresh_access_token_at` both take a
/// base URL and append `/v1/oauth/token` internally. This helper extracts
/// the base once rather than duplicating the strip+fallback everywhere.
fn claude_token_endpoint_base() -> &'static str {
    CLAUDE_TOKEN_ENDPOINT
        .strip_suffix("/v1/oauth/token")
        .unwrap_or("https://console.anthropic.com")
}

/// Build `ClaudeAuthJson` from a token response, persist to `claude_auth.json`,
/// and convert to `NapiClaudeTokens`.
///
/// Mirrors `build_and_persist_tokens` from `codex_oauth.rs` (PROV-015).
/// Used by both `headless_complete` and `refresh_token` to eliminate duplication.
async fn build_and_persist_tokens(
    token_response: ClaudeTokenResponse,
    error_context: &str,
) -> Result<NapiClaudeTokens> {
    let expires = calculate_expiry(token_response.expires_in);
    let auth = ClaudeAuthJson {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        expires,
    };

    write_claude_auth(&auth)
        .await
        .map_err(|e| Error::from_reason(format!("{error_context}: {e}")))?;

    Ok(NapiClaudeTokens::from(auth))
}

// ============================================================================
// Headless Login (Two-Phase)
// ============================================================================

/// Phase 1: Start headless login flow — generate PKCE and build authorize URL.
///
/// Returns NapiClaudeHeadlessStartResult with authorize_url and pkce_verifier
/// so the TUI can display the URL immediately. The TUI then:
/// 1. Shows the URL to the user
/// 2. Collects the pasted code#state string
/// 3. Calls claude_oauth_headless_complete() with the code and verifier
///
/// Rule [1]: Two-phase design keeps the NAPI boundary clean — no
/// CodeEntryFn callback needed.
#[napi]
pub fn claude_oauth_headless_start() -> NapiClaudeHeadlessStartResult {
    let pkce = generate_pkce();
    let authorize_url = build_authorize_url(&pkce);

    NapiClaudeHeadlessStartResult {
        authorize_url,
        pkce_verifier: pkce.verifier,
    }
}

/// Phase 2: Complete headless login — validate state, exchange code, persist tokens.
///
/// Receives the user-pasted code_with_state (in "code#state" format) and the
/// pkce_verifier from Phase 1. Validates that the state matches the verifier
/// (CSRF protection), exchanges the code for tokens, persists to claude_auth.json,
/// and returns NapiClaudeTokens.
///
/// Rule [1]: claude_oauth_headless_complete(code_with_state, pkce_verifier)
/// validates state, exchanges code, and returns NapiClaudeTokens.
#[napi]
pub async fn claude_oauth_headless_complete(
    code_with_state: String,
    pkce_verifier: String,
) -> Result<NapiClaudeTokens> {
    // Parse code#state format
    let (code, maybe_state) = parse_authorization_code(&code_with_state);

    let state = match maybe_state {
        Some(s) => s,
        None => {
            return Err(Error::from_reason(
                "Missing state in authorization code — code must be in 'code#state' format",
            ));
        }
    };

    // Validate state matches PKCE verifier (CSRF protection)
    if state != pkce_verifier {
        return Err(Error::from_reason(format!(
            "CSRF validation failed — state mismatch. Expected: {pkce_verifier}, Got: {state}"
        )));
    }

    // Exchange code for tokens
    let token_response = exchange_authorization_code(
        claude_token_endpoint_base(),
        &code,
        &state,
        &pkce_verifier,
    )
    .await
    .map_err(|e| Error::from_reason(format!("Token exchange failed: {e}")))?;

    build_and_persist_tokens(token_response, "Failed to persist tokens").await
}

// ============================================================================
// Token Refresh
// ============================================================================

/// Refresh an access token using a refresh_token.
///
/// Calls refresh_access_token_at(), persists the refreshed tokens to
/// claude_auth.json, and returns NapiClaudeTokens.
///
/// Rule [2]: Async NAPI function that calls refresh_access_token_at() and
/// returns NapiClaudeTokens with refreshed tokens persisted.
#[napi]
pub async fn claude_oauth_refresh_token(refresh_token: String) -> Result<NapiClaudeTokens> {
    let token_response = refresh_access_token_at(claude_token_endpoint_base(), &refresh_token)
        .await
        .map_err(|e| Error::from_reason(format!("Token refresh failed: {e}")))?;

    build_and_persist_tokens(token_response, "Failed to persist refreshed tokens").await
}

// ============================================================================
// Token Retrieval
// ============================================================================

/// Read stored tokens from claude_auth.json.
///
/// Returns NapiClaudeTokens if tokens exist, or null if no claude_auth.json
/// is found. This is an async function because claude_auth uses tokio::fs
/// (unlike Codex which is sync).
///
/// Rule [3]: Async NAPI function that reads claude_auth.json via
/// read_claude_auth() and returns NapiClaudeTokens or null.
#[napi]
pub async fn claude_oauth_get_tokens() -> Result<Option<NapiClaudeTokens>> {
    let auth = read_claude_auth()
        .await
        .map_err(|e| Error::from_reason(format!("Failed to read claude_auth.json: {e}")))?;

    match auth {
        Some(auth_json) => Ok(Some(NapiClaudeTokens::from(auth_json))),
        None => Ok(None),
    }
}

// ============================================================================
// Token Clearing
// ============================================================================

/// Clear stored OAuth tokens by deleting claude_auth.json.
///
/// Idempotent: returns Ok(()) even if the file doesn't exist.
/// Used by the TUI when the user disconnects their Claude subscription.
///
/// Rule [4]: Async NAPI function that deletes claude_auth.json for disconnect.
#[napi]
pub async fn claude_oauth_clear_tokens() -> Result<()> {
    let auth_path = get_claude_auth_path();

    match tokio::fs::remove_file(&auth_path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // Idempotent
        Err(e) => Err(Error::from_reason(format!(
            "Failed to clear claude_auth.json: {e}"
        ))),
    }
}
