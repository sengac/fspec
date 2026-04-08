//! Shared constants for the Copilot provider module (PROV-053/056).
//!
//! Extracted so that test, catalog, middleware, and provider code all
//! reference exactly one source of truth for these identifiers.

/// Provider id used in `ProviderError::Api` messages and the `providerID`
/// option on every emitted `ModelInfo`.
pub const COPILOT_PROVIDER_ID: &str = "github-copilot";

/// NPM AI-SDK key advertised on every emitted `ModelInfo`.
pub const COPILOT_NPM_KEY: &str = "@ai-sdk/github-copilot";

/// User-agent header value for all outgoing Copilot API requests.
/// Built dynamically at request time from `env!("CARGO_PKG_VERSION")`.
/// Exposed as a helper constant so tests can compute the expected value.
pub const COPILOT_USER_AGENT_PREFIX: &str = "codelet/";

/// Static `Openai-Intent` header value for all outgoing Copilot API requests.
pub const COPILOT_OPENAI_INTENT_VALUE: &str = "conversation-edits";

/// Compile-time helper: the full default User-Agent string as the header
/// value sent at runtime. Exists so test code can assert exact equality
/// rather than `starts_with("codelet/")`.
#[must_use]
pub fn copilot_user_agent() -> String {
    format!("{COPILOT_USER_AGENT_PREFIX}{}", env!("CARGO_PKG_VERSION"))
}
