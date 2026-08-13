//! GitHub Copilot OAuth Device Flow orchestrator (PROV-054).
//!
//! Implements the OAuth 2.0 device authorization grant (RFC 8628) for
//! authenticating with GitHub Copilot from the CLI without a browser.
//!
//! Flow (mirrors opencode-copilot.ts):
//! 1. POST to `{host}/login/device/code` → device_code + user_code + verification_uri + interval
//! 2. Display user_code and verification_uri to the user (via display_fn callback)
//! 3. Poll `{host}/login/oauth/access_token` at `(interval + 3s)` until success
//!    - On `authorization_pending`: continue polling
//!    - On `slow_down`: adopt server-provided interval + 5s increment per RFC 8628 §3.5
//! 4. Persist the resulting `access_token` to `~/.fspec/credentials/copilot_auth.json` (mode 0600)
//!
//! Two deployment types are supported:
//! - `GitHubCom` — `host = https://github.com`
//! - `Enterprise` — `host = https://<normalized-domain>`
//!
//! This module was split into focused sibling files to honor the 300-line
//! SoC budget (PROV-053 rule 21):
//!
//! - [`oauth_types`](super::oauth_types) — types, constants, configuration
//! - [`oauth_device_code`](super::oauth_device_code) — device-code
//!   request + enterprise-URL normalization
//! - [`oauth_polling`](super::oauth_polling) — token polling loop + RFC
//!   8628 §3.5 backoff
//!
//! Reference: opencode `copilot.ts` (normalizeDomain at line 15, polling
//! logic at line 80+)

use anyhow::{anyhow, Result};
use tracing::debug;

use crate::copilot::auth::{write_copilot_auth, CopilotAuthJson};
use crate::copilot::oauth_device_code::request_device_code;
use crate::copilot::oauth_polling::poll_device_token;
use crate::copilot::oauth_types::{
    CopilotDeploymentType, CopilotDeviceAuthConfig, CopilotPollConfig, CopilotPollResult,
};

/// Orchestrate the full Copilot device authorization login flow.
///
/// 1. Request device code
/// 2. Display user_code and verification URL via callback
/// 3. Poll for authorization
/// 4. Persist credentials to copilot_auth.json (mode 0600)
/// 5. Return the persisted CopilotAuthJson
pub async fn copilot_device_auth_login(config: CopilotDeviceAuthConfig) -> Result<CopilotAuthJson> {
    // Step 1: Request device code
    let device_code = request_device_code(&config.host_url).await?;

    // Step 2: Display user_code and verification URL
    if let Some(display_fn) = &config.display_fn {
        display_fn(&device_code.user_code, &device_code.verification_uri);
    } else {
        debug!(
            "\nTo authenticate, visit: {}\nEnter code: {}\n",
            device_code.verification_uri, device_code.user_code
        );
    }

    // Step 3: Poll for authorization
    let poll_config = CopilotPollConfig {
        host_url: &config.host_url,
        timeout_ms: config.timeout_ms,
        poll_interval_override_ms: config.poll_interval_override_ms,
        slow_down_increment_override_ms: config.slow_down_increment_override_ms,
        authorization_pending_safety_margin_override_ms: config
            .authorization_pending_safety_margin_override_ms,
    };
    let poll_result = poll_device_token(&poll_config, &device_code).await?;

    let access_token = match poll_result {
        CopilotPollResult::Success { access_token } => access_token,
        CopilotPollResult::TerminalError { error } => {
            return Err(anyhow!("Device auth failed: {error}"));
        }
    };

    // Step 4: Build CopilotAuthJson per PROV-057 (two-token model).
    // The token exchange to mint a short-lived Copilot token happens on the
    // first API call, not at login time.
    let enterprise_url = match &config.deployment_type {
        CopilotDeploymentType::GitHubCom => None,
        CopilotDeploymentType::Enterprise { host } => Some(host.clone()),
    };

    let auth = CopilotAuthJson::from_github_oauth_token(access_token, enterprise_url);

    // Step 5: Persist with mode 0600
    write_copilot_auth(&auth).await?;

    Ok(auth)
}
