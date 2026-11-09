//! Browser-hosted Anthropic authorization form and standalone login.

#[path = "claude_browser_authorization.rs"]
mod authorization;
#[path = "claude_oauth_form.rs"]
mod form;
#[path = "claude_oauth_handler.rs"]
mod handler;

pub use authorization::{ClaudeBrowserAuthorization, ClaudeBrowserAuthorizationError};

use anyhow::Result;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::claude_auth::ClaudeAuthJson;
use crate::claude_oauth::build_authorize_url;
use crate::oauth_crypto::{generate_pkce, PkceCodes};
use handler::{serve, Submission, SubmissionMode};

pub const CLAUDE_OAUTH_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Configuration for standalone login, including injectable test endpoints.
pub struct ClaudeOAuthServerConfig {
    pub token_endpoint_base: String,
    pub listener: TcpListener,
    pub open_browser: bool,
    pub timeout_ms: u64,
    pub pkce: Option<PkceCodes>,
}

pub async fn claude_browser_oauth_login() -> Result<ClaudeAuthJson> {
    claude_browser_oauth_login_inner(ClaudeOAuthServerConfig {
        listener: TcpListener::bind("127.0.0.1:0").await?,
        token_endpoint_base: "https://console.anthropic.com".to_owned(),
        open_browser: true,
        timeout_ms: CLAUDE_OAUTH_TIMEOUT_MS,
        pkce: None,
    })
    .await
}

pub async fn claude_browser_oauth_login_inner(
    config: ClaudeOAuthServerConfig,
) -> Result<ClaudeAuthJson> {
    let port = config.listener.local_addr()?.port();
    let pkce = match config.pkce {
        Some(pkce) => pkce,
        None => generate_pkce(),
    };
    let authorize_url = build_authorize_url(&pkce);
    let form_url = format!("http://localhost:{port}/");
    info!("Claude OAuth server listening on {form_url}");
    if config.open_browser {
        if let Err(error) = open::that(&form_url) {
            warn!("Failed to open browser: {error}. Please open this URL manually: {form_url}");
        }
    }
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        serve(
            config.listener,
            authorize_url,
            pkce.verifier,
            SubmissionMode::Exchange(config.token_endpoint_base),
        ),
    )
    .await
    .map_err(|_| ClaudeBrowserAuthorizationError::Timeout)??;
    match result {
        Submission::Exchanged(auth) => Ok(auth),
        Submission::Captured(_) => {
            Err(ClaudeBrowserAuthorizationError::UnexpectedSubmission.into())
        }
    }
}
