use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::codex_oauth::rewrite_codex_url;

pub const EXPIRY_BUFFER_SECS: u64 = 30;
pub const DEFAULT_EXPIRY_SECS: u64 = 3600;

pub(super) fn token_expiry(seconds: u64) -> Instant {
    let now = Instant::now();
    now.checked_add(std::time::Duration::from_secs(seconds))
        .unwrap_or_else(|| now + std::time::Duration::from_secs(DEFAULT_EXPIRY_SECS))
}

#[path = "client_http.rs"]
mod client_http;
#[path = "client_test_endpoint.rs"]
mod test_endpoint;

#[derive(Debug, Clone)]
pub struct TokenState {
    pub binding: Result<Option<super::auth_refresh::Binding>, String>,
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
    pub expires_at: Instant,
    pub issuer_url: String,
}

#[derive(Debug, Clone)]
pub enum TokenMode {
    OAuth {
        token_state: Arc<RwLock<TokenState>>,
    },
    ApiKey,
}

#[derive(Debug, Clone)]
pub struct RefreshingCodexClient {
    inner: Result<reqwest::Client, Arc<rig::http_client::Error>>,
    mode: TokenMode,
    idle_timeout: std::time::Duration,
    websocket: Option<super::websocket::CodexWebSocketHandle>,
    #[cfg(feature = "test-support")]
    test_endpoint: Option<test_endpoint::LocalEndpoint>,
}

impl Default for RefreshingCodexClient {
    fn default() -> Self {
        Self {
            inner: super::recovery_auth::http_client().map_err(Arc::new),
            mode: TokenMode::ApiKey,
            idle_timeout: super::recovery::DEFAULT_IDLE_TIMEOUT,
            websocket: None,
            #[cfg(feature = "test-support")]
            test_endpoint: None,
        }
    }
}

impl RefreshingCodexClient {
    pub(super) async fn diagnostic_redactor(
        mode: &TokenMode,
        headers: &http::HeaderMap,
    ) -> super::diagnostic_redaction::Redactor {
        let mut redactor = super::diagnostic_redaction::Redactor::from_headers(headers);
        if let TokenMode::OAuth { token_state } = mode {
            let state = token_state.read().await;
            redactor.add(&state.access_token);
            redactor.add(&state.refresh_token);
            redactor.add(&state.account_id);
        }
        redactor
    }

    pub(super) async fn redact_diagnostics(
        &self,
        headers: &http::HeaderMap,
    ) -> super::diagnostic_redaction::Redactor {
        Self::diagnostic_redactor(&self.mode, headers).await
    }

    pub fn with_websocket(mut self, handle: super::websocket::CodexWebSocketHandle) -> Self {
        self.websocket = Some(handle);
        self
    }

    pub fn new_oauth(
        access_token: String,
        refresh_token: String,
        account_id: String,
        expires_in_secs: Option<u64>,
        issuer_url: String,
    ) -> Self {
        let expires_at = token_expiry(expires_in_secs.unwrap_or(DEFAULT_EXPIRY_SECS));
        let token_state = TokenState {
            binding: super::auth_refresh::bind(&access_token, &refresh_token)
                .map_err(|error| error.to_string()),
            access_token,
            refresh_token,
            account_id,
            expires_at,
            issuer_url,
        };
        Self {
            inner: super::recovery_auth::http_client().map_err(Arc::new),
            mode: TokenMode::OAuth {
                token_state: Arc::new(RwLock::new(token_state)),
            },
            idle_timeout: super::recovery::DEFAULT_IDLE_TIMEOUT,
            websocket: None,
            #[cfg(feature = "test-support")]
            test_endpoint: None,
        }
    }

    pub(crate) fn require_stored_binding(&self) -> Result<(), super::auth_storage::AuthError> {
        if let TokenMode::OAuth { token_state } = &self.mode {
            let state = token_state
                .try_read()
                .map_err(|_| super::auth_storage::AuthError::Changed)?;
            if state
                .binding
                .as_ref()
                .map_err(|_| super::auth_storage::AuthError::Changed)?
                .is_none()
            {
                return Err(super::auth_storage::AuthError::Changed);
            }
            super::auth_refresh::validate(&state)?;
        }
        Ok(())
    }

    pub fn new_api_key() -> Self {
        Self::default()
    }

    pub fn with_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

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

    pub(super) async fn ensure_fresh_token(
        token_state: &Arc<RwLock<TokenState>>,
    ) -> Result<(), rig::http_client::Error> {
        let mut state = token_state.write().await;
        super::auth_refresh::ensure(&mut state).await
    }
}

pub(super) fn prepare_oauth_request<T>(
    request: http::Request<T>,
    state: &TokenState,
) -> http::Request<T> {
    let (mut parts, body) = request.into_parts();
    let original_url = parts.uri.to_string();
    let rewritten_url = test_endpoint::url_for_request(&original_url, &parts.extensions);
    if rewritten_url != original_url {
        if let Ok(uri) = rewritten_url.parse() {
            parts.uri = uri;
        }
    }
    parts.headers.remove(http::header::AUTHORIZATION);
    if let Ok(value) = format!("Bearer {}", state.access_token).parse() {
        parts.headers.insert(http::header::AUTHORIZATION, value);
    }
    if let Ok(value) = state.account_id.parse() {
        parts.headers.insert("ChatGPT-Account-Id", value);
    }
    if let Ok(value) = "rust".parse() {
        parts.headers.insert("originator", value);
    }
    if let Ok(Some(binding)) = &state.binding {
        parts.extensions.insert(binding.identity());
    }
    http::Request::from_parts(parts, body)
}
