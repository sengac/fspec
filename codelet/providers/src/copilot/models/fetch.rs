//! HTTP fetch for the Copilot `/models` endpoint (PROV-056 rule [0]).
//!
//! This module owns network IO only. The pure wire-format → domain mapping
//! lives in [`crate::copilot::models::builder`], so this module is trivially
//! replaceable by a different transport (wiremock, reqwest_mock, etc.) in
//! tests.

use super::builder::build_catalog_from_response;
use super::schema::CopilotModelsResponse;
use crate::copilot::constants::COPILOT_PROVIDER_ID;
use crate::error::ProviderError;
use crate::models::ModelInfo;
use std::time::Duration;

/// Path segment appended to the configured base URL when fetching models.
pub const COPILOT_MODELS_PATH: &str = "/models";

/// Single-attempt timeout for the `/models` fetch (PROV-056 rule [0]:
/// 5000 ms, no retry).
pub const COPILOT_FETCH_TIMEOUT: Duration = Duration::from_millis(5_000);

/// Fetch the Copilot `/models` endpoint and convert it to a `Vec<ModelInfo>`.
///
/// Sole source of truth: every field on every returned `ModelInfo` comes
/// directly from this single HTTP response. The function does not consult any
/// other registry, cache, or local table. Each call fully replaces the
/// caller's view of the catalog.
///
/// Network behaviour matches the rule set:
/// - 5 second timeout
/// - single attempt (no retry)
/// - bearer-token `Authorization` header
///
/// # Errors
///
/// Returns [`ProviderError::Api`] for transport, status, or JSON parse failures.
pub async fn fetch_models(base_url: &str, token: &str) -> Result<Vec<ModelInfo>, ProviderError> {
    let client = reqwest::Client::builder()
        .timeout(COPILOT_FETCH_TIMEOUT)
        .build()
        .map_err(|e| ProviderError::Api {
            provider: COPILOT_PROVIDER_ID.to_string(),
            message: format!("failed to build HTTP client: {e}"),
        })?;

    let url = format!("{}{}", base_url.trim_end_matches('/'), COPILOT_MODELS_PATH);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| ProviderError::Api {
            provider: COPILOT_PROVIDER_ID.to_string(),
            message: format!("/models request failed: {e}"),
        })?;

    if !response.status().is_success() {
        return Err(ProviderError::Api {
            provider: COPILOT_PROVIDER_ID.to_string(),
            message: format!("/models returned HTTP {}", response.status()),
        });
    }

    let parsed: CopilotModelsResponse = response.json().await.map_err(|e| ProviderError::Api {
        provider: COPILOT_PROVIDER_ID.to_string(),
        message: format!("/models response parse error: {e}"),
    })?;

    Ok(build_catalog_from_response(&parsed))
}
