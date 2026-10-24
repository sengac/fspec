use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{error, info};

use super::auth_lifecycle::LoginAttempt;
use super::codex_auth::CodexTokens;
use super::codex_oauth::{
    build_authorize_url, exchange_authorization_code, extract_account_id, generate_pkce,
    generate_state, PkceCodes, CODEX_ISSUER, OAUTH_PORT, OAUTH_TIMEOUT_MS,
};

#[path = "oauth_callback.rs"]
mod callback;
use callback::{serve_until_done, CallbackResult};

pub struct OAuthServerConfig {
    pub issuer_url: String,
    pub listener: TcpListener,
    pub open_browser: bool,
    pub timeout_ms: u64,
    pub pkce: Option<PkceCodes>,
    pub state: Option<String>,
}

pub async fn browser_oauth_login() -> Result<CodexTokens> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(OAUTH_TIMEOUT_MS);
    let attempt = LoginAttempt::begin_until(deadline).await?;
    let address = SocketAddr::from(([127, 0, 0, 1], OAUTH_PORT));
    #[cfg(feature = "test-support")]
    let address =
        super::oauth_test_support::config().map_or(address, |config| config.callback_address);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|error| anyhow!("Failed to bind OAuth server to {address}: {error}"))?;
    let config = OAuthServerConfig {
        issuer_url: CODEX_ISSUER.to_owned(),
        listener,
        open_browser: true,
        timeout_ms: OAUTH_TIMEOUT_MS,
        pkce: None,
        state: None,
    };
    #[cfg(feature = "test-support")]
    let config = match super::oauth_test_support::config() {
        Some(test) => OAuthServerConfig {
            issuer_url: test.issuer,
            open_browser: false,
            state: Some(test.state),
            ..config
        },
        None => config,
    };
    login(config, attempt, deadline).await
}

pub async fn browser_oauth_login_inner(config: OAuthServerConfig) -> Result<CodexTokens> {
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_millis(config.timeout_ms);
    login(config, LoginAttempt::begin_until(deadline).await?, deadline).await
}

async fn login(
    config: OAuthServerConfig,
    attempt: LoginAttempt,
    deadline: tokio::time::Instant,
) -> Result<CodexTokens> {
    tokio::select! {
        biased;
        () = attempt.cancelled() => Err(anyhow!("Login cancelled")),
        result = tokio::time::timeout_at(deadline, login_inner(config, &attempt)) => {
            result.map_err(|_| anyhow!("OAuth login timed out"))?
        }
    }
}

async fn login_inner(config: OAuthServerConfig, attempt: &LoginAttempt) -> Result<CodexTokens> {
    let port = config.listener.local_addr()?.port();
    let pkce = config.pkce.unwrap_or_else(generate_pkce);
    let expected_state = config.state.unwrap_or_else(generate_state);
    let redirect_uri = format!("http://localhost:{port}/auth/callback");
    let auth_url = build_authorize_url(&redirect_uri, &pkce, &expected_state);
    info!("OAuth callback server listening on http://localhost:{port}");
    if config.open_browser {
        if let Err(error) = open::that(&auth_url) {
            error!("Failed to open browser: {error}. Please open this URL manually:\n{auth_url}");
        }
    }
    let (sender, receiver) = oneshot::channel();
    let server_state = Arc::new(tokio::sync::Mutex::new(Some(sender)));
    serve_until_done(config.listener, server_state, expected_state).await?;
    let code = match receiver
        .await
        .map_err(|_| anyhow!("Callback channel closed without result"))?
    {
        CallbackResult::Success { code } => code,
        CallbackResult::Cancelled => return Err(anyhow!("Login cancelled by user")),
        CallbackResult::AuthError { error, description } => {
            return Err(anyhow!("Authorization failed: {error} - {description}"));
        }
    };
    let response = exchange_authorization_code(
        &config.issuer_url,
        &code,
        &pkce.verifier,
        Some(&redirect_uri),
    )
    .await?;
    let account_id = extract_account_id(Some(&response.id_token), Some(&response.access_token))
        .ok_or_else(|| anyhow!("Could not extract account_id from token response"))?;
    let tokens = CodexTokens {
        id_token: response.id_token,
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        account_id,
    };
    attempt
        .commit_with_expiry(&tokens, response.expires_in)
        .await?;
    info!("OAuth login successful — tokens persisted to auth.json");
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use crate::oauth_http_utils::{html_response, parse_urlencoded_params, urlencoded_decode};
    use hyper::StatusCode;

    #[test]
    fn test_parse_query_params_basic() {
        let params = parse_urlencoded_params("code=abc123&state=xyz");
        assert_eq!(params.get("code"), Some(&"abc123".to_string()));
        assert_eq!(params.get("state"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_parse_query_params_empty() {
        assert!(parse_urlencoded_params("").is_empty());
    }

    #[test]
    fn test_parse_query_params_encoded() {
        let params = parse_urlencoded_params("redirect_uri=http%3A%2F%2Flocalhost%3A1455");
        assert_eq!(
            params.get("redirect_uri"),
            Some(&"http://localhost:1455".to_string())
        );
    }

    #[test]
    fn test_urlencoded_decode_plus_as_space() {
        assert_eq!(urlencoded_decode("hello+world"), "hello world");
    }

    #[test]
    fn test_html_response_sets_content_type() {
        let response = html_response(StatusCode::OK, "<h1>Test</h1>");
        assert_eq!(
            response
                .headers()
                .get("Content-Type")
                .map(|value| value.to_str().unwrap_or_default()),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(response.status(), StatusCode::OK);
    }
}
