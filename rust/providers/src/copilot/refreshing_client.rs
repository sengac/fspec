//! `CopilotHttpClient` — `rig::http_client::HttpClientExt` middleware that
//! injects the required Copilot header set on every outgoing request.
//!
//! PROV-055 rule 1: "`CopilotHttpClient` implements `rig::http_client::HttpClientExt`
//! as a middleware layer that wraps every outgoing request, mirroring the
//! `RefreshingClaudeClient` / `RefreshingCodexClient` pattern."
//!
//! Provider-bound requests reuse the existing short-token refresh coordinator.
//! Opaque protocol metadata is injected only at the transport boundary, after
//! generic request tracing, and removed before generic response handling.

use http::Request;
use tracing::debug;

use crate::copilot::classifier::{CopilotRequestClassifier, RequestClassification};
use crate::copilot::header_facade::CopilotHeaderFacade;
use crate::copilot::prompt_cache;

#[path = "http_auth.rs"]
mod http_auth;

/// HTTP middleware that injects Copilot headers on every outgoing request.
///
/// Implements `rig::http_client::HttpClientExt`. The middleware is cloneable
/// so rig can clone it per-request; the inner `reqwest::Client` is already
/// cheap to clone and the access token is stored behind an `Arc` so the
/// clone cost is a single pointer copy.
#[derive(Clone)]
pub struct CopilotHttpClient {
    inner: reqwest::Client,
    access_token: std::sync::Arc<str>,
    auth: Option<std::sync::Arc<tokio::sync::RwLock<super::CopilotAuthJson>>>,
    initial_base: Option<String>,
    capture: Option<super::capture::Capture>,
}

impl std::fmt::Debug for CopilotHttpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CopilotHttpClient")
            .finish_non_exhaustive()
    }
}

impl CopilotHttpClient {
    /// Create a new `CopilotHttpClient` with the given access token.
    #[must_use]
    pub fn new(access_token: String) -> Self {
        Self {
            inner: reqwest::Client::new(),
            access_token: std::sync::Arc::from(access_token),
            auth: None,
            initial_base: None,
            capture: None,
        }
    }

    pub(crate) fn with_capture(mut self, capture: Option<super::capture::Capture>) -> Self {
        self.capture = capture;
        self
    }

    /// Access the bound access token (for tests and diagnostics).
    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Borrow the inner reqwest client (for tests that need to introspect
    /// the underlying HTTP backend).
    #[must_use]
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }
}

impl Default for CopilotHttpClient {
    fn default() -> Self {
        // The default client carries no token — attempting to use it for a
        // real request will fail at the API layer with an authentication
        // error, which is the same behaviour as the other providers.
        Self::new(String::new())
    }
}

/// Parse the request body as JSON, inject prompt cache control if the model
/// is a Claude-family model, and return the classification plus the
/// (possibly-mutated) body bytes.
///
/// Returns the default classification and the **original** body bytes if the
/// body is empty, not JSON, or malformed. This avoids double-parsing: the
/// JSON is deserialized once, classified, cache-control-injected, then
/// re-serialized.
fn classify_and_cache_body(body: bytes::Bytes) -> (RequestClassification, bytes::Bytes) {
    if body.is_empty() {
        return (RequestClassification::default(), body);
    }
    match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(mut value) => {
            let classification = CopilotRequestClassifier::classify(&value);
            // Inject copilot_cache_control for Claude models (PROV-058).
            prompt_cache::inject_cache_control(&mut value);
            // Re-serialize the (possibly mutated) body.
            let new_body = serde_json::to_vec(&value)
                .map(bytes::Bytes::from)
                .unwrap_or_else(|_| body.clone());
            (classification, new_body)
        }
        Err(e) => {
            debug!(
                "CopilotHttpClient: body is not JSON — falling back to default classification: {e}"
            );
            (RequestClassification::default(), body)
        }
    }
}

