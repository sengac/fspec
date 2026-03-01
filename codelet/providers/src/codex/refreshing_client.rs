//! RefreshingCodexClient - HTTP middleware for Codex OAuth token management
//!
//! Wraps reqwest::Client and implements rig's HttpClientExt trait to intercept
//! every HTTP request and:
//! - Check token expiry and refresh if needed (OAuth mode)
//! - Rewrite URLs from OpenAI endpoints to Codex API endpoint
//! - Set Authorization, ChatGPT-Account-Id, and originator headers
//! - Pass through unchanged in API key mode
//!
//! PROV-016: Codex Custom Fetch - Token Refresh and API Rewriting

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::debug;

use super::codex_auth::{write_codex_auth, CodexAuthJson, CodexTokens};
use super::codex_oauth::{
    extract_account_id, rewrite_codex_url, refresh_access_token_at, TokenRefreshResponse,
};

/// Expiry buffer in seconds - refresh token this many seconds before actual expiry
/// to prevent edge cases where token expires between check and actual API call
pub const EXPIRY_BUFFER_SECS: u64 = 30;

/// Default token expiry in seconds when expires_in is not provided in token response
pub const DEFAULT_EXPIRY_SECS: u64 = 3600;

/// Internal token state for OAuth mode
#[derive(Debug, Clone)]
pub struct TokenState {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
    pub expires_at: Instant,
    pub issuer_url: String,
}

/// Token mode determines whether RefreshingCodexClient intercepts requests (OAuth)
/// or passes them through unchanged (ApiKey)
#[derive(Debug, Clone)]
pub enum TokenMode {
    /// OAuth mode: refresh tokens, rewrite URLs, set auth headers
    OAuth {
        /// Shared mutable token state protected by tokio RwLock
        token_state: Arc<RwLock<TokenState>>,
    },
    /// API key mode: pass-through to reqwest unchanged
    ApiKey,
}

/// HTTP client wrapper that intercepts requests for Codex OAuth token management.
///
/// Implements rig's `HttpClientExt` trait. In OAuth mode, every request is intercepted
/// to check token expiry, refresh if needed, rewrite URLs, and set auth headers.
/// In API key mode, requests pass through to the inner reqwest::Client unchanged.
#[derive(Debug, Clone)]
pub struct RefreshingCodexClient {
    inner: reqwest::Client,
    mode: TokenMode,
}

impl Default for RefreshingCodexClient {
    /// Default creates an API key mode client (pass-through, no interception)
    fn default() -> Self {
        Self {
            inner: reqwest::Client::default(),
            mode: TokenMode::ApiKey,
        }
    }
}

impl RefreshingCodexClient {
    /// Create a new RefreshingCodexClient in OAuth mode with initial tokens
    pub fn new_oauth(
        access_token: String,
        refresh_token: String,
        account_id: String,
        expires_in_secs: Option<u64>,
        issuer_url: String,
    ) -> Self {
        let expiry_secs = expires_in_secs.unwrap_or(DEFAULT_EXPIRY_SECS);
        let expires_at = Instant::now() + std::time::Duration::from_secs(expiry_secs);

        let token_state = TokenState {
            access_token,
            refresh_token,
            account_id,
            expires_at,
            issuer_url,
        };

        Self {
            inner: reqwest::Client::new(),
            mode: TokenMode::OAuth {
                token_state: Arc::new(RwLock::new(token_state)),
            },
        }
    }

    /// Create a new RefreshingCodexClient in API key mode (pass-through)
    pub fn new_api_key() -> Self {
        Self::default()
    }

    /// Check if the token is expired (including buffer)
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

    /// Ensure token is fresh. Uses double-check locking to avoid redundant refreshes.
    /// Returns Ok(()) if token is fresh (or was refreshed), Err if refresh failed.
    async fn ensure_fresh_token(
        token_state: &Arc<RwLock<TokenState>>,
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

            debug!("Codex access token expired, refreshing...");
            let response = refresh_access_token_at(&state.issuer_url, &state.refresh_token)
                .await
                .map_err(|e| {
                    rig::http_client::Error::Instance(
                        format!("Token refresh failed: {e}").into(),
                    )
                })?;

            // Update in-memory state
            update_token_state(&mut state, &response);

            debug!("Codex access token refreshed successfully");

            // Clone data for persistence outside the lock
            Some((state.clone(), response))
        };
        // Write lock dropped here

        // Persist to auth.json outside the lock (best-effort — don't fail the request)
        if let Some((state, response)) = persist_data {
            persist_tokens(&state, &response);
        }

        Ok(())
    }
}

/// Update in-memory token state from a refresh response.
fn update_token_state(state: &mut TokenState, response: &TokenRefreshResponse) {
    let expiry_secs = response.expires_in.unwrap_or(DEFAULT_EXPIRY_SECS);
    state.access_token = response.access_token.clone();
    state.refresh_token = response.refresh_token.clone();
    state.expires_at = Instant::now() + std::time::Duration::from_secs(expiry_secs);

    // Update account_id if extractable from new tokens
    if let Some(new_account_id) = extract_account_id(
        Some(&response.id_token),
        Some(&response.access_token),
    ) {
        state.account_id = new_account_id;
    }
}

/// Persist refreshed tokens to auth.json (best-effort).
fn persist_tokens(state: &TokenState, response: &TokenRefreshResponse) {
    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(CodexTokens {
            id_token: response.id_token.clone(),
            access_token: response.access_token.clone(),
            refresh_token: response.refresh_token.clone(),
            account_id: state.account_id.clone(),
        }),
        last_refresh: Some(chrono::Utc::now().to_rfc3339()),
    };
    if let Err(e) = write_codex_auth(&auth) {
        debug!("Failed to persist refreshed tokens to auth.json: {e}");
    }
}

/// Prepare a request for the Codex API: rewrite URL and inject headers.
/// Returns the modified request parts.
fn prepare_oauth_request<T>(
    req: http::Request<T>,
    access_token: &str,
    account_id: &str,
) -> http::Request<T> {
    let (mut parts, body) = req.into_parts();

    // Rewrite URL
    let original_url = parts.uri.to_string();
    let rewritten_url = rewrite_codex_url(&original_url);
    if rewritten_url != original_url {
        if let Ok(uri) = rewritten_url.parse() {
            parts.uri = uri;
        }
    }

    // Strip existing Authorization header (rig sets a dummy key)
    parts.headers.remove(http::header::AUTHORIZATION);

    // Inject OAuth headers
    if let Ok(val) = format!("Bearer {access_token}").parse() {
        parts.headers.insert(http::header::AUTHORIZATION, val);
    }
    if let Ok(val) = account_id.parse() {
        parts.headers.insert("ChatGPT-Account-Id", val);
    }
    if let Ok(val) = "codelet".parse() {
        parts.headers.insert("originator", val);
    }

    http::Request::from_parts(parts, body)
}

impl rig::http_client::HttpClientExt for RefreshingCodexClient {
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
                TokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token, &state.account_id)
                }
                TokenMode::ApiKey => req,
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
                TokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token, &state.account_id)
                }
                TokenMode::ApiKey => req,
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
                TokenMode::OAuth { token_state } => {
                    Self::ensure_fresh_token(token_state).await?;
                    let state = token_state.read().await;
                    prepare_oauth_request(req, &state.access_token, &state.account_id)
                }
                TokenMode::ApiKey => req,
            };
            inner.send_streaming(req).await
        }
    }
}
