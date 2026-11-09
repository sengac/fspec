//! Feature: spec/features/anthropic-client-browser-authorization.feature

mod fixtures;

use codelet_providers::claude_oauth_server::{
    ClaudeBrowserAuthorization, ClaudeBrowserAuthorizationError,
};
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn capture_browser_authorization_without_claiming_token_success() -> TestResult {
    // @step Given a client-local Anthropic authorization listener
    let (home, _guard) = fixtures::setup_fspec_home()?;
    let flow = ClaudeBrowserAuthorization::bind().await?;
    let browser_url = flow.browser_url().to_owned();
    let authorize_url = flow.authorize_url().to_owned();
    let state = url::Url::parse(&authorize_url)?
        .query_pairs()
        .find(|(name, _)| name == "state")
        .or_fail("state parameter")?
        .1
        .into_owned();
    let completion = tokio::spawn(flow.complete());
    let client = reqwest::Client::new();
    // @step When I open its browser URL and submit a valid code and state
    let form = client.get(&browser_url).send().await?.text().await?;
    let raw_code = format!("test-code#{state}");
    let response = client
        .post(format!("{browser_url}submit"))
        .form(&[("code", &raw_code)])
        .send()
        .await?;
    assert!(response.status().is_success());
    // @step Then the original form links to the authorization URL
    assert!(form.contains(&authorize_url));
    assert!(form.contains("action=\"/submit\""));
    assert!(form.contains("href=\"/cancel\""));
    // @step And completion returns the code with state and matching PKCE verifier
    let (code, verifier) = completion.await??;
    assert_eq!(code, raw_code);
    assert_eq!(verifier, state);
    assert!(!home.path().join("credentials/claude_auth.json").exists());
    // @step And the browser tells me to return to fspec for the result
    let html = response.text().await?;
    assert!(html.contains("return to fspec"));
    assert!(!html.contains("Authorization Successful"));
    Ok(())
}

#[tokio::test]
async fn cancelling_pending_completion_closes_active_browser_connections() -> TestResult {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // @step Given a client-local Anthropic authorization listener with an incomplete browser request
    let flow = ClaudeBrowserAuthorization::bind().await?;
    let address = flow
        .browser_url()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    let completion = tokio::spawn(flow.complete());
    let mut connection = tokio::net::TcpStream::connect(&address).await?;
    connection
        .write_all(b"POST /submit HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n")
        .await?;
    tokio::task::yield_now().await;
    // @step When I cancel its pending completion
    completion.abort();
    assert!(completion
        .await
        .or_fail_err("cancelled task")?
        .is_cancelled());
    // @step Then its listener and active browser connection are closed
    let mut bytes = [0; 1];
    let closed = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        connection.read(&mut bytes),
    )
    .await?;
    match closed {
        Ok(length) => assert_eq!(length, 0),
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset),
    }
    let _listener = tokio::net::TcpListener::bind(address).await?;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn authorization_expires_after_five_minutes() -> TestResult {
    // @step Given a client-local Anthropic authorization listener
    let flow = ClaudeBrowserAuthorization::bind().await?;
    let address = flow
        .browser_url()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    let completion = tokio::spawn(flow.complete());
    // @step When five minutes pass without completion
    tokio::time::advance(std::time::Duration::from_secs(299)).await;
    assert!(!completion.is_finished());
    tokio::time::advance(std::time::Duration::from_secs(1)).await;
    // @step Then completion reports a typed timeout and releases the local port
    assert!(matches!(
        completion.await?,
        Err(ClaudeBrowserAuthorizationError::Timeout)
    ));
    let _listener = tokio::net::TcpListener::bind(address).await?;
    Ok(())
}

#[tokio::test]
async fn reject_missing_and_mismatched_state() -> TestResult {
    for (code, missing) in [("test-code", true), ("test-code#wrong", false)] {
        // @step Given a client-local Anthropic authorization listener
        let flow = ClaudeBrowserAuthorization::bind().await?;
        let browser_url = flow.browser_url().to_owned();
        let completion = tokio::spawn(flow.complete());
        // @step When I submit a code with missing or mismatched state
        let response = reqwest::Client::new()
            .post(format!("{browser_url}submit"))
            .form(&[("code", code)])
            .send()
            .await?;
        // @step Then the browser shows an error and completion fails with a typed error
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        let error = completion.await?.or_fail_err("reject state")?;
        if missing {
            assert!(matches!(
                error,
                ClaudeBrowserAuthorizationError::MissingState
            ));
        } else {
            assert!(matches!(
                error,
                ClaudeBrowserAuthorizationError::StateMismatch
            ));
        }
    }
    Ok(())
}

#[tokio::test]
async fn empty_submissions_and_unknown_routes_do_not_terminate_authorization() -> TestResult {
    // @step Given a client-local Anthropic authorization listener
    let flow = ClaudeBrowserAuthorization::bind().await?;
    let browser_url = flow.browser_url().to_owned();
    let completion = tokio::spawn(flow.complete());
    let client = reqwest::Client::new();
    // @step When I visit an unknown route or submit an empty code
    assert_eq!(
        client
            .get(format!("{browser_url}missing"))
            .send()
            .await?
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .post(format!("{browser_url}submit"))
            .form(&[("code", "")])
            .send()
            .await?
            .status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    // @step Then I can still cancel through the original browser cancel link
    assert!(client
        .get(format!("{browser_url}cancel"))
        .send()
        .await?
        .status()
        .is_success());
    // @step And completion reports cancellation
    assert!(matches!(
        completion.await?,
        Err(ClaudeBrowserAuthorizationError::Cancelled)
    ));
    Ok(())
}

#[tokio::test]
async fn dropping_authorization_closes_the_local_listener() -> TestResult {
    // @step Given a client-local Anthropic authorization listener
    let flow = ClaudeBrowserAuthorization::bind().await?;
    let address = flow
        .browser_url()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    // @step When I drop the pending authorization
    drop(flow);
    // @step Then its local port is released
    let _listener = tokio::net::TcpListener::bind(address).await?;
    Ok(())
}
