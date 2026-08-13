//! `CopilotHttpClient` — `rig::http_client::HttpClientExt` middleware that
//! injects the required Copilot header set on every outgoing request.
//!
//! PROV-055 rule 1: "`CopilotHttpClient` implements `rig::http_client::HttpClientExt`
//! as a middleware layer that wraps every outgoing request, mirroring the
//! `RefreshingClaudeClient` / `RefreshingCodexClient` pattern."
//!
//! Unlike the Claude/Codex variants, the Copilot OAuth access token never
//! expires on its own (`expires: 0` in `copilot_auth.json`), so this client
//! does **not** need a refresh loop, a write-lock state machine, or a token
//! endpoint. Its single responsibility is:
//!
//! 1. Classify the outgoing request body with [`CopilotRequestClassifier`]
//!    (pure function, no IO).
//! 2. Build the full Copilot header set with
//!    [`CopilotHeaderFacade::build_headers`] (pure function, no IO).
//! 3. Strip any stale `Authorization` header that rig might have set and
//!    forward the request to the inner `reqwest::Client` with the fresh
//!    header set applied.
//!
//! The middleware is deliberately thin and composable — all decision logic
//! lives in the facades from PROV-055, so this file contains no business
//! rules about headers, vision detection, agent-mode, or endpoints.

use http::Request;
use tracing::debug;

use crate::copilot::classifier::{CopilotRequestClassifier, RequestClassification};
use crate::copilot::header_facade::CopilotHeaderFacade;
use crate::copilot::prompt_cache;

/// HTTP middleware that injects Copilot headers on every outgoing request.
///
/// Implements `rig::http_client::HttpClientExt`. The middleware is cloneable
/// so rig can clone it per-request; the inner `reqwest::Client` is already
/// cheap to clone and the access token is stored behind an `Arc` so the
/// clone cost is a single pointer copy.
#[derive(Debug, Clone)]
pub struct CopilotHttpClient {
    inner: reqwest::Client,
    access_token: std::sync::Arc<str>,
}

impl CopilotHttpClient {
    /// Create a new `CopilotHttpClient` with the given access token.
    #[must_use]
    pub fn new(access_token: String) -> Self {
        Self {
            inner: reqwest::Client::new(),
            access_token: std::sync::Arc::from(access_token),
        }
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
        Self {
            inner: reqwest::Client::default(),
            access_token: std::sync::Arc::from(""),
        }
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
        let access_token = self.access_token.clone();
        // Convert T → Bytes *before* the async block so we don't need `T: 'static`.
        let req = req.map(Into::into);

        async move {
            let (classification, new_body) = classify_and_cache_body(req.body().clone());
            let req = {
                let (parts, _) = req.into_parts();
                Request::from_parts(parts, new_body)
            };
            let req = inject_copilot_headers(req, classification, &access_token);
            inner.send(req).await
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
        let access_token = self.access_token.clone();

        async move {
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
        let access_token = self.access_token.clone();
        let req = req.map(Into::into);

        async move {
            let (classification, new_body) = classify_and_cache_body(req.body().clone());
            let req = {
                let (parts, _) = req.into_parts();
                Request::from_parts(parts, new_body)
            };
            let req = inject_copilot_headers(req, classification, &access_token);
            inner.send_streaming(req).await
        }
    }
}

#[cfg(test)]
#[path = "refreshing_client_tests.rs"]
mod tests;
