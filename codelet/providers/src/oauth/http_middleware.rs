//! Generic Refreshing HTTP Client Middleware (PROV-060)
//!
//! `RefreshingHttpClient<S: TokenStrategy>` unifies the double-check locking
//! token refresh pattern used by Codex and Claude providers.

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::Instant;

use super::token_refresh::{TokenState, EXPIRY_BUFFER_SECS};

/// Strategy trait for provider-specific token refresh and request preparation.
///
/// Implementations provide the provider-specific logic for refreshing tokens
/// and modifying HTTP requests with fresh auth credentials.
pub trait TokenStrategy: Send + Sync + Clone + 'static {
    /// Extra state type carried in `TokenState` (e.g. issuer_url, account_id).
    type Extra: Clone + Send + Sync + std::fmt::Debug + 'static;

    /// Refresh the token. Returns a new `TokenState` with fresh credentials.
    fn refresh(
        &self,
        state: TokenState<Self::Extra>,
    ) -> impl std::future::Future<Output = Result<TokenState<Self::Extra>, String>> + Send;

    /// Persist refreshed tokens to disk (best-effort, fire-and-forget).
    fn persist(&self, state: &TokenState<Self::Extra>);

    /// Prepare an HTTP request with fresh auth credentials.
    /// Strips any stale Authorization header and injects the appropriate ones.
    fn prepare_request(
        &self,
        req: http::Request<bytes::Bytes>,
        state: &TokenState<Self::Extra>,
    ) -> http::Request<bytes::Bytes>;
}

/// Token mode: OAuth (with refresh) or ApiKey (pass-through).
#[derive(Debug, Clone)]
pub enum TokenMode<E: Clone + Send + Sync + std::fmt::Debug + 'static> {
    /// OAuth mode with shared mutable token state.
    OAuth {
        token_state: Arc<RwLock<TokenState<E>>>,
    },
    /// API key mode: pass-through, no interception.
    ApiKey,
}

/// Generic refreshing HTTP client middleware.
///
/// Wraps `reqwest::Client` and implements `rig::http_client::HttpClientExt`.
/// In OAuth mode, every request goes through double-check locking token refresh
/// before being modified by the strategy's `prepare_request`.
#[derive(Debug, Clone)]
pub struct RefreshingHttpClient<S: TokenStrategy> {
    inner: reqwest::Client,
    mode: TokenMode<S::Extra>,
    strategy: S,
}

impl<S: TokenStrategy> RefreshingHttpClient<S> {
    /// Create a new client in OAuth mode.
    pub fn new_oauth(strategy: S, initial_state: TokenState<S::Extra>) -> Self {
        Self {
            inner: reqwest::Client::new(),
            mode: TokenMode::OAuth {
                token_state: Arc::new(RwLock::new(initial_state)),
            },
            strategy,
        }
    }

    /// Create a new client in API key mode (pass-through).
    pub fn new_api_key(strategy: S) -> Self {
        Self {
            inner: reqwest::Client::new(),
            mode: TokenMode::ApiKey,
            strategy,
        }
    }

    /// Check if the token is expired (including buffer).
    pub async fn is_token_expired(&self) -> bool {
        match &self.mode {
            TokenMode::OAuth { token_state } => {
                let state = token_state.read().await;
                let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);
                Instant::now() + buffer >= state.expires_at
            }
            TokenMode::ApiKey => false,
        }
    }

    /// Ensure the token is fresh and prepare the request.
    async fn ensure_and_prepare(
        &self,
        req: http::Request<bytes::Bytes>,
    ) -> Result<http::Request<bytes::Bytes>, rig::http_client::Error> {
        match &self.mode {
            TokenMode::OAuth { token_state } => {
                let strategy = self.strategy.clone();
                let ts = token_state.clone();

                // Double-check locking refresh
                {
                    let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);
                    // Read lock
                    {
                        let state = ts.read().await;
                        if Instant::now() + buffer >= state.expires_at {
                            drop(state);
                            // Write lock
                            let persist_data = {
                                let mut state = ts.write().await;
                                if Instant::now() + buffer >= state.expires_at {
                                    let old = state.clone();
                                    let new_state = strategy.refresh(old).await.map_err(|e| {
                                        rig::http_client::Error::Instance(e.into())
                                    })?;
                                    *state = new_state.clone();
                                    Some(new_state)
                                } else {
                                    None
                                }
                            };
                            if let Some(new_state) = persist_data {
                                strategy.persist(&new_state);
                            }
                        }
                    }
                }

                let state = ts.read().await;
                Ok(self.strategy.prepare_request(req, &state))
            }
            TokenMode::ApiKey => Ok(req),
        }
    }
}

impl<S: TokenStrategy> rig::http_client::HttpClientExt for RefreshingHttpClient<S> {
    fn send<T, U>(
        &self,
        req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        let this = self.clone();
        let req = req.map(Into::into);
        async move {
            let req = this.ensure_and_prepare(req).await?;
            this.inner.send(req).await
        }
    }

    fn send_multipart<U>(
        &self,
        req: http::Request<rig::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
           + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        let inner = self.inner.clone();
        let mode = self.mode.clone();
        let strategy = self.strategy.clone();

        async move {
            match &mode {
                TokenMode::OAuth { token_state } => {
                    // Refresh if needed
                    {
                        let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);
                        let needs = {
                            let state = token_state.read().await;
                            Instant::now() + buffer >= state.expires_at
                        };
                        if needs {
                            let persist_data = {
                                let mut state = token_state.write().await;
                                if Instant::now() + buffer >= state.expires_at {
                                    let old = state.clone();
                                    let new_state = strategy.refresh(old).await.map_err(|e| {
                                        rig::http_client::Error::Instance(e.into())
                                    })?;
                                    *state = new_state.clone();
                                    Some(new_state)
                                } else {
                                    None
                                }
                            };
                            if let Some(new_state) = persist_data {
                                strategy.persist(&new_state);
                            }
                        }
                    }

                    // Apply headers to multipart request
                    let state = token_state.read().await;
                    let (mut parts, body) = req.into_parts();
                    // Strip and re-inject auth header
                    parts.headers.remove(http::header::AUTHORIZATION);
                    if let Ok(val) = format!("Bearer {}", state.access_token).parse() {
                        parts.headers.insert(http::header::AUTHORIZATION, val);
                    }
                    let req = http::Request::from_parts(parts, body);
                    inner.send_multipart(req).await
                }
                TokenMode::ApiKey => inner.send_multipart(req).await,
            }
        }
    }

    fn send_streaming<T>(
        &self,
        req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<rig::http_client::StreamingResponse>,
    > + Send
    where
        T: Into<bytes::Bytes>,
    {
        let this = self.clone();
        let req = req.map(Into::into);
        async move {
            let req = this.ensure_and_prepare(req).await?;
            this.inner.send_streaming(req).await
        }
    }
}
