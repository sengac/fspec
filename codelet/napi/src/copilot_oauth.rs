//! NAPI Bindings for GitHub Copilot OAuth Flows (PROV-054)
//!
//! Exposes the Rust Copilot OAuth device flow (RFC 8628) and credential
//! persistence to the TypeScript TUI layer via NAPI bindings.
//!
//! Architecture mirrors `claude_oauth.rs` and `codex_oauth.rs`:
//!
//! - `copilot_oauth_device_login_start(enterprise_url?)` → async, requests
//!   a device code from `https://github.com/login/device/code` (or the
//!   normalised enterprise host) and returns the user_code +
//!   verification_url + the device_code that polling needs.
//! - `copilot_oauth_device_login_poll(device_code, interval, host_url,
//!   enterprise_host?)` → async, polls the access_token endpoint until
//!   the user authorises (or a terminal error / timeout occurs), then
//!   persists the credential to `~/.fspec/credentials/copilot_auth.json`
//!   with mode 0600.
//! - `copilot_oauth_get_credential()` → async, reads the persisted credential
//!   or returns null.
//! - `copilot_oauth_clear_credential()` → async, deletes the credential file
//!   (idempotent).
//! - `copilot_normalize_enterprise_domain(input)` → sync helper exposing
//!   the same normalisation logic as the Rust core so the TS layer can
//!   preview the host before submission.

use codelet_providers::copilot::auth::{
    delete_copilot_auth, read_copilot_auth, write_copilot_auth, CopilotAuthJson,
    COPILOT_TOKEN_NEVER_EXPIRES,
};
use codelet_providers::copilot::oauth_device_code::{
    normalize_enterprise_domain, request_device_code,
};
use codelet_providers::copilot::oauth_polling::poll_device_token;
use codelet_providers::copilot::oauth_types::{
    CopilotPollConfig, CopilotPollResult, COPILOT_DEFAULT_HOST,
};
use napi::bindgen_prelude::*;

/// Default polling timeout for the entire device-flow polling loop.
/// 10 minutes — matches GitHub's documented device-code lifetime upper bound.
const COPILOT_POLL_TIMEOUT_MS: u64 = 10 * 60 * 1000;

// ============================================================================
// NAPI Object Structs
// ============================================================================

/// Copilot OAuth credential exposed to TypeScript. Mirrors `CopilotAuthJson`.
///
/// PROV-057: after the schema migration to the two-token model the
/// `access_token`/`refresh_token`/`expires` fields are kept here for
/// TypeScript backward compatibility. They are all populated from the
/// `github_oauth_token` slot so existing TUI code keeps working while the
/// TS layer migrates to reading the new field directly.
#[napi(object)]
pub struct NapiCopilotCredential {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiry timestamp in milliseconds since Unix epoch. 0 = never expires
    /// (the long-lived GitHub OAuth token does not expire on its own).
    pub expires: f64,
    /// Some(normalised host) for GitHub Enterprise, null for github.com.
    pub enterprise_url: Option<String>,
}

impl From<CopilotAuthJson> for NapiCopilotCredential {
    fn from(auth: CopilotAuthJson) -> Self {
        Self {
            access_token: auth.github_oauth_token.clone(),
            refresh_token: auth.github_oauth_token,
            expires: COPILOT_TOKEN_NEVER_EXPIRES as f64,
            enterprise_url: auth.enterprise_url,
        }
    }
}

/// Result from `copilot_oauth_device_login_start` — phase 1 of the device flow.
///
/// Returned synchronously to the TUI so the user_code + verification_url
/// can be displayed before polling begins. The TUI then passes the
/// device_code + interval + host_url back into
/// `copilot_oauth_device_login_poll` to drive phase 2.
#[napi(object)]
pub struct NapiCopilotDeviceStartResult {
    pub user_code: String,
    pub verification_url: String,
    pub device_code: String,
    /// Server-provided polling interval, in seconds.
    pub interval: f64,
    /// Resolved host URL the device-code request was issued against
    /// (`https://github.com` or `https://<ghe-host>`).
    pub host_url: String,
    /// `"github.com"` for the public deployment, `"enterprise"` otherwise.
    pub deployment_type: String,
    /// Normalised enterprise host (present when deployment_type == "enterprise").
    pub enterprise_host: Option<String>,
}

// ============================================================================
// Device Auth Login (Two-Phase)
// ============================================================================

