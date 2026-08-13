//! GitHub Copilot OAuth Device Flow — token polling loop (PROV-054).
//!
//! Extracted from `oauth.rs` to keep each file within the 300-line SoC
//! budget (PROV-053 rule 21). Implements the full polling state machine
//! with `authorization_pending` / `slow_down` handling per RFC 8628 §3.5.

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::copilot::oauth_types::{
    CopilotDeviceCodeResponse, CopilotPollConfig, CopilotPollResult,
    AUTHORIZATION_PENDING_SAFETY_MARGIN_MS, COPILOT_CLIENT_ID, SLOW_DOWN_INCREMENT_MS,
};

/// Internal: response shape for the device token polling endpoint.
///
/// The endpoint returns EITHER:
/// - `{ "error": "authorization_pending" | "slow_down" | "expired_token" | "access_denied", "interval"?: u64 }`
/// - `{ "access_token": "...", "token_type": "bearer", "scope": "..." }`
#[derive(Debug, Deserialize)]
struct DevicePollResponse {
    /// Present when the endpoint returns an error
    error: Option<String>,
    /// Optional server-provided polling interval (slow_down responses)
    interval: Option<u64>,
    /// Present on successful authorization
    access_token: Option<String>,
}

/// Poll `{host_url}/login/oauth/access_token` until authorization completes
/// or a terminal error occurs.
///
/// Handles:
/// - `authorization_pending` → sleep `(interval + 3s safety margin)` then retry
/// - `slow_down`             → adopt server-provided interval + 5s increment, then retry
/// - `expired_token`         → terminal error
/// - `access_denied`         → terminal error
pub async fn poll_device_token(
    config: &CopilotPollConfig<'_>,
    device_code: &CopilotDeviceCodeResponse,
) -> Result<CopilotPollResult> {
    let url = format!("{}/login/oauth/access_token", config.host_url);
    let client = reqwest::Client::new();

    // Use override interval if provided (tests), otherwise use server-provided interval (in seconds → ms)
    let base_interval_ms = config
        .poll_interval_override_ms
        .unwrap_or(device_code.interval * 1000);
    let mut current_interval_ms = base_interval_ms;

    let safety_margin_ms = config
        .authorization_pending_safety_margin_override_ms
        .unwrap_or(AUTHORIZATION_PENDING_SAFETY_MARGIN_MS);

    let slow_down_increment = config
        .slow_down_increment_override_ms
        .unwrap_or(SLOW_DOWN_INCREMENT_MS);

    let result = tokio::time::timeout(std::time::Duration::from_millis(config.timeout_ms), async {
        loop {
            let response = client
                .post(&url)
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", COPILOT_CLIENT_ID),
                    ("device_code", device_code.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
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
                        // Sleep current interval + safety margin per PROV-054 Rule 5
                        let sleep_ms = current_interval_ms + safety_margin_ms;
                        tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                        continue;
                    }
                    "slow_down" => {
                        // RFC 8628 §3.5: adopt server-provided interval (if any) and add 5s increment
                        if let Some(server_interval_secs) = poll_response.interval {
                            // Server-provided interval is in seconds; convert to ms but
                            // preserve test override behaviour: if override is set, scale
                            // proportionally to keep tests fast.
                            let server_interval_ms = if config.poll_interval_override_ms.is_some() {
                                // Use the existing scaled interval base and just add the increment
                                current_interval_ms
                            } else {
                                server_interval_secs * 1000
                            };
                            current_interval_ms = server_interval_ms;
                        }
                        current_interval_ms += slow_down_increment;
                        tokio::time::sleep(std::time::Duration::from_millis(current_interval_ms))
                            .await;
                        continue;
                    }
                    "expired_token" => {
                        return Ok(CopilotPollResult::TerminalError {
                            error: "Device code has expired. Please restart the login flow."
                                .to_string(),
                        });
                    }
                    "access_denied" => {
                        return Ok(CopilotPollResult::TerminalError {
                            error: "User denied authorization (access_denied).".to_string(),
                        });
                    }
                    other => {
                        return Ok(CopilotPollResult::TerminalError {
                            error: format!("Device auth polling returned error: {other}"),
                        });
                    }
                }
            }

            // Success: access_token should be present
            match poll_response.access_token {
                Some(access_token) => {
                    return Ok(CopilotPollResult::Success { access_token });
                }
                None => {
                    return Err(anyhow!(
                        "Unexpected poll response: no error and no access_token"
                    ));
                }
            }
        }
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(anyhow!(
            "Device auth polling timed out after {}ms",
            config.timeout_ms
        )),
    }
}
