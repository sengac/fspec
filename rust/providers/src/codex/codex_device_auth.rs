use anyhow::{anyhow, Result};
use serde::Deserialize;

use super::auth_lifecycle::LoginAttempt;
use super::codex_auth::CodexTokens;
use super::codex_oauth::{exchange_authorization_code, extract_account_id, CODEX_CLIENT_ID};

#[path = "device_poll.rs"]
mod polling;
pub use super::device_session::{complete_device_login, start_device_login};
pub use polling::poll_device_token;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_auth_id: String,
    pub user_code: String,
    pub interval: u64,
}

#[derive(Debug, Clone)]
pub enum PollResult {
    Success {
        authorization_code: String,
        code_verifier: String,
    },
    TerminalError {
        error: String,
    },
}

pub type DisplayCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

pub struct DeviceAuthConfig {
    pub issuer_url: String,
    pub timeout_ms: u64,
    pub poll_interval_override_ms: Option<u64>,
    pub slow_down_increment_override_ms: Option<u64>,
    pub display_fn: Option<DisplayCallback>,
}

pub struct PollConfig<'a> {
    pub issuer_url: &'a str,
    pub timeout_ms: u64,
    pub poll_interval_override_ms: Option<u64>,
    pub slow_down_increment_override_ms: Option<u64>,
}

pub async fn request_device_code(issuer_url: &str) -> Result<DeviceCodeResponse> {
    let response = reqwest::Client::new()
        .post(format!("{issuer_url}/api/accounts/deviceauth/usercode"))
        .form(&[("client_id", CODEX_CLIENT_ID)])
        .send()
        .await
        .map_err(|error| anyhow!("Failed to connect to device auth endpoint: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Device auth usercode request failed with status {status}: {body}"
        ));
    }
    response
        .json()
        .await
        .map_err(|error| anyhow!("Failed to parse device code response: {error}"))
}

pub async fn device_auth_login(config: DeviceAuthConfig) -> Result<CodexTokens> {
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_millis(config.timeout_ms);
    let attempt = LoginAttempt::begin_until(deadline).await?;
    tokio::select! {
        biased;
        () = attempt.cancelled() => Err(anyhow!("Login cancelled")),
        result = tokio::time::timeout_at(deadline, async {
            let device_code = request_device_code(&config.issuer_url).await?;
            let verification_url = format!("{}/codex/device", config.issuer_url);
            if let Some(display) = &config.display_fn {
                display(&device_code.user_code, &verification_url);
            } else {
                tracing::debug!("To authenticate, visit: {verification_url}; enter code: {}", device_code.user_code);
            }
            finish(&config, &device_code, &attempt).await
        }) => result.map_err(|_| anyhow!("Device login timed out"))?,
    }
}

pub(super) async fn finish(
    config: &DeviceAuthConfig,
    device_code: &DeviceCodeResponse,
    attempt: &LoginAttempt,
) -> Result<CodexTokens> {
    let poll_config = PollConfig {
        issuer_url: &config.issuer_url,
        timeout_ms: config.timeout_ms,
        poll_interval_override_ms: config.poll_interval_override_ms,
        slow_down_increment_override_ms: config.slow_down_increment_override_ms,
    };
    let (authorization_code, code_verifier) = match poll_device_token(&poll_config, device_code)
        .await?
    {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => (authorization_code, code_verifier),
        PollResult::TerminalError { error } => return Err(anyhow!("Device auth failed: {error}")),
    };
    let response = exchange_authorization_code(
        &config.issuer_url,
        &authorization_code,
        &code_verifier,
        None,
    )
    .await?;
    let account_id = extract_account_id(Some(&response.id_token), Some(&response.access_token))
        .ok_or_else(|| anyhow!("Failed to extract account_id from token response"))?;
    let tokens = CodexTokens {
        id_token: response.id_token,
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        account_id,
    };
    attempt
        .commit_with_expiry(&tokens, response.expires_in)
        .await?;
    Ok(tokens)
}
