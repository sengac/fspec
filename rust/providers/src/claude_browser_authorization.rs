use std::time::Duration;

use tokio::net::TcpListener;
use tokio::time::Instant;

use super::handler::{serve, Submission, SubmissionMode};
use super::CLAUDE_OAUTH_TIMEOUT_MS;
use crate::claude_oauth::build_authorize_url;
use crate::oauth_crypto::generate_pkce;

#[derive(Debug, thiserror::Error)]
pub enum ClaudeBrowserAuthorizationError {
    #[error("Claude OAuth listener error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Claude OAuth HTTP error: {0}")]
    Http(#[from] hyper::Error),
    #[error("Claude OAuth connection task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
    #[error("Claude OAuth login timed out after five minutes")]
    Timeout,
    #[error("Login cancelled by user")]
    Cancelled,
    #[error("Missing state in authorization code — use code#state format")]
    MissingState,
    #[error("CSRF validation failed — state mismatch")]
    StateMismatch,
    #[error("Token exchange failed: {0}")]
    Exchange(String),
    #[error("Failed to persist tokens: {0}")]
    Persistence(String),
    #[error("Unexpected Claude OAuth submission mode")]
    UnexpectedSubmission,
}

/// Client-local browser form capture. Never exchanges or persists credentials.
/// Dropping this value or its completion future closes the listener and requests.
pub struct ClaudeBrowserAuthorization {
    listener: TcpListener,
    authorize_url: String,
    browser_url: String,
    verifier: String,
    deadline: Instant,
}

impl ClaudeBrowserAuthorization {
    pub async fn bind() -> Result<Self, ClaudeBrowserAuthorizationError> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let browser_url = format!("http://{}/", listener.local_addr()?);
        let pkce = generate_pkce();
        Ok(Self {
            listener,
            authorize_url: build_authorize_url(&pkce),
            browser_url,
            verifier: pkce.verifier,
            deadline: Instant::now() + Duration::from_millis(CLAUDE_OAUTH_TIMEOUT_MS),
        })
    }

    pub fn authorize_url(&self) -> &str {
        &self.authorize_url
    }

    pub fn browser_url(&self) -> &str {
        &self.browser_url
    }

    /// Return validated code#state and PKCE verifier for daemon-side exchange.
    pub async fn complete(self) -> Result<(String, String), ClaudeBrowserAuthorizationError> {
        let submission = tokio::time::timeout_at(
            self.deadline,
            serve(
                self.listener,
                self.authorize_url,
                self.verifier.clone(),
                SubmissionMode::Capture,
            ),
        )
        .await
        .map_err(|_| ClaudeBrowserAuthorizationError::Timeout)??;
        match submission {
            Submission::Captured(code) => Ok((code, self.verifier)),
            Submission::Exchanged(_) => Err(ClaudeBrowserAuthorizationError::UnexpectedSubmission),
        }
    }
}
