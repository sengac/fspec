use codelet_providers::codex::codex_auth::{
    clear_codex_oauth, read_codex_auth, refresh_codex_tokens, CodexTokens,
};
use codelet_providers::codex::codex_device_auth::{complete_device_login, start_device_login};
use codelet_providers::codex::codex_oauth::CODEX_ISSUER;
use codelet_providers::codex::codex_oauth_server::browser_oauth_login;
use napi::bindgen_prelude::*;

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

#[napi(object)]
pub struct NapiDeviceAuthStartResult {
    pub user_code: String,
    pub verification_url: String,
    pub device_auth_id: String,
    pub interval: f64,
}

#[napi]
pub async fn codex_oauth_browser_login() -> Result<NapiCodexTokens> {
    browser_oauth_login()
        .await
        .map(Into::into)
        .map_err(|error| Error::from_reason(format!("Browser OAuth login failed: {error}")))
}

#[napi]
pub async fn codex_oauth_device_login_start() -> Result<NapiDeviceAuthStartResult> {
    let code = start_device_login()
        .await
        .map_err(|error| Error::from_reason(format!("Device auth start failed: {error}")))?;
    Ok(NapiDeviceAuthStartResult {
        user_code: code.user_code,
        verification_url: format!("{CODEX_ISSUER}/codex/device"),
        device_auth_id: code.device_auth_id,
        interval: code.interval as f64,
    })
}

#[napi]
pub async fn codex_oauth_device_login_poll(
    device_auth_id: String,
    _interval: f64,
) -> Result<NapiCodexTokens> {
    complete_device_login(&device_auth_id)
        .await
        .map(Into::into)
        .map_err(|error| Error::from_reason(format!("Device auth failed: {error}")))
}

#[napi]
pub async fn codex_oauth_refresh_token(refresh_token: String) -> Result<NapiCodexTokens> {
    refresh_codex_tokens(&refresh_token)
        .await
        .map(Into::into)
        .map_err(|error| Error::from_reason(format!("Token refresh failed: {error}")))
}

#[napi]
pub fn codex_oauth_get_tokens() -> Result<Option<NapiCodexTokens>> {
    read_codex_auth()
        .map(|auth| auth.and_then(|auth| auth.tokens).map(Into::into))
        .map_err(|error| Error::from_reason(format!("Failed to read Codex credentials: {error}")))
}

#[napi]
pub fn codex_oauth_clear_tokens() -> Result<()> {
    clear_codex_oauth()
        .map_err(|error| Error::from_reason(format!("Failed to clear Codex credentials: {error}")))
}
