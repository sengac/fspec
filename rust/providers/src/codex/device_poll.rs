use super::{DeviceCodeResponse, PollConfig, PollResult};
use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct DevicePollResponse {
    error: Option<String>,
    authorization_code: Option<String>,
    code_verifier: Option<String>,
}

pub async fn poll_device_token(
    config: &PollConfig<'_>,
    device_code: &DeviceCodeResponse,
) -> Result<PollResult> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/accounts/deviceauth/token",
        config.issuer_url.trim_end_matches('/')
    );
    let mut interval = config
        .poll_interval_override_ms
        .unwrap_or(device_code.interval.saturating_mul(1000));
    let increment = config.slow_down_increment_override_ms.unwrap_or(5000);
    tokio::time::timeout(std::time::Duration::from_millis(config.timeout_ms), async {
        loop {
            let response = client
                .post(&url)
                .json(&serde_json::json!({
                    "device_auth_id": device_code.device_auth_id,
                    "user_code": device_code.user_code,
                }))
                .send()
                .await
                .map_err(|error| anyhow!("Polling request failed: {error}"))?;
            let status = response.status();
            if matches!(
                status,
                reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND
            ) {
                tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
                continue;
            }
            if !status.is_success() {
                return Err(anyhow!("Device auth polling failed with status {status}"));
            }
            let response: DevicePollResponse = response
                .json()
                .await
                .map_err(|error| anyhow!("Failed to parse poll response: {error}"))?;
            if let Some(error) = response.error {
                match error.as_str() {
                    "authorization_pending" | "slow_down" => {
                        if error == "slow_down" {
                            interval = interval.saturating_add(increment);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
                        continue;
                    }
                    "expired_token" => {
                        return Ok(PollResult::TerminalError {
                            error: "Device code has expired. Please restart the login flow."
                                .to_owned(),
                        })
                    }
                    "access_denied" => {
                        return Ok(PollResult::TerminalError {
                            error: "User denied authorization (access_denied).".to_owned(),
                        })
                    }
                    _ => {
                        return Ok(PollResult::TerminalError {
                            error: "Device auth polling returned an unrecognized error".to_owned(),
                        })
                    }
                }
            }
            return match (response.authorization_code, response.code_verifier) {
                (Some(authorization_code), Some(code_verifier)) => Ok(PollResult::Success {
                    authorization_code,
                    code_verifier,
                }),
                _ => Err(anyhow!(
                    "Unexpected poll response: no error and no authorization_code/code_verifier"
                )),
            };
        }
    })
    .await
    .map_err(|_| {
        anyhow!(
            "Device auth polling timed out after {}ms",
            config.timeout_ms
        )
    })?
}
