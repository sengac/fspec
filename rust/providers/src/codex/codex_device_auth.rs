//! Device Authorization Flow for Headless Environments (PROV-014)
//!
//! Implements the device authorization grant (RFC 8628) for environments
//! where a browser can't be opened (SSH, containers, headless servers).
//!
//! Flow:
//! 1. POST to {ISSUER}/api/accounts/deviceauth/usercode → device_auth_id + user_code + interval
//! 2. Display user_code and verification URL to user
//! 3. Poll {ISSUER}/api/accounts/deviceauth/token at interval
//! 4. On success: authorization_code + code_verifier returned
//! 5. Exchange authorization_code for tokens at {ISSUER}/oauth/token (no redirect_uri)
//! 6. Persist tokens to auth.json

use anyhow::{anyhow, Result};
use serde::Deserialize;
use tracing::debug;

use super::codex_auth::{write_codex_auth, CodexAuthJson, CodexTokens};
use super::codex_oauth::{exchange_authorization_code, extract_account_id, CODEX_CLIENT_ID};

/// Response from the device authorization usercode endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_auth_id: String,
    pub user_code: String,
    pub interval: u64,
}

/// Result of a single poll to the device token endpoint
#[derive(Debug, Clone)]
pub enum PollResult {
    /// User completed authorization — contains the authorization code and verifier
    Success {
        authorization_code: String,
        code_verifier: String,
    },
    /// Terminal error — stop polling (expired_token, access_denied, etc.)
    TerminalError { error: String },
}

/// Type alias for the display callback used in device auth flows
pub type DisplayCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Configuration for `device_auth_login` to support both production and test use.
pub struct DeviceAuthConfig {
    /// The issuer URL (e.g. "https://auth.openai.com" or a wiremock URL)
    pub issuer_url: String,
    /// Overall timeout in milliseconds for the entire device auth flow
    pub timeout_ms: u64,
    /// Optional override for the polling interval in ms (tests use short intervals)
    pub poll_interval_override_ms: Option<u64>,
    /// Optional override for the slow_down backoff increment in ms.
    /// Production uses `SLOW_DOWN_INCREMENT_MS` (5000ms per RFC 8628 §3.5).
    /// Tests use small values for fast, deterministic timing assertions.
    pub slow_down_increment_override_ms: Option<u64>,
    /// Optional callback to display user_code and verification URL
    /// (default: prints to stderr)
    pub display_fn: Option<DisplayCallback>,
}

/// Configuration for polling the device token endpoint.
///
/// Bundles the polling parameters to reduce the argument count of
/// `poll_device_token` and improve call-site readability.
pub struct PollConfig<'a> {
    /// The issuer URL (e.g. "https://auth.openai.com" or a wiremock URL)
    pub issuer_url: &'a str,
    /// Overall timeout in milliseconds for polling
    pub timeout_ms: u64,
    /// Optional override for the polling interval in ms (tests use short intervals)
    pub poll_interval_override_ms: Option<u64>,
    /// Optional override for the slow_down backoff increment in ms.
    /// Production uses `SLOW_DOWN_INCREMENT_MS` (5000ms per RFC 8628 §3.5).
    /// Tests use small values for fast, deterministic timing assertions.
    pub slow_down_increment_override_ms: Option<u64>,
}

/// Slow-down backoff increment per RFC 8628 Section 3.5 (5 seconds)
const SLOW_DOWN_INCREMENT_MS: u64 = 5_000;

/// Response shape for the device token polling endpoint.
///
/// The endpoint returns EITHER:
/// - `{ "error": "authorization_pending" | "slow_down" | "expired_token" | "access_denied" }`
/// - `{ "authorization_code": "...", "code_verifier": "..." }`
#[derive(Debug, Deserialize)]
struct DevicePollResponse {
    /// Present when the endpoint returns an error status
    error: Option<String>,
    /// Present on successful authorization
    authorization_code: Option<String>,
    /// Present on successful authorization
    code_verifier: Option<String>,
}

/// Request a device code from the usercode endpoint.
///
/// POST to {issuer_url}/api/accounts/deviceauth/usercode with client_id.
pub async fn request_device_code(issuer_url: &str) -> Result<DeviceCodeResponse> {
    let url = format!("{issuer_url}/api/accounts/deviceauth/usercode");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[("client_id", CODEX_CLIENT_ID)])
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to device auth endpoint: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Device auth usercode request failed with status {status}: {body}"
        ));
    }

    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse device code response: {e}"))?;

    Ok(device_code)
}

