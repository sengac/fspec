use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio::time::Instant;

use super::codex_oauth::{
    build_authorize_url, generate_pkce, generate_state, PkceCodes, OAUTH_PORT, OAUTH_TIMEOUT_MS,
};
use super::codex_oauth_server::callback::{serve_until_done, CallbackResult};

pub const BROWSER_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";

#[derive(Debug, thiserror::Error)]
pub enum BrowserAuthorizationError {
    #[error("Invalid Codex authorization URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Codex authorization URL does not match its login state and PKCE verifier")]
    InvalidLoginParameters,
    #[error("Codex callback listener failed: {0}")]
    Listener(#[from] std::io::Error),
    #[error("Codex browser authorization timed out")]
    Timeout,
    #[error("Codex browser authorization cancelled")]
    Cancelled,
    #[error("Codex callback closed without authorization: {0}")]
    Channel(#[from] oneshot::error::RecvError),
    #[error("Codex authorization failed: {error}: {description}")]
    Authorization { error: String, description: String },
}

pub struct CodexBrowserAuthorization {
    listener: TcpListener,
    authorize_url: String,
    verifier: String,
    state: String,
    deadline: Instant,
}

impl CodexBrowserAuthorization {
    pub async fn bind_for_login(
        authorize_url: String,
        verifier: String,
    ) -> Result<Self, BrowserAuthorizationError> {
        let address = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, OAUTH_PORT));
        // BUG-686: tests bind their own callback address, as the
        // daemon-hosted codex_oauth_server does; the redirect URI is unchanged.
        #[cfg(feature = "test-support")]
        let address =
            super::oauth_test_support::config().map_or(address, |config| config.callback_address);
        let listener = TcpListener::bind(address).await?;
        Self::for_login(listener, authorize_url, verifier)
    }

    fn for_login(
        listener: TcpListener,
        authorize_url: String,
        verifier: String,
    ) -> Result<Self, BrowserAuthorizationError> {
        let parsed = url::Url::parse(&authorize_url)?;
        let state = parsed
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .filter(|state| !state.is_empty())
            .ok_or(BrowserAuthorizationError::InvalidLoginParameters)?;
        if verifier.is_empty()
            || authorize_url
                != build_authorize_url(
                    BROWSER_REDIRECT_URI,
                    &PkceCodes::from_verifier(verifier.clone()),
                    &state,
                )
        {
            return Err(BrowserAuthorizationError::InvalidLoginParameters);
        }
        Ok(Self {
            listener,
            authorize_url,
            verifier,
            state,
            deadline: Instant::now() + Duration::from_millis(OAUTH_TIMEOUT_MS),
        })
    }

    #[cfg(feature = "test-support")]
    pub fn from_listener_for_login(
        listener: TcpListener,
        authorize_url: String,
        verifier: String,
    ) -> Result<Self, BrowserAuthorizationError> {
        Self::for_login(listener, authorize_url, verifier)
    }

    pub async fn bind() -> Result<Self, BrowserAuthorizationError> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, OAUTH_PORT)).await?;
        Ok(Self::new(listener, Duration::from_millis(OAUTH_TIMEOUT_MS)))
    }

    fn new(listener: TcpListener, timeout: Duration) -> Self {
        let pkce = generate_pkce();
        let state = generate_state();
        Self {
            listener,
            authorize_url: build_authorize_url(BROWSER_REDIRECT_URI, &pkce, &state),
            verifier: pkce.verifier,
            state,
            deadline: Instant::now() + timeout,
        }
    }

    #[cfg(feature = "test-support")]
    pub fn from_listener(listener: TcpListener, timeout: Duration) -> Self {
        Self::new(listener, timeout)
    }

    pub fn authorize_url(&self) -> &str {
        &self.authorize_url
    }

    pub async fn complete(self) -> Result<(String, String), BrowserAuthorizationError> {
        let (sender, receiver) = oneshot::channel();
        let callback = async {
            serve_until_done(
                self.listener,
                Arc::new(Mutex::new(Some(sender))),
                self.state,
            )
            .await?;
            match receiver.await? {
                CallbackResult::Success { code } => {
                    Ok((format!("{code}#{}", self.verifier), self.verifier))
                }
                CallbackResult::Cancelled => Err(BrowserAuthorizationError::Cancelled),
                CallbackResult::AuthError { error, description } => {
                    Err(BrowserAuthorizationError::Authorization { error, description })
                }
            }
        };
        tokio::time::timeout_at(self.deadline, callback)
            .await
            .map_err(|_| BrowserAuthorizationError::Timeout)?
    }
}
