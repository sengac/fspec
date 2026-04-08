//! GitHub Copilot OAuth Device Flow — types, constants, and configuration
//! structs (PROV-054).
//!
//! Extracted from `oauth.rs` to keep each file within the 300-line SoC
//! budget (PROV-053 rule 21).

use serde::Deserialize;

// =========================================================================
// Constants
// =========================================================================

/// Well-known GitHub Copilot OAuth App client_id (PROV-057).
///
/// This is the shared public client_id used by copilot.vim, the JetBrains
/// Copilot plugin, aider, cline, and most third-party Copilot integrations.
/// It is **required** — GitHub's `/copilot_internal/v2/token` token-exchange
/// endpoint validates the originating `client_id` and rejects any `gho_*`
/// token minted from a non-Copilot client id.
///
/// The previous value (`Ov23li8tweQw6odWQebz`) was opencode's client_id and
/// caused every fspec login to silently produce an unusable token that
/// returned 401 from every call to `api.githubcopilot.com`. See the
/// PROV-057 investigation document for the full analysis.
pub const COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

/// Default github.com host for non-Enterprise deployments
pub const COPILOT_DEFAULT_HOST: &str = "https://github.com";

/// OAuth scope requested by the device flow
pub const COPILOT_OAUTH_SCOPE: &str = "read:user";

/// Authorization-pending safety margin added to the polling interval (per PROV-054 Rule 5)
pub const AUTHORIZATION_PENDING_SAFETY_MARGIN_MS: u64 = 3_000;

/// Slow-down backoff increment per RFC 8628 §3.5 (5 seconds)
pub const SLOW_DOWN_INCREMENT_MS: u64 = 5_000;

// =========================================================================
// Types
// =========================================================================

/// Which GitHub deployment a Copilot login is targeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopilotDeploymentType {
    /// github.com (the public deployment)
    GitHubCom,
    /// GitHub Enterprise — requires the normalized host (no scheme, no trailing slash)
    Enterprise { host: String },
}

/// Response from the GitHub device authorization endpoint
/// (`POST /login/device/code`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CopilotDeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

/// Result of polling the GitHub access_token endpoint.
#[derive(Debug, Clone)]
pub enum CopilotPollResult {
    /// User completed authorization — contains the access_token
    Success { access_token: String },
    /// Terminal error — stop polling (expired_token, access_denied, etc.)
    TerminalError { error: String },
}

/// Display callback for showing user_code + verification_uri to the user
pub type CopilotDisplayCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Configuration for the polling loop.
///
/// Bundles the polling parameters to keep `poll_device_token` ergonomic.
pub struct CopilotPollConfig<'a> {
    /// Base host URL (e.g. "https://github.com" or a wiremock URI)
    pub host_url: &'a str,
    /// Overall timeout in milliseconds for the entire polling loop
    pub timeout_ms: u64,
    /// Optional override for the polling interval in ms (tests use short intervals)
    pub poll_interval_override_ms: Option<u64>,
    /// Optional override for the slow_down backoff increment in ms.
    /// Production uses `SLOW_DOWN_INCREMENT_MS` (5000ms per RFC 8628 §3.5).
    pub slow_down_increment_override_ms: Option<u64>,
    /// Optional override for the authorization_pending safety margin in ms.
    /// Production uses `AUTHORIZATION_PENDING_SAFETY_MARGIN_MS` (3000ms per PROV-054 Rule 5).
    pub authorization_pending_safety_margin_override_ms: Option<u64>,
}

/// Configuration for `copilot_device_auth_login`.
pub struct CopilotDeviceAuthConfig {
    /// Base host URL (`https://github.com` or `https://<enterprise-host>`)
    pub host_url: String,
    /// Which deployment is being targeted (used to populate `enterprise_url` field on persist)
    pub deployment_type: CopilotDeploymentType,
    /// Overall timeout in milliseconds for the entire device auth flow
    pub timeout_ms: u64,
    /// Optional override for the polling interval in ms (tests use short intervals)
    pub poll_interval_override_ms: Option<u64>,
    /// Optional override for the slow_down backoff increment in ms.
    pub slow_down_increment_override_ms: Option<u64>,
    /// Optional override for the authorization_pending safety margin in ms.
    pub authorization_pending_safety_margin_override_ms: Option<u64>,
    /// Optional callback to display user_code and verification URL
    pub display_fn: Option<CopilotDisplayCallback>,
}
