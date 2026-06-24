//! Generic Device Code Flow (PROV-060)
//!
//! `DeviceCodeFlow<P: DeviceCodeProvider>` unifies the RFC 8628 device
//! authorization grant polling loops used by Copilot and Codex providers.

use anyhow::{anyhow, Result};

/// Provider-specific device code flow configuration.
///
/// Implementations supply the endpoint URLs, client_id, and post-processing
/// logic for the specific OAuth provider.
pub trait DeviceCodeProvider: Send + Sync {
    /// The device authorization endpoint URL.
    fn device_authorize_url(&self) -> String;

    /// The token polling endpoint URL.
    fn token_poll_url(&self) -> String;

    /// The OAuth client_id.
    fn client_id(&self) -> &str;

    /// Build the form body for the device code request.
    fn device_code_form_body(&self) -> Vec<(&str, String)>;

    /// Build the form body for the token poll request.
    fn token_poll_form_body(&self, device_code: &str) -> Vec<(String, String)>;

    /// Parse the device code response JSON into a `DeviceCodeResponse`.
    fn parse_device_code_response(&self, json: &serde_json::Value) -> Result<DeviceCodeResponse>;

    /// Parse the poll response JSON.
    fn parse_poll_response(&self, json: &serde_json::Value) -> PollResponse;
}

/// Generic device code response (provider-agnostic).
#[derive(Debug, Clone)]
pub struct DeviceCodeResponse {
    /// Opaque device code for polling
    pub device_code: String,
    /// Human-readable user code to display
    pub user_code: String,
    /// Verification URL for the user
    pub verification_uri: String,
    /// Server-suggested polling interval in seconds
    pub interval: u64,
}

/// Result of a single poll iteration.
#[derive(Debug, Clone)]
pub enum PollResponse {
    /// Continue polling (authorization_pending)
    Pending,
    /// Slow down polling (RFC 8628 §3.5)
    SlowDown,
    /// Terminal error — stop polling
    TerminalError(String),
    /// Success with the raw token JSON value
    Success(serde_json::Value),
}

/// Configuration for device code polling.
pub struct DeviceCodeFlowConfig {
    /// Overall timeout in milliseconds
    pub timeout_ms: u64,
    /// Optional override for the polling interval in ms (for tests)
    pub poll_interval_override_ms: Option<u64>,
    /// Optional override for the slow_down backoff increment in ms
    pub slow_down_increment_override_ms: Option<u64>,
}

/// Slow-down backoff increment per RFC 8628 §3.5 (5 seconds)
const SLOW_DOWN_INCREMENT_MS: u64 = 5_000;

/// Generic device code flow orchestrator.
pub struct DeviceCodeFlow<P: DeviceCodeProvider> {
    provider: P,
}

impl<P: DeviceCodeProvider> DeviceCodeFlow<P> {
    /// Create a new device code flow for the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Access the underlying provider (for tests/diagnostics).
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Request a device code from the provider.
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse> {
        let url = self.provider.device_authorize_url();
        let client = reqwest::Client::new();
        let form_body = self.provider.device_code_form_body();

        let response = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&form_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to connect to device auth endpoint: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|e| {
                tracing::debug!("Failed to read error response body: {e}");
                String::new()
            });
            return Err(anyhow!(
                "Device auth request failed with status {status}: {body}"
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse device code response: {e}"))?;

        self.provider.parse_device_code_response(&json)
    }

    /// Poll for token completion.
    pub async fn poll_for_token(
        &self,
        config: &DeviceCodeFlowConfig,
        device_code: &DeviceCodeResponse,
    ) -> Result<serde_json::Value> {
        let url = self.provider.token_poll_url();
        let client = reqwest::Client::new();

        let base_interval_ms = config
            .poll_interval_override_ms
            .unwrap_or(device_code.interval * 1000);
        let mut current_interval_ms = base_interval_ms;

        let slow_down_increment = config
            .slow_down_increment_override_ms
            .unwrap_or(SLOW_DOWN_INCREMENT_MS);

        let result =
            tokio::time::timeout(std::time::Duration::from_millis(config.timeout_ms), async {
                loop {
                    let form_body = self.provider.token_poll_form_body(&device_code.device_code);

                    let response = client
                        .post(&url)
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("Accept", "application/json")
                        .form(&form_body)
                        .send()
                        .await
                        .map_err(|e| anyhow!("Polling request failed: {e}"))?;

                    let json: serde_json::Value = response
                        .json()
                        .await
                        .map_err(|e| anyhow!("Failed to parse poll response: {e}"))?;

                    match self.provider.parse_poll_response(&json) {
                        PollResponse::Pending => {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                current_interval_ms,
                            ))
                            .await;
                        }
                        PollResponse::SlowDown => {
                            current_interval_ms += slow_down_increment;
                            tokio::time::sleep(std::time::Duration::from_millis(
                                current_interval_ms,
                            ))
                            .await;
                        }
                        PollResponse::TerminalError(msg) => {
                            return Err(anyhow!("{msg}"));
                        }
                        PollResponse::Success(value) => {
                            return Ok(value);
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
}
