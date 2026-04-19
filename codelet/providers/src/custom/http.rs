//! HTTP transport helpers for `RhaiCustomProvider` (PROV-063).
//!
//! Keeps `provider.rs` under the 300-line cap by factoring out the
//! reqwest-facing glue.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rhai::{Dynamic, Map};

use crate::error::ProviderError;

/// POST `body` to `url` with `headers` and return `(status_code, body_text)`.
pub(crate) async fn post_json(
    client: &reqwest::Client,
    provider: &str,
    url: &str,
    headers: HeaderMap,
    body: &serde_json::Value,
) -> Result<(u16, String), ProviderError> {
    let body_string = serde_json::to_string(body).map_err(|e| {
        ProviderError::api(
            provider.to_string(),
            format!("serialise request body: {e}"),
        )
    })?;
    let response = client
        .post(url)
        .headers(headers)
        .body(body_string)
        .send()
        .await
        .map_err(|e| {
            ProviderError::api(
                provider.to_string(),
                format!("HTTP request failed: {e}"),
            )
        })?;
    let status = response.status().as_u16();
    let body_text = response.text().await.map_err(|e| {
        ProviderError::api(
            provider.to_string(),
            format!("reading response body: {e}"),
        )
    })?;
    Ok((status, body_text))
}

/// Convert a Rhai `Dynamic` (expected to be a `Map` of string→string)
/// into a `reqwest::HeaderMap`.
pub(crate) fn dynamic_to_header_map(
    provider: &str,
    value: Dynamic,
) -> Result<HeaderMap, ProviderError> {
    let map = value.try_cast::<Map>().ok_or_else(|| {
        ProviderError::api(provider.to_string(), "build_headers must return a Map")
    })?;
    let mut headers = HeaderMap::new();
    for (k, v) in &map {
        let name = HeaderName::from_bytes(k.as_bytes()).map_err(|e| {
            ProviderError::api(
                provider.to_string(),
                format!("invalid header name '{k}': {e}"),
            )
        })?;
        let val_str = v.clone().into_string().map_err(|typ| {
            ProviderError::api(
                provider.to_string(),
                format!("header '{k}' must be a string (got {typ})"),
            )
        })?;
        let val = HeaderValue::from_str(&val_str).map_err(|e| {
            ProviderError::api(
                provider.to_string(),
                format!("invalid header value for '{k}': {e}"),
            )
        })?;
        headers.insert(name, val);
    }
    Ok(headers)
}
