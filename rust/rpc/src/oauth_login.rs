//! PROV-113 — Anthropic + Codex OAuth login wiring (browser, headless, device).
//!
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! `FspecService::oauth_browser_login` / `oauth_headless_start` /
//! `oauth_headless_complete` / `oauth_device_start` / `oauth_device_poll`
//! delegate here. Each function forwards to the SAME `codelet_providers`
//! primitives the napi `*_oauth.rs` wrappers use (browser local-server login,
//! claude headless start/complete, codex device start/poll), so the Rust
//! frontend gets a real, napi-direct login WITHOUT a `codelet-napi` dependency
//! (rpc → providers is allowed; rpc → napi is forbidden) and WITHOUT routing
//! through `SessionManagerHandle`/core.
//!
//! All browser/device flows persist tokens to disk inside the providers-layer
//! server modules (`claude_oauth_server`, `codex_oauth_server`,
//! `codex_device_auth`); the headless-complete path persists here. Errors are
//! returned as `String` (the tarpc `Result<_, String>` shape); the frontend
//! swallows them so no RPC/method name ever leaks into the UI.

use codelet_providers::claude_auth::{write_claude_auth, ClaudeAuthJson};
use codelet_providers::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code as claude_exchange_code,
    parse_authorization_code, CLAUDE_TOKEN_ENDPOINT_BASE,
};
use codelet_providers::claude_oauth_server::claude_browser_oauth_login;
use codelet_providers::codex::codex_auth::{write_codex_auth, CodexAuthJson, CodexTokens};
use codelet_providers::codex::codex_device_auth::{
    poll_device_token, request_device_code, DeviceCodeResponse, PollConfig, PollResult,
};
use codelet_providers::codex::codex_oauth::{
    exchange_authorization_code as codex_exchange_code, extract_account_id, CODEX_ISSUER,
    OAUTH_TIMEOUT_MS,
};
use codelet_providers::codex::codex_oauth_server::browser_oauth_login as codex_browser_login;
use codelet_providers::oauth_crypto::generate_pkce;
use codelet_rpc_types::{OAuthDeviceStart, OAuthHeadlessStart};

/// Run the browser OAuth login for `provider_id` to completion (the providers
/// layer binds a local HTTP server, opens the browser, awaits the callback and
/// persists the tokens). Resolves `Ok(())` only when tokens were obtained.
pub async fn browser_login(provider_id: &str) -> Result<(), String> {
    match provider_id {
        "anthropic" => claude_browser_oauth_login()
            .await
            .map(|_| ())
            .map_err(|e| format!("login failed: {e}")),
        "codex" => codex_browser_login()
            .await
            .map(|_| ())
            .map_err(|e| format!("login failed: {e}")),
        other => Err(format!("browser login unsupported for {other}")),
    }
}

/// Phase 1 of the anthropic headless flow: generate PKCE + the authorize URL
/// the user must visit. Synchronous (no network) so the TUI can show the URL
/// immediately; the `pkce_verifier` is round-tripped to `headless_complete`.
pub fn headless_start(provider_id: &str) -> Result<OAuthHeadlessStart, String> {
    match provider_id {
        "anthropic" => {
            let pkce = generate_pkce();
            let authorize_url = build_authorize_url(&pkce);
            Ok(OAuthHeadlessStart {
                authorize_url,
                pkce_verifier: pkce.verifier,
            })
        }
        other => Err(format!("headless login unsupported for {other}")),
    }
}

/// Phase 2 of the anthropic headless flow: validate the pasted `code#state`
/// against the PKCE verifier, exchange the code for tokens, and persist.
pub async fn headless_complete(
    provider_id: &str,
    code_with_state: &str,
    pkce_verifier: &str,
) -> Result<(), String> {
    match provider_id {
        "anthropic" => {
            let (code, maybe_state) = parse_authorization_code(code_with_state);
            let state = maybe_state
                .ok_or_else(|| "missing state — code must be 'code#state'".to_string())?;
            if state != pkce_verifier {
                return Err("CSRF validation failed — state mismatch".to_string());
            }
            let token_response =
                claude_exchange_code(CLAUDE_TOKEN_ENDPOINT_BASE, &code, &state, pkce_verifier)
                    .await
                    .map_err(|e| format!("token exchange failed: {e}"))?;
            let auth = ClaudeAuthJson {
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                expires: calculate_expiry(token_response.expires_in),
            };
            write_claude_auth(&auth)
                .await
                .map_err(|e| format!("persist failed: {e}"))
        }
        other => Err(format!("headless login unsupported for {other}")),
    }
}

/// Phase 1 of the codex device flow: request a device code and return the
/// user-facing `user_code` + `verification_url` plus the `device_auth_id` /
/// `interval` the follow-up poll needs.
pub async fn device_start(provider_id: &str) -> Result<OAuthDeviceStart, String> {
    match provider_id {
        "codex" => {
            let dc = request_device_code(CODEX_ISSUER)
                .await
                .map_err(|e| format!("device start failed: {e}"))?;
            Ok(OAuthDeviceStart {
                user_code: dc.user_code,
                verification_url: format!("{CODEX_ISSUER}/codex/device"),
                device_auth_id: dc.device_auth_id,
                interval: dc.interval,
            })
        }
        other => Err(format!("device login unsupported for {other}")),
    }
}

/// Phase 2 of the codex device flow: poll the device token endpoint until the
/// user authorizes (or a terminal error), then exchange the authorization code
/// for tokens and persist them.
pub async fn device_poll(
    provider_id: &str,
    device_auth_id: String,
    interval: u64,
) -> Result<(), String> {
    match provider_id {
        // PROV-114: github-copilot reuses the shared device-poll entry point
        // but drives its own providers-direct poll + persist.
        "github-copilot" => crate::oauth_copilot::device_poll(device_auth_id, interval).await,
        "codex" => {
            let device_code = DeviceCodeResponse {
                device_auth_id,
                user_code: String::new(),
                interval,
            };
            let poll_config = PollConfig {
                issuer_url: CODEX_ISSUER,
                timeout_ms: OAUTH_TIMEOUT_MS,
                poll_interval_override_ms: None,
                slow_down_increment_override_ms: None,
            };
            let (authorization_code, code_verifier) =
                match poll_device_token(&poll_config, &device_code)
                    .await
                    .map_err(|e| format!("device poll failed: {e}"))?
                {
                    PollResult::Success {
                        authorization_code,
                        code_verifier,
                    } => (authorization_code, code_verifier),
                    PollResult::TerminalError { error } => {
                        return Err(format!("device auth failed: {error}"));
                    }
                };
            let token_response =
                codex_exchange_code(CODEX_ISSUER, &authorization_code, &code_verifier, None)
                    .await
                    .map_err(|e| format!("token exchange failed: {e}"))?;
            let account_id = extract_account_id(
                Some(&token_response.id_token),
                Some(&token_response.access_token),
            )
            .ok_or_else(|| "failed to extract account_id".to_string())?;
            let tokens = CodexTokens {
                id_token: token_response.id_token,
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                account_id,
            };
            let auth = CodexAuthJson {
                openai_api_key: None,
                tokens: Some(tokens),
                last_refresh: None,
            };
            write_codex_auth(&auth).map_err(|e| format!("persist failed: {e}"))
        }
        other => Err(format!("device login unsupported for {other}")),
    }
}
