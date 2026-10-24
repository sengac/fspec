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
    let url = format!("{}/api/accounts/deviceauth/token", config.issuer_url);
    let mut interval = config
        .poll_interval_override_ms
        .unwrap_or(device_code.interval.saturating_mul(1000));
    let increment = config.slow_down_increment_override_ms.unwrap_or(5000);
    tokio::time::timeout(std::time::Duration::from_millis(config.timeout_ms), async {
        loop {
            let response: DevicePollResponse = client
                .post(&url)
                .form(&[("device_auth_id", &device_code.device_auth_id)])
                .send()
                .await
                .map_err(|error| anyhow!("Polling request failed: {error}"))?
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
                            error: format!("Device auth polling returned error: {error}"),
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
