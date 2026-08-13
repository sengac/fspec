//! Base URL type for GitHub Copilot deployments (PROV-055).
//!
//! Wraps a `String` to keep a typed boundary between "computed base URL"
//! and arbitrary strings and to make the intent self-documenting at call
//! sites.

use crate::copilot::oauth_types::CopilotDeploymentType;

/// Base URL for an active Copilot deployment.
///
/// Wraps a `String` to keep the typed boundary between "computed base URL"
/// and arbitrary strings and to make the intent self-documenting at call
/// sites (see the wiremock integration tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopilotBaseUrl(String);

impl CopilotBaseUrl {
    /// Borrow the base URL as a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner `String`.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Build a `CopilotBaseUrl` from a raw string (e.g. the `endpoints.api`
    /// URL returned by `/copilot_internal/v2/token` — PROV-057).
    #[must_use]
    pub fn from_string(url: String) -> Self {
        Self(url)
    }
}

/// Compute the API base URL for an active Copilot deployment.
///
/// - [`CopilotDeploymentType::GitHubCom`] → `https://api.githubcopilot.com`
/// - [`CopilotDeploymentType::Enterprise`] → `https://copilot-api.<host>`
///
/// Per PROV-055 rule 9 the enterprise URL uses the `copilot-api.`
/// subdomain on the enterprise host (not the plain `api.` subdomain).
#[must_use]
pub fn base_url_for(deployment: &CopilotDeploymentType) -> CopilotBaseUrl {
    match deployment {
        CopilotDeploymentType::GitHubCom => {
            CopilotBaseUrl("https://api.githubcopilot.com".to_string())
        }
        CopilotDeploymentType::Enterprise { host } => {
            CopilotBaseUrl(format!("https://copilot-api.{host}"))
        }
    }
}
