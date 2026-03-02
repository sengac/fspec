//! RefreshingClaudeClient - HTTP middleware for Claude OAuth token management
//!
//! Wraps reqwest::Client and implements rig's HttpClientExt trait to intercept
//! every HTTP request and:
//! - Check token expiry and refresh if needed (OAuth mode)
//! - Strip any existing Authorization header and inject Bearer {access_token}
//! - Pass through unchanged in API key mode
//!
//! Key difference from Codex RefreshingCodexClient:
//! - NO URL rewriting (rig's AnthropicExt::build_uri handles ?beta=true)
//! - NO extra headers (no ChatGPT-Account-Id or originator)
//! - Only handles Authorization: Bearer header and token refresh
//! - Static headers (anthropic-beta, user-agent, x-app) set at rig client build time
//!
//! PROV-023: Anthropic token refresh client and resilient request auth

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::debug;

use crate::claude_auth::{write_claude_auth, ClaudeAuthJson};
use crate::claude_oauth::{calculate_expiry, refresh_access_token_at, ClaudeTokenResponse};

/// Expiry buffer in seconds - refresh token this many seconds before actual expiry
/// to prevent edge cases where token expires between check and actual API call
pub const EXPIRY_BUFFER_SECS: u64 = 30;

/// Internal token state for Claude OAuth mode
#[derive(Debug, Clone)]
pub struct ClaudeTokenState {
    pub access_token: String,
    pub refresh_token: String,
    pub token_endpoint_base: String,
    pub expires_at: Instant,
}

/// Token mode determines whether RefreshingClaudeClient intercepts requests (OAuth)
/// or passes them through unchanged (ApiKey)
#[derive(Debug, Clone)]
pub enum ClaudeTokenMode {
    /// OAuth mode: refresh tokens, set auth headers
    OAuth {
        /// Shared mutable token state protected by tokio RwLock
        token_state: Arc<RwLock<ClaudeTokenState>>,
    },
    /// API key mode: pass-through to reqwest unchanged
    ApiKey,
}

/// HTTP client wrapper that intercepts requests for Claude OAuth token management.
///
/// Implements rig's `HttpClientExt` trait. In OAuth mode, every request is intercepted
/// to check token expiry, refresh if needed, and set the Authorization: Bearer header.
/// In API key mode, requests pass through to the inner reqwest::Client unchanged.
#[derive(Debug, Clone)]
pub struct RefreshingClaudeClient {
    inner: reqwest::Client,
    mode: ClaudeTokenMode,
}

impl Default for RefreshingClaudeClient {
    /// Default creates an API key mode client (pass-through, no interception)
    fn default() -> Self {
        Self {
            inner: reqwest::Client::default(),
            mode: ClaudeTokenMode::ApiKey,
        }
    }
}

impl RefreshingClaudeClient {
    /// Create a new RefreshingClaudeClient in OAuth mode with initial tokens
    pub fn new_oauth(
        access_token: String,
        refresh_token: String,
        expires_in_secs: Option<u64>,
        token_endpoint_base: String,
    ) -> Self {
        // Claude's expires_in is always present, but we accept Option for
        // the Some(0) pattern when loading from disk (force immediate refresh)
        let expiry_secs = expires_in_secs.unwrap_or(0);
        let expires_at = Instant::now() + std::time::Duration::from_secs(expiry_secs);

        let token_state = ClaudeTokenState {
            access_token,
            refresh_token,
            token_endpoint_base,
            expires_at,
        };

        Self {
            inner: reqwest::Client::new(),
            mode: ClaudeTokenMode::OAuth {
                token_state: Arc::new(RwLock::new(token_state)),
            },
        }
    }

    /// Create a new RefreshingClaudeClient in API key mode (pass-through)
    pub fn new_api_key() -> Self {
        Self::default()
    }