/// Strip any stale `Authorization` header and replace the entire header set
/// with the Copilot-specific one. This mirrors `prepare_oauth_request` in
/// `claude_refreshing_client.rs`, but uses [`CopilotHeaderFacade`] so there
/// is only **one** place in the codebase that owns the Copilot header rules.
fn inject_copilot_headers(
    mut req: Request<bytes::Bytes>,
    classification: RequestClassification,
    access_token: &str,
) -> Request<bytes::Bytes> {
    // CopilotHeaderFacade::build_headers returns a fresh HeaderMap containing
    // every required Copilot header. We copy each entry into the request's
    // header map, replacing any existing value so stale rig-injected headers
    // are overwritten (rather than duplicated, which is valid HTTP for some
    // headers but would confuse the Copilot API).
    let headers = CopilotHeaderFacade::build_headers(&classification, access_token);
    let req_headers = req.headers_mut();
    for (name, value) in headers.iter() {
        req_headers.insert(name.clone(), value.clone());
    }
    req
}

impl rig::http_client::HttpClientExt for CopilotHttpClient {
    fn prepare_request_body(&self, body: Vec<u8>) -> rig::http_client::Result<Vec<u8>> {
        let (_, body) = classify_and_cache_body(body.into());
        Ok(body.to_vec())
    }

    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        let inner = self.inner.clone();
        let client = self.clone();
        // Convert T → Bytes *before* the async block so we don't need `T: 'static`.
        let req = req.map(Into::into);

        async move {
            let (req, access_token) = client.fresh_request(req).await?;
            let (classification, new_body) = classify_and_cache_body(req.body().clone());
            let req = {
                let (parts, _) = req.into_parts();
                Request::from_parts(parts, new_body)
            };
            let req = inject_copilot_headers(req, classification, &access_token);
            let Some(capture) = client.capture else {
                return inner.send(req).await;
            };
            let (parts, body) = req.into_parts();
            let req = Request::from_parts(parts, capture.request(body)?);
            let response: http::Response<rig::http_client::LazyBody<bytes::Bytes>> =
                inner.send(req).await?;
            let (parts, body) = response.into_parts();
            let body: rig::http_client::LazyBody<U> = Box::pin(async move {
                let bytes = body.await?;
                let value = serde_json::from_slice(&bytes)
                    .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
                let value = capture.response(value, true)?;
                let bytes = serde_json::to_vec(&value)
                    .map_err(|error| rig::http_client::Error::Instance(Box::new(error)))?;
                Ok(U::from(bytes::Bytes::from(bytes)))
            });
            Ok(http::Response::from_parts(parts, body))
        }
    }

    fn send_multipart<U>(
        &self,
        req: Request<rig::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        let inner = self.inner.clone();
        let client = self.clone();

        async move {
            let (req, access_token) = client.fresh_request(req).await?;
            // Multipart requests (rare for Copilot) cannot be JSON-classified
            // without consuming the body. Fall back to the default
            // classification (no vision, no agent) and inject the headers.
            let classification = RequestClassification::default();
            // Copy headers manually because we can't `map` the body.
            let (mut parts, body) = req.into_parts();
            let headers = CopilotHeaderFacade::build_headers(&classification, &access_token);
            for (name, value) in headers.iter() {
                parts.headers.insert(name.clone(), value.clone());
            }
            let req = Request::from_parts(parts, body);
            inner.send_multipart(req).await
        }
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<rig::http_client::StreamingResponse>,
    > + Send
    where
        T: Into<bytes::Bytes>,
    {
        let inner = self.inner.clone();
        let client = self.clone();
        let req = req.map(Into::into);

        async move {
            let (req, access_token) = client.fresh_request(req).await?;
            let (classification, new_body) = classify_and_cache_body(req.body().clone());
            let req = {
                let (parts, _) = req.into_parts();
                Request::from_parts(parts, new_body)
            };
            let req = inject_copilot_headers(req, classification, &access_token);
            if let Some(capture) = client.capture {
                let (parts, body) = req.into_parts();
                let req = Request::from_parts(parts, capture.request(body)?);
                return inner
                    .send_streaming(req)
                    .await
                    .map(|response| capture.stream(response));
            }
            inner.send_streaming(req).await
        }
    }
}

#[cfg(test)]
#[path = "refreshing_client_tests.rs"]
mod tests;