/// Phase 1: Start the Copilot device authorization flow.
///
/// `enterprise_url` is the optional raw user-supplied enterprise URL. When
/// provided, it is normalised via `normalize_enterprise_domain` before the
/// device-code request is issued. When omitted/null, the request goes to
/// `https://github.com/login/device/code`.
///
/// Returns the user_code + verification_url + the device_code so the TUI
/// can display the UX immediately and then drive polling.
#[napi]
pub async fn copilot_oauth_device_login_start(
    enterprise_url: Option<String>,
) -> Result<NapiCopilotDeviceStartResult> {
    let (host_url, deployment_type, enterprise_host) = match enterprise_url {
        Some(raw) if !raw.is_empty() => {
            let host = normalize_enterprise_domain(&raw);
            (
                format!("https://{host}"),
                "enterprise".to_string(),
                Some(host),
            )
        }
        _ => (
            COPILOT_DEFAULT_HOST.to_string(),
            "github.com".to_string(),
            None,
        ),
    };

    let device_code = request_device_code(&host_url)
        .await
        .map_err(|e| Error::from_reason(format!("Copilot device code request failed: {e}")))?;

    Ok(NapiCopilotDeviceStartResult {
        user_code: device_code.user_code,
        verification_url: device_code.verification_uri,
        device_code: device_code.device_code,
        interval: device_code.interval as f64,
        host_url,
        deployment_type,
        enterprise_host,
    })
}

/// Phase 2: Poll the device-token endpoint until authorization completes.
///
/// On success, persists the credential to `copilot_auth.json` (mode 0600)
/// and returns it. On terminal error or timeout, returns a NAPI error
/// with a human-readable message that the TUI can render in the
/// `oauth-error` mode.
#[napi]
pub async fn copilot_oauth_device_login_poll(
    device_code: String,
    interval: f64,
    host_url: String,
    enterprise_host: Option<String>,
) -> Result<NapiCopilotCredential> {
    let device_code_response = codelet_providers::copilot::oauth_types::CopilotDeviceCodeResponse {
        device_code,
        user_code: String::new(),        // not needed for polling
        verification_uri: String::new(), // not needed for polling
        interval: interval as u64,
    };

    let poll_config = CopilotPollConfig {
        host_url: &host_url,
        timeout_ms: COPILOT_POLL_TIMEOUT_MS,
        poll_interval_override_ms: None,
        slow_down_increment_override_ms: None,
        authorization_pending_safety_margin_override_ms: None,
    };

    let poll_result = poll_device_token(&poll_config, &device_code_response)
        .await
        .map_err(|e| Error::from_reason(format!("Copilot polling failed: {e}")))?;

    let access_token = match poll_result {
        CopilotPollResult::Success { access_token } => access_token,
        CopilotPollResult::TerminalError { error } => {
            return Err(Error::from_reason(format!(
                "Copilot device login failed: {error}"
            )));
        }
    };

    // Persist credential per PROV-057 (two-token model). The Copilot
    // token exchange happens on the first API call.
    let auth = CopilotAuthJson::from_github_oauth_token(access_token, enterprise_host);

    write_copilot_auth(&auth)
        .await
        .map_err(|e| Error::from_reason(format!("Failed to persist copilot_auth.json: {e}")))?;

    Ok(NapiCopilotCredential::from(auth))
}

// ============================================================================
// Credential Read / Clear
// ============================================================================

/// Read the persisted Copilot credential.
///
/// Returns `null` if `~/.fspec/credentials/copilot_auth.json` does not exist.
/// Used by the TUI to detect whether GitHub Copilot is currently authenticated.
#[napi]
pub async fn copilot_oauth_get_credential() -> Result<Option<NapiCopilotCredential>> {
    let auth = read_copilot_auth()
        .await
        .map_err(|e| Error::from_reason(format!("Failed to read copilot_auth.json: {e}")))?;
    Ok(auth.map(NapiCopilotCredential::from))
}

/// Delete the persisted Copilot credential. Idempotent.
///
/// Used by the TUI's "Logout from OAuth" action.
#[napi]
pub async fn copilot_oauth_clear_credential() -> Result<()> {
    delete_copilot_auth()
        .await
        .map_err(|e| Error::from_reason(format!("Failed to delete copilot_auth.json: {e}")))
}

// ============================================================================
// Pure Helpers
// ============================================================================

/// Normalise a user-supplied enterprise URL to a bare host.
///
/// Strips the scheme (`https://` / `http://`) and any trailing `/`.
/// Exposed so the TS layer can preview the normalised host before
/// submission, mirroring the Rust core to prevent drift.
#[napi]
pub fn copilot_normalize_enterprise_domain(input: String) -> String {
    normalize_enterprise_domain(&input)
}