    /// Check if the token is expired (including buffer)
    pub async fn is_token_expired(&self) -> bool {
        match &self.mode {
            ClaudeTokenMode::OAuth { token_state } => {
                let state = token_state.read().await;
                let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);
                Instant::now() + buffer >= state.expires_at
            }
            ClaudeTokenMode::ApiKey => false,
        }
    }

    /// Ensure token is fresh. Uses double-check locking to avoid redundant refreshes.
    /// Returns Ok(()) if token is fresh (or was refreshed), Err if refresh failed.
    async fn ensure_fresh_token(
        token_state: &Arc<RwLock<ClaudeTokenState>>,
    ) -> Result<(), rig::http_client::Error> {
        let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);

        // Read lock: check if expired
        {
            let state = token_state.read().await;
            if Instant::now() + buffer < state.expires_at {
                return Ok(()); // Token still valid
            }
        }
        // Read lock dropped

        // Write lock: double-check and refresh if still expired
        let persist_data = {
            let mut state = token_state.write().await;
            // Re-check under write lock (another task may have refreshed)
            if Instant::now() + buffer < state.expires_at {
                return Ok(()); // Another task refreshed it
            }

            debug!("Claude access token expired, refreshing...");
            let response =
                refresh_access_token_at(&state.token_endpoint_base, &state.refresh_token)
                    .await
                    .map_err(|e| {
                        rig::http_client::Error::Instance(
                            format!("Claude token refresh failed: {e}").into(),
                        )
                    })?;

            // Update in-memory state
            update_token_state(&mut state, &response);

            debug!("Claude access token refreshed successfully");

            // Clone data for persistence outside the lock
            Some(response)
        };
        // Write lock dropped here

        // Persist to claude_auth.json outside the lock (best-effort)
        // Using tokio::spawn because write_claude_auth is async
        if let Some(response) = persist_data {
            persist_tokens(&response);
        }

        Ok(())
    }
}

/// Update in-memory token state from a refresh response.
fn update_token_state(state: &mut ClaudeTokenState, response: &ClaudeTokenResponse) {
    state.access_token = response.access_token.clone();
    state.refresh_token = response.refresh_token.clone();
    state.expires_at =
        Instant::now() + std::time::Duration::from_secs(response.expires_in);
}

/// Persist refreshed tokens to claude_auth.json (best-effort, fire-and-forget).
///
/// Uses tokio::spawn because write_claude_auth is async (tokio::fs).
/// Errors are logged but don't fail the request.
fn persist_tokens(response: &ClaudeTokenResponse) {
    let auth = ClaudeAuthJson {
        access_token: response.access_token.clone(),
        refresh_token: response.refresh_token.clone(),
        expires: calculate_expiry(response.expires_in),
    };
    tokio::spawn(async move {
        if let Err(e) = write_claude_auth(&auth).await {
            debug!("Failed to persist refreshed Claude tokens to claude_auth.json: {e}");
        }
    });
}

/// Prepare a request for the Claude API: strip existing Authorization and inject Bearer.
/// Unlike Codex, no URL rewriting or extra headers needed.
fn prepare_oauth_request<T>(
    req: http::Request<T>,
    access_token: &str,
) -> http::Request<T> {
    let (mut parts, body) = req.into_parts();

    // Strip existing Authorization header (rig may set a stale token)
    parts.headers.remove(http::header::AUTHORIZATION);

    // Inject fresh Bearer token
    if let Ok(val) = format!("Bearer {access_token}").parse() {
        parts.headers.insert(http::header::AUTHORIZATION, val);
    }

    http::Request::from_parts(parts, body)
}

impl rig::http_client::HttpClientExt for RefreshingClaudeClient {
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
        let inner = self.inner.clone();
        let mode = self.mode.clone();

        // Convert T → Bytes before the async block so we don't need T: 'static
        let req = req.map(Into::into);

        async move {
            let req: http::Request<bytes::Bytes> = match &mode {
                ClaudeTokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token)
                }
                ClaudeTokenMode::ApiKey => req,
            };
            inner.send(req).await
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

        async move {
            let req = match &mode {
                ClaudeTokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token)
                }
                ClaudeTokenMode::ApiKey => req,
            };
            inner.send_multipart(req).await
        }
    }

    fn send_streaming<T>(
        &self,
        req: http::Request<T>,
    ) -> impl std::future::Future<Output = rig::http_client::Result<rig::http_client::StreamingResponse>>
           + Send
    where
        T: Into<bytes::Bytes>,
    {
        let inner = self.inner.clone();
        let mode = self.mode.clone();

        // Convert T → Bytes before the async block so we don't need T: Send
        let req = req.map(Into::into);

        async move {
            let req: http::Request<bytes::Bytes> = match &mode {
                ClaudeTokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token)
                }
                ClaudeTokenMode::ApiKey => req,
            };
            inner.send_streaming(req).await
        }
    }
}