/// Poll the device token endpoint until authorization completes or a terminal error occurs.
///
/// Handles: authorization_pending (continue), slow_down (increase interval by 5s),
/// expired_token/access_denied (terminal errors).
pub async fn poll_device_token(
    config: &PollConfig<'_>,
    device_code: &DeviceCodeResponse,
) -> Result<PollResult> {
    let url = format!("{}/api/accounts/deviceauth/token", config.issuer_url);
    let client = reqwest::Client::new();

    // Use override interval if provided (tests), otherwise use server-provided interval
    let base_interval_ms = config
        .poll_interval_override_ms
        .unwrap_or(device_code.interval * 1000);
    let mut current_interval_ms = base_interval_ms;

    let slow_down_increment = config
        .slow_down_increment_override_ms
        .unwrap_or(SLOW_DOWN_INCREMENT_MS);

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        async {
            loop {
                let response = client
                    .post(&url)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .form(&[("device_auth_id", &device_code.device_auth_id)])
                    .send()
                    .await
                    .map_err(|e| anyhow!("Polling request failed: {e}"))?;

                let poll_response: DevicePollResponse = response
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse poll response: {e}"))?;

                // Check for error responses
                if let Some(error) = &poll_response.error {
                    match error.as_str() {
                        "authorization_pending" => {
                            // Continue polling after waiting
                            tokio::time::sleep(std::time::Duration::from_millis(
                                current_interval_ms,
                            ))
                            .await;
                            continue;
                        }
                        "slow_down" => {
                            // RFC 8628 Section 3.5: increase interval by 5 seconds
                            current_interval_ms += slow_down_increment;
                            tokio::time::sleep(std::time::Duration::from_millis(
                                current_interval_ms,
                            ))
                            .await;
                            continue;
                        }
                        "expired_token" => {
                            return Ok(PollResult::TerminalError {
                                error: "Device code has expired. Please restart the login flow."
                                    .to_string(),
                            });
                        }
                        "access_denied" => {
                            return Ok(PollResult::TerminalError {
                                error: "User denied authorization (access_denied).".to_string(),
                            });
                        }
                        other => {
                            return Ok(PollResult::TerminalError {
                                error: format!("Device auth polling returned error: {other}"),
                            });
                        }
                    }
                }

                // Success: authorization_code and code_verifier should be present
                match (poll_response.authorization_code, poll_response.code_verifier) {
                    (Some(authorization_code), Some(code_verifier)) => {
                        return Ok(PollResult::Success {
                            authorization_code,
                            code_verifier,
                        });
                    }
                    _ => {
                        return Err(anyhow!(
                            "Unexpected poll response: no error and no authorization_code/code_verifier"
                        ));
                    }
                }
            }
        },
    )
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(anyhow!(
            "Device auth polling timed out after {}ms",
            config.timeout_ms
        )),
    }
}

/// Orchestrate the full device authorization login flow.
///
/// 1. Request device code
/// 2. Display user_code and verification URL
/// 3. Poll for authorization
/// 4. Exchange authorization_code for tokens (no redirect_uri)
/// 5. Extract account_id from JWT
/// 6. Persist tokens to auth.json
/// 7. Return CodexTokens
pub async fn device_auth_login(config: DeviceAuthConfig) -> Result<CodexTokens> {
    // Step 1: Request device code
    let device_code = request_device_code(&config.issuer_url).await?;

    // Step 2: Display user_code and verification URL
    let verification_url = format!("{}/codex/device", config.issuer_url);
    if let Some(display_fn) = &config.display_fn {
        display_fn(&device_code.user_code, &verification_url);
    } else {
        debug!(
            "\nTo authenticate, visit: {}\nEnter code: {}\n",
            verification_url, device_code.user_code
        );
    }

    // Step 3: Poll for authorization
    let poll_config = PollConfig {
        issuer_url: &config.issuer_url,
        timeout_ms: config.timeout_ms,
        poll_interval_override_ms: config.poll_interval_override_ms,
        slow_down_increment_override_ms: config.slow_down_increment_override_ms,
    };
    let poll_result = poll_device_token(&poll_config, &device_code).await?;

    let (authorization_code, code_verifier) = match poll_result {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => (authorization_code, code_verifier),
        PollResult::TerminalError { error } => {
            return Err(anyhow!("Device auth failed: {error}"));
        }
    };

    // Step 4: Exchange authorization_code for tokens (no redirect_uri for device auth)
    let token_response = exchange_authorization_code(
        &config.issuer_url,
        &authorization_code,
        &code_verifier,
        None, // Device auth never uses redirect_uri
    )
    .await?;

    // Step 5: Extract account_id from JWT
    let account_id = extract_account_id(
        Some(&token_response.id_token),
        Some(&token_response.access_token),
    )
    .ok_or_else(|| anyhow!("Failed to extract account_id from token response"))?;

    // Step 6: Build CodexTokens
    let tokens = CodexTokens {
        id_token: token_response.id_token,
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        account_id,
    };

    // Step 7: Persist to auth.json
    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(tokens.clone()),
        last_refresh: None,
    };
    write_codex_auth(&auth)?;

    Ok(tokens)
}
