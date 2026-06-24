//! Generic OAuth Callback Server (PROV-060)
//!
//! `OAuthCallbackServer<H: CodeExchangeHandler>` unifies the local HTTP
//! PKCE callback servers used by Codex and Claude providers.

use std::convert::Infallible;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::error;

use crate::oauth_http_utils::html_response;

/// Handler trait for provider-specific code exchange logic.
///
/// Implementations provide the HTML pages, state validation strategy,
/// and token exchange logic for the specific OAuth provider.
pub trait CodeExchangeHandler: Send + Sync + 'static {
    /// HTML to show on successful token exchange.
    fn success_html(&self) -> &str;

    /// HTML to show on cancellation.
    fn cancelled_html(&self) -> &str;

    /// Build error HTML from an error message.
    fn error_html(&self, message: &str) -> String;

    /// Extract the authorization code and state from the callback.
    ///
    /// For redirect-based flows (Codex): parse query params `?code=...&state=...`
    /// For paste-based flows (Claude): parse form body `code=...`
    fn extract_code_and_state(
        &self,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<(String, String)>;

    /// Validate the state parameter. Returns `Ok(())` if valid.
    fn validate_state(&self, expected: &str, received: &str) -> Result<()>;
}

/// Result from the OAuth callback.
enum CallbackResult {
    /// Successful callback with authorization code and state.
    Success { code: String, state: String },
    /// User cancelled.
    Cancelled,
    /// Error from the authorization server.
    AuthError(String),
}

/// Configuration for the OAuth callback server.
pub struct OAuthServerConfig {
    /// Pre-bound TCP listener
    pub listener: TcpListener,
    /// Expected state parameter for CSRF validation
    pub expected_state: String,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

/// Generic OAuth callback server.
pub struct OAuthCallbackServer<H: CodeExchangeHandler> {
    handler: H,
}

impl<H: CodeExchangeHandler> OAuthCallbackServer<H> {
    /// Create a new callback server with the given handler.
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    /// Run the server until a callback or cancellation is received.
    ///
    /// Returns the authorization code and validated state on success.
    pub async fn run(self, config: OAuthServerConfig) -> Result<(String, String)> {
        let (tx, rx) = oneshot::channel::<CallbackResult>();
        let server_state = Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let handler = Arc::new(self.handler);

        let callback_result = tokio::select! {
            result = Self::serve_until_done(
                config.listener,
                server_state,
                config.expected_state.clone(),
                handler.clone(),
            ) => {
                match result {
                    Ok(()) => rx.await.map_err(|_| anyhow!("Callback channel closed without result"))?,
                    Err(e) => return Err(anyhow!("OAuth server error: {e}")),
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(config.timeout_ms)) => {
                return Err(anyhow!(
                    "OAuth login timed out after {} seconds.",
                    config.timeout_ms / 1000
                ));
            }
        };

        match callback_result {
            CallbackResult::Cancelled => Err(anyhow!("Login cancelled by user")),
            CallbackResult::AuthError(msg) => Err(anyhow!("Authorization failed: {msg}")),
            CallbackResult::Success { code, state } => Ok((code, state)),
        }
    }

    /// Accept connections until a terminal route fires.
    async fn serve_until_done(
        listener: TcpListener,
        callback_state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<CallbackResult>>>>,
        expected_state: String,
        handler: Arc<H>,
    ) -> Result<()> {
        let done = Arc::new(tokio::sync::Notify::new());

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (stream, _) = accept_result?;
                    let io = TokioIo::new(stream);
                    let st = callback_state.clone();
                    let dn = done.clone();
                    let exp = expected_state.clone();
                    let h = handler.clone();

                    let svc = service_fn(move |req: Request<Incoming>| {
                        let st = st.clone();
                        let dn = dn.clone();
                        let exp = exp.clone();
                        let h = h.clone();
                        async move {
                            Self::handle_request(req, st, dn, exp, h).await
                        }
                    });

                    tokio::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .serve_connection(io, svc)
                            .await
                        {
                            error!("HTTP connection error: {e}");
                        }
                    });
                }
                () = done.notified() => {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single HTTP request.
    async fn handle_request(
        req: Request<Incoming>,
        state: Arc<tokio::sync::Mutex<Option<oneshot::Sender<CallbackResult>>>>,
        done: Arc<tokio::sync::Notify>,
        expected_state: String,
        handler: Arc<H>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or_default().to_string();

        match path.as_str() {
            "/auth/callback" => {
                let params = crate::oauth_http_utils::parse_urlencoded_params(&query);

                // Check for error from authorization server
                if let Some(err) = params.get("error") {
                    let desc = params
                        .get("error_description")
                        .cloned()
                        .unwrap_or_else(|| err.clone());
                    {
                        let mut guard = state.lock().await;
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(CallbackResult::AuthError(desc.clone()));
                        }
                    }
                    done.notify_one();
                    let html = handler.error_html(&desc);
                    return Ok(html_response(StatusCode::BAD_REQUEST, &html));
                }

                match handler.extract_code_and_state(&params) {
                    Ok((code, received_state)) => {
                        if let Err(e) = handler.validate_state(&expected_state, &received_state) {
                            {
                                let mut guard = state.lock().await;
                                if let Some(tx) = guard.take() {
                                    let _ = tx.send(CallbackResult::AuthError(e.to_string()));
                                }
                            }
                            done.notify_one();
                            let html = handler.error_html(&e.to_string());
                            return Ok(html_response(StatusCode::BAD_REQUEST, &html));
                        }

                        {
                            let mut guard = state.lock().await;
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(CallbackResult::Success {
                                    code,
                                    state: received_state,
                                });
                            }
                        }
                        done.notify_one();
                        Ok(html_response(StatusCode::OK, handler.success_html()))
                    }
                    Err(e) => {
                        let html = handler.error_html(&e.to_string());
                        Ok(html_response(StatusCode::BAD_REQUEST, &html))
                    }
                }
            }
            "/cancel" => {
                {
                    let mut guard = state.lock().await;
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(CallbackResult::Cancelled);
                    }
                }
                done.notify_one();
                Ok(html_response(StatusCode::OK, handler.cancelled_html()))
            }
            _ => {
                let html = handler.error_html("Not found");
                Ok(html_response(StatusCode::NOT_FOUND, &html))
            }
        }
    }
}
