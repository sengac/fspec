//! HTTP transport helpers for `RhaiCustomProvider` (PROV-063).
//!
//! Keeps `provider.rs` under the 300-line cap by factoring out the
//! reqwest-facing glue.

use futures::Stream;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT};
use rhai::{Dynamic, Map};
use std::pin::Pin;

use crate::error::ProviderError;

/// POST `body` to `url` with `headers` and return `(status_code, body_text)`.
pub(crate) async fn post_json(
    client: &reqwest::Client,
    provider: &str,
    url: &str,
    headers: HeaderMap,
    body: &serde_json::Value,
) -> Result<(u16, String), ProviderError> {
    tracing::warn!(
        provider = provider,
        url = url,
        header_count = headers.len(),
        "[rhai-dispatch] http::post_json ENTER"
    );
    let body_string = serde_json::to_string(body).map_err(|e| {
        ProviderError::api(
            provider.to_string(),
            format!("serialise request body: {e}"),
        )
    })?;
    let body_len = body_string.len();
    let response = client
        .post(url)
        .headers(headers)
        .body(body_string)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(
                provider = provider,
                url = url,
                error = %e,
                "[rhai-dispatch] http::post_json: request send failed"
            );
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
    tracing::warn!(
        provider = provider,
        url = url,
        status = status,
        request_body_len = body_len,
        response_body_len = body_text.len(),
        "[rhai-dispatch] http::post_json EXIT"
    );
    Ok((status, body_text))
}

/// POST `body` to `url` with SSE-friendly headers and return
/// `(status_code, byte_stream)`. Caller is responsible for parsing the
/// SSE stream (typically via [`super::stream_http::open_stream`]).
///
/// Adds `Accept: text/event-stream` if the script-supplied headers do
/// not already declare an Accept header.
pub(crate) async fn post_sse(
    client: &reqwest::Client,
    provider: &str,
    url: &str,
    mut headers: HeaderMap,
    body: &serde_json::Value,
) -> Result<
    (
        u16,
        Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    ),
    ProviderError,
> {
    tracing::warn!(
        provider = provider,
        url = url,
        header_count = headers.len(),
        has_accept_header = headers.contains_key(ACCEPT),
        "[rhai-dispatch] http::post_sse ENTER"
    );
    if !headers.contains_key(ACCEPT) {
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    }
    let body_string = serde_json::to_string(body).map_err(|e| {
        ProviderError::api(
            provider.to_string(),
            format!("serialise request body: {e}"),
        )
    })?;
    let body_len = body_string.len();
    let response = client
        .post(url)
        .headers(headers)
        .body(body_string)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(
                provider = provider,
                url = url,
                error = %e,
                "[rhai-dispatch] http::post_sse: request send failed"
            );
            ProviderError::api(
                provider.to_string(),
                format!("HTTP request failed: {e}"),
            )
        })?;
    let status = response.status().as_u16();
    tracing::warn!(
        provider = provider,
        url = url,
        status = status,
        request_body_len = body_len,
        "[rhai-dispatch] http::post_sse: got response, returning byte_stream"
    );
    let stream = Box::pin(response.bytes_stream());
    Ok((status, stream))
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
