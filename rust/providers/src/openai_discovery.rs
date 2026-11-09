use std::time::Duration;

use crate::error::{ModelDiscoveryFailure, ProviderError};

/// BUG-309: the endpoint origin (scheme://host:port) without path, query or
/// credentials, so errors say which server failed without leaking secrets.
fn origin(base_url: &str) -> String {
    reqwest::Url::parse(base_url)
        .ok()
        .filter(reqwest::Url::has_host)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_else(|| "an invalid endpoint".to_string())
}

fn failed(endpoint: &str, failure: ModelDiscoveryFailure) -> ProviderError {
    ProviderError::ModelDiscoveryFailed {
        provider: "openai".to_string(),
        endpoint: endpoint.to_string(),
        failure,
    }
}

fn transport_failure(endpoint: &str, error: reqwest::Error) -> ProviderError {
    failed(
        endpoint,
        if error.is_timeout() {
            ModelDiscoveryFailure::Timeout
        } else {
            ModelDiscoveryFailure::Transport
        },
    )
}

pub(super) async fn list_models(
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<String>, ProviderError> {
    let endpoint = origin(base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| transport_failure(&endpoint, error))?;
    let trimmed = base_url.trim_end_matches('/');
    let normalized = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
    let mut request = client.get(format!("{normalized}/v1/models"));
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|error| transport_failure(&endpoint, error))?;
    let status = response.status().as_u16();
    if matches!(status, 404 | 405) {
        return Err(ProviderError::ModelDiscoveryUnsupported {
            provider: "openai".to_string(),
            status,
        });
    }
    if matches!(status, 401 | 403) {
        return Err(failed(
            &endpoint,
            ModelDiscoveryFailure::Authentication(status),
        ));
    }
    if !response.status().is_success() {
        return Err(failed(&endpoint, ModelDiscoveryFailure::Http(status)));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| transport_failure(&endpoint, error))?;
    let body: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| failed(&endpoint, ModelDiscoveryFailure::Protocol))?;
    body.get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| failed(&endpoint, ModelDiscoveryFailure::Protocol))?
        .iter()
        .map(|model| {
            model
                .get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|model_id| !model_id.trim().is_empty())
                .map(String::from)
                .ok_or_else(|| failed(&endpoint, ModelDiscoveryFailure::Protocol))
        })
        .collect()
}
