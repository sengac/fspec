//! PROV-114 — GitHub Copilot OAuth device flow wiring (start + poll).
//!
//! Feature: spec/features/provider-settings-oauth-copilot-device.feature
//!
//! `FspecService::oauth_copilot_device_start` delegates to [`device_start`];
//! the shared `oauth_device_poll` delegates to [`device_poll`] when the
//! provider is `github-copilot`. Both forward to the SAME `codelet_providers`
//! copilot primitives the napi `copilot_oauth.rs` wrapper uses
//! (`request_device_code`, `poll_device_token`, `write_copilot_auth`,
//! `normalize_enterprise_domain`) so the Rust frontend gets a real,
//! providers-direct device login WITHOUT a `codelet-napi` dependency.
//!
//! The shared `OAuthDeviceStart` carries a single opaque `device_auth_id`, so
//! the host URL + GitHub `device_code` (+ optional normalized enterprise host)
//! the poll needs are packed into it (unit-separated) by `device_start` and
//! unpacked by `device_poll`. Errors are returned as `String`; the frontend
//! swallows them so no RPC/method name leaks into the UI.

use codelet_providers::copilot::auth::{write_copilot_auth, CopilotAuthJson};
use codelet_providers::copilot::oauth_device_code::{
    normalize_enterprise_domain, request_device_code,
};
use codelet_providers::copilot::oauth_polling::poll_device_token;
use codelet_providers::copilot::oauth_types::{
    CopilotDeviceCodeResponse, CopilotPollConfig, CopilotPollResult, COPILOT_DEFAULT_HOST,
};
use codelet_rpc_types::OAuthDeviceStart;

/// Polling timeout for the entire device-flow loop (10 minutes — matches
/// GitHub's documented device-code lifetime upper bound).
const COPILOT_POLL_TIMEOUT_MS: u64 = 10 * 60 * 1000;

/// Unit Separator used to pack `host_url`/`device_code`/`enterprise_host` into
/// the opaque `device_auth_id` round-tripped through the shared device flow.
const FIELD_SEP: char = '\u{1f}';

/// Phase 1 of the github-copilot device flow: resolve the host (github.com or
/// the normalized enterprise host), request a device code, and return the
/// user-facing `user_code` + `verification_url` plus the packed
/// `device_auth_id` the follow-up poll needs.
pub async fn device_start(enterprise_host: Option<String>) -> Result<OAuthDeviceStart, String> {
    let (host_url, normalized_host) = match enterprise_host {
        Some(raw) if !raw.trim().is_empty() => {
            let host = normalize_enterprise_domain(&raw);
            (format!("https://{host}"), Some(host))
        }
        _ => (COPILOT_DEFAULT_HOST.to_string(), None),
    };

    let dc = request_device_code(&host_url)
        .await
        .map_err(|e| format!("device start failed: {e}"))?;

    Ok(OAuthDeviceStart {
        user_code: dc.user_code,
        verification_url: dc.verification_uri,
        device_auth_id: pack_device_auth_id(&host_url, &dc.device_code, normalized_host.as_deref()),
        interval: dc.interval,
    })
}

/// Phase 2 of the github-copilot device flow: poll the access-token endpoint
/// until the user authorizes (or a terminal error), then persist the
/// long-lived GitHub OAuth token (the Copilot token is exchanged lazily on the
/// first API call).
pub async fn device_poll(device_auth_id: String, interval: u64) -> Result<(), String> {
    let (host_url, device_code, enterprise_host) = unpack_device_auth_id(&device_auth_id)?;

    let device_code_response = CopilotDeviceCodeResponse {
        device_code,
        user_code: String::new(),
        verification_uri: String::new(),
        interval,
    };
    let poll_config = CopilotPollConfig {
        host_url: &host_url,
        timeout_ms: COPILOT_POLL_TIMEOUT_MS,
        poll_interval_override_ms: None,
        slow_down_increment_override_ms: None,
        authorization_pending_safety_margin_override_ms: None,
    };

    let access_token = match poll_device_token(&poll_config, &device_code_response)
        .await
        .map_err(|e| format!("device poll failed: {e}"))?
    {
        CopilotPollResult::Success { access_token } => access_token,
        CopilotPollResult::TerminalError { error } => {
            return Err(format!("device auth failed: {error}"));
        }
    };

    let auth = CopilotAuthJson::from_github_oauth_token(access_token, enterprise_host);
    write_copilot_auth(&auth)
        .await
        .map_err(|e| format!("persist failed: {e}"))
}

/// Pack the poll inputs into the opaque `device_auth_id`.
fn pack_device_auth_id(host_url: &str, device_code: &str, enterprise_host: Option<&str>) -> String {
    format!(
        "{host_url}{FIELD_SEP}{device_code}{FIELD_SEP}{}",
        enterprise_host.unwrap_or("")
    )
}

/// Unpack the poll inputs from the opaque `device_auth_id`.
fn unpack_device_auth_id(packed: &str) -> Result<(String, String, Option<String>), String> {
    let mut parts = packed.split(FIELD_SEP);
    let host_url = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "malformed copilot device handle".to_string())?
        .to_string();
    let device_code = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "malformed copilot device handle".to_string())?
        .to_string();
    let enterprise_host = parts.next().filter(|s| !s.is_empty()).map(str::to_string);
    Ok((host_url, device_code, enterprise_host))
}
