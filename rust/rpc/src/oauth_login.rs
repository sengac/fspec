use codelet_providers::claude_auth::{write_claude_auth, ClaudeAuthJson};
use codelet_providers::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code as claude_exchange_code,
    parse_authorization_code, CLAUDE_TOKEN_ENDPOINT_BASE,
};
use codelet_providers::claude_oauth_server::claude_browser_oauth_login;
use codelet_providers::codex::codex_device_auth::{complete_device_login, start_device_login};
use codelet_providers::codex::codex_oauth::CODEX_ISSUER;
use codelet_providers::codex::codex_oauth_server::browser_oauth_login as codex_browser_login;
use codelet_providers::oauth_crypto::generate_pkce;
use codelet_rpc_types::{OAuthDeviceStart, OAuthHeadlessStart};

pub async fn browser_login(provider_id: &str) -> Result<(), String> {
    match provider_id {
        "anthropic" => claude_browser_oauth_login()
            .await
            .map(|_| ())
            .map_err(|error| format!("login failed: {error}")),
        "codex" => codex_browser_login()
            .await
            .map(|_| ())
            .map_err(|error| format!("login failed: {error}")),
        other => Err(format!("browser login unsupported for {other}")),
    }
}

pub fn headless_start(provider_id: &str) -> Result<OAuthHeadlessStart, String> {
    match provider_id {
        "anthropic" => {
            let pkce = generate_pkce();
            Ok(OAuthHeadlessStart {
                authorize_url: build_authorize_url(&pkce),
                pkce_verifier: pkce.verifier,
            })
        }
        other => Err(format!("headless login unsupported for {other}")),
    }
}

pub async fn headless_complete(
    provider_id: &str,
    code_with_state: &str,
    pkce_verifier: &str,
) -> Result<(), String> {
    match provider_id {
        "anthropic" => {
            let (code, maybe_state) = parse_authorization_code(code_with_state);
            let state = maybe_state
                .ok_or_else(|| "missing state — code must be 'code#state'".to_owned())?;
            if state != pkce_verifier {
                return Err("CSRF validation failed — state mismatch".to_owned());
            }
            let response =
                claude_exchange_code(CLAUDE_TOKEN_ENDPOINT_BASE, &code, &state, pkce_verifier)
                    .await
                    .map_err(|error| format!("token exchange failed: {error}"))?;
            write_claude_auth(&ClaudeAuthJson {
                access_token: response.access_token,
                refresh_token: response.refresh_token,
                expires: calculate_expiry(response.expires_in),
            })
            .await
            .map_err(|error| format!("persist failed: {error}"))
        }
        other => Err(format!("headless login unsupported for {other}")),
    }
}

pub async fn device_start(provider_id: &str) -> Result<OAuthDeviceStart, String> {
    match provider_id {
        "codex" => {
            let code = start_device_login()
                .await
                .map_err(|error| format!("device start failed: {error}"))?;
            Ok(OAuthDeviceStart {
                user_code: code.user_code,
                verification_url: format!("{CODEX_ISSUER}/codex/device"),
                device_auth_id: code.device_auth_id,
                interval: code.interval,
            })
        }
        other => Err(format!("device login unsupported for {other}")),
    }
}

pub async fn device_poll(
    provider_id: &str,
    device_auth_id: String,
    interval: u64,
) -> Result<(), String> {
    match provider_id {
        "github-copilot" => crate::oauth_copilot::device_poll(device_auth_id, interval).await,
        "codex" => complete_device_login(&device_auth_id)
            .await
            .map(|_| ())
            .map_err(|error| format!("device poll failed: {error}")),
        other => Err(format!("device login unsupported for {other}")),
    }
}
