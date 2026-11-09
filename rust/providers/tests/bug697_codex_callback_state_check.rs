#![cfg(feature = "test-support")]
//! Feature: spec/features/codex-oauth-callback-state-check.feature
//!
//! BUG-697: the Codex OAuth callback handler acted on `error` before it
//! checked `state`, so any local page could end a pending sign-in with a
//! forged `/auth/callback?error=...`. Both listeners (the client one the TUI
//! hosts and the daemon-hosted `codex_oauth_server`) serve the same handler.

mod fixtures;

use codelet_providers::codex::codex_browser_authorization::{
    BrowserAuthorizationError, CodexBrowserAuthorization,
};
use codelet_providers::codex::codex_oauth_server::{browser_oauth_login_inner, OAuthServerConfig};
use fixtures::setup_codex_home;
use std::time::Duration;
use tokio::net::TcpListener;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn assert_csrf_rejection(url: String) -> TestResult {
    let response = reqwest::get(url).await?;
    assert_eq!(response.status(), 400);
    let body = response.text().await?;
    assert!(body.contains("CSRF"), "expected the CSRF page, got: {body}");
    Ok(())
}

#[tokio::test]
async fn client_listener_ignores_error_callbacks_without_the_login_state() -> TestResult {
    // @step Given a client callback listener with a fresh authorization URL
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let authorization = CodexBrowserAuthorization::from_listener(listener, Duration::from_secs(30));
    let state = reqwest::Url::parse(authorization.authorize_url())?
        .query_pairs()
        .find(|(key, _)| key == "state")
        .ok_or("missing state")?
        .1
        .into_owned();
    let completion = tokio::spawn(authorization.complete());

    // @step When the browser returns an error callback without a state and then one with a different state
    // @step Then each answers 400 with the CSRF page and the authorization keeps waiting
    assert_csrf_rejection(format!("{address}/auth/callback?error=access_denied")).await?;
    assert_csrf_rejection(format!(
        "{address}/auth/callback?error=access_denied&state=forged-{state}"
    ))
    .await?;

    // @step And the legitimate callback with the login's state still completes it
    let response = reqwest::get(format!(
        "{address}/auth/callback?code=legitimate&state={state}"
    ))
    .await?;
    assert_eq!(response.status(), 200);
    match completion.await? {
        Ok((code, verifier)) => assert_eq!(code, format!("legitimate#{verifier}")),
        Err(BrowserAuthorizationError::Authorization { error, description }) => {
            return Err(
                format!("a forged error callback ended the login: {error} {description}").into(),
            )
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[tokio::test]
async fn daemon_listener_ignores_error_callback_with_a_different_state() -> TestResult {
    // @step Given the daemon-hosted OAuth server is waiting for its callback with a known state
    let (home, _guard) = setup_codex_home()?;
    let auth_path = home.path().join("auth.json");
    let issuer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&issuer)
        .await;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let config = OAuthServerConfig {
        issuer_url: issuer.uri(),
        listener,
        open_browser: false,
        timeout_ms: 300_000,
        pkce: None,
        state: Some("expected-state".to_owned()),
    };
    let login = tokio::spawn(browser_oauth_login_inner(config));

    // @step When an error callback arrives with a different state
    // @step Then it answers 400 with the CSRF page, nothing is exchanged or persisted and the login keeps waiting
    assert_csrf_rejection(format!(
        "{address}/auth/callback?error=access_denied&error_description=forged&state=wrong-state"
    ))
    .await?;
    assert!(!auth_path.exists());

    // @step And an error callback with the login's state ends the login with that authorization error
    let response = reqwest::get(format!(
        "{address}/auth/callback?error=access_denied&error_description=User%20denied&state=expected-state"
    ))
    .await?;
    assert_eq!(response.status(), 400);
    let error = match login.await? {
        Ok(_) => return Err("the login succeeded without a code".into()),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("access_denied"), "{error}");
    assert!(error.contains("User denied"), "{error}");
    assert!(
        !error.contains("forged"),
        "the forged callback ended the login: {error}"
    );
    assert!(!auth_path.exists());
    issuer.verify().await;
    Ok(())
}
