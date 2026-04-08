//! GitHub Copilot OAuth Device Flow — device code request + domain
//! normalization (PROV-054).
//!
//! Extracted from `oauth.rs` to keep each file within the 300-line SoC
//! budget (PROV-053 rule 21).

use anyhow::{anyhow, Result};

use crate::copilot::oauth_types::{
    CopilotDeviceCodeResponse, COPILOT_CLIENT_ID, COPILOT_OAUTH_SCOPE,
};

/// Normalize a user-supplied Enterprise URL to a bare host.
///
/// Mirrors `copilot.ts:15 normalizeDomain`:
/// - Strip leading `https://` or `http://`
/// - Strip trailing `/`
/// - Return the bare domain
///
/// Examples:
/// - `https://ghe.example.com/` → `ghe.example.com`
/// - `http://ghe.example.com`   → `ghe.example.com`
/// - `ghe.example.com`          → `ghe.example.com`
pub fn normalize_enterprise_domain(input: &str) -> String {
    let stripped = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .unwrap_or(input);
    stripped.trim_end_matches('/').to_string()
}

/// Request a device code from `{host_url}/login/device/code`.
///
/// Posts `client_id` and `scope` as form-encoded parameters and parses the
/// JSON response into a `CopilotDeviceCodeResponse`.
pub async fn request_device_code(host_url: &str) -> Result<CopilotDeviceCodeResponse> {
    let url = format!("{host_url}/login/device/code");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", COPILOT_CLIENT_ID),
            ("scope", COPILOT_OAUTH_SCOPE),
        ])
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to device code endpoint: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Device code request failed with status {status}: {body}"
        ));
    }

    let device_code: CopilotDeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse device code response: {e}"))?;

    Ok(device_code)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/github-copilot-end-to-end-integration.feature
    //!
    //! PROV-057 Scenario: OAuth device flow uses the well-known Copilot
    //! client_id.

    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn copilot_client_id_constant_is_the_well_known_copilot_app_id() {
        // @step Given the Copilot OAuth device flow is invoked from the TUI
        // @step When the device-code request is sent to GitHub
        // @step Then the request body contains client_id "Iv1.b507a08c87ecfe98"
        assert_eq!(COPILOT_CLIENT_ID, "Iv1.b507a08c87ecfe98");
        // @step And the request body does not contain client_id "Ov23li8tweQw6odWQebz"
        assert_ne!(COPILOT_CLIENT_ID, "Ov23li8tweQw6odWQebz");
    }

    #[tokio::test]
    async fn device_code_request_form_body_carries_corrected_client_id() {
        // @step Given the Copilot OAuth device flow is invoked from the TUI
        let mock_server = MockServer::start().await;

        // @step When the device-code request is sent to GitHub
        // @step Then the request body contains client_id "Iv1.b507a08c87ecfe98"
        Mock::given(method("POST"))
            .and(path("/login/device/code"))
            .and(body_string_contains("client_id=Iv1.b507a08c87ecfe98"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "dc_prov_057",
                "user_code": "ABCD-9999",
                "verification_uri": "https://github.com/login/device",
                "interval": 5
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // @step And the request body does not contain client_id "Ov23li8tweQw6odWQebz"
        // (wiremock's expect(1) will fail the test if the matcher does not
        // fire — a body still carrying the opencode client_id would not
        // match `body_string_contains("client_id=Iv1.b507a08c87ecfe98")`
        // and wiremock would panic on drop.)

        let result = request_device_code(&mock_server.uri()).await;
        assert!(
            result.is_ok(),
            "device code request should succeed, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().device_code, "dc_prov_057");
    }
}
