#![cfg(feature = "test-support")]

use codelet_providers::codex::codex_browser_authorization::{
    BrowserAuthorizationError, CodexBrowserAuthorization,
};
use codelet_test_helpers::{setup_temp_directory, OrFailErr};
use std::time::Duration;
use tokio::net::TcpListener;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn binds_daemon_authorization() -> TestResult {
    // @step Given the daemon issued a URL with independent state and a matching PKCE verifier
    use codelet_providers::codex::codex_oauth::{
        build_authorize_url, generate_pkce, generate_state,
    };
    let pkce = generate_pkce();
    let state = generate_state();
    let url = build_authorize_url("http://localhost:1455/auth/callback", &pkce, &state);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let authorization = CodexBrowserAuthorization::from_listener_for_login(
        listener,
        url.clone(),
        pkce.verifier.clone(),
    )?;
    assert_eq!(authorization.authorize_url(), url);
    let completion = tokio::spawn(authorization.complete());
    // @step When the client receives the matching browser callback
    assert_eq!(
        reqwest::get(format!(
            "http://{address}/auth/callback?code=daemon-code&state={state}"
        ))
        .await?
        .status(),
        200
    );
    // @step Then it returns the code with that original verifier without replacing the authorization
    assert_eq!(
        completion.await??,
        (format!("daemon-code#{}", pkce.verifier), pkce.verifier)
    );
    Ok(())
}

async fn authorization(
) -> Result<(CodexBrowserAuthorization, String, String), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let authorization = CodexBrowserAuthorization::from_listener(listener, Duration::from_secs(30));
    let url = reqwest::Url::parse(authorization.authorize_url())?;
    let state = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .ok_or("missing state")?
        .1
        .into_owned();
    assert!(url.query_pairs().any(
        |(key, value)| key == "redirect_uri" && value == "http://localhost:1455/auth/callback"
    ));
    Ok((authorization, address, state))
}

#[tokio::test]
#[serial_test::serial]
async fn valid_callback_returns_material_without_tokens() -> TestResult {
    // @step Given a client callback listener with a fresh authorization URL
    let home = setup_temp_directory()?;
    let previous = std::env::var_os("CODEX_HOME");
    std::env::set_var("CODEX_HOME", home.path());
    let (authorization, address, state) = authorization().await?;
    let completion = tokio::spawn(authorization.complete());
    // @step When the browser returns a code with the matching random state
    let response = reqwest::get(format!(
        "{address}/auth/callback?code=browser-code&state={state}"
    ))
    .await?;
    assert_eq!(response.status(), 200);
    let (code, verifier) = completion.await??;
    // @step Then completion returns code and verifier for the daemon
    assert_eq!(code, format!("browser-code#{verifier}"));
    // @step And the browser state differs from the verifier and no credentials are written
    assert_ne!(state, verifier);
    assert_eq!(std::fs::read_dir(home.path())?.count(), 0);
    match previous {
        Some(value) => std::env::set_var("CODEX_HOME", value),
        None => std::env::remove_var("CODEX_HOME"),
    }
    Ok(())
}

#[tokio::test]
async fn malicious_callback_is_rejected_before_legitimate_callback() -> TestResult {
    // @step Given a client callback listener with a fresh authorization URL
    let (authorization, address, state) = authorization().await?;
    let completion = tokio::spawn(authorization.complete());
    // @step When a callback uses a different state
    let response =
        reqwest::get(format!("{address}/auth/callback?code=attacker&state=wrong")).await?;
    // @step Then it is rejected and the listener still accepts the legitimate callback
    assert_eq!(response.status(), 400);
    assert!(!completion.is_finished());
    let response = reqwest::get(format!(
        "{address}/auth/callback?code=legitimate&state={state}"
    ))
    .await?;
    assert_eq!(response.status(), 200);
    let (code, verifier) = completion.await??;
    assert_eq!(code, format!("legitimate#{verifier}"));
    Ok(())
}

#[tokio::test]
async fn cancellation_releases_listener() -> TestResult {
    // @step Given a client callback listener with a fresh authorization URL
    let (authorization, address, _) = authorization().await?;
    let completion = tokio::spawn(authorization.complete());
    // @step When the browser cancels authorization
    assert_eq!(
        reqwest::get(format!("{address}/cancel")).await?.status(),
        200
    );
    // @step Then completion reports cancellation and the listener is released
    assert!(matches!(
        completion.await?,
        Err(BrowserAuthorizationError::Cancelled)
    ));
    let _rebound = TcpListener::bind(address.trim_start_matches("http://")).await?;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn timeout_and_drop_release_listener() -> TestResult {
    // @step Given a client callback listener with a bounded authorization deadline
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let authorization = CodexBrowserAuthorization::from_listener(listener, Duration::from_secs(1));
    // @step When authorization times out or its owning future is dropped
    assert!(matches!(
        authorization.complete().await,
        Err(BrowserAuthorizationError::Timeout)
    ));
    let listener = TcpListener::bind(address).await?;
    let authorization = CodexBrowserAuthorization::from_listener(listener, Duration::from_secs(30));
    let task = tokio::spawn(authorization.complete());
    tokio::task::yield_now().await;
    task.abort();
    assert!(task
        .await
        .or_fail_err("aborted callback task")?
        .is_cancelled());
    // @step Then the listener is released without storing credentials
    let _rebound = TcpListener::bind(address).await?;
    Ok(())
}
