//! Feature: spec/features/claude-oauth-napi-bindings.feature
use super::*;

// =========================================================================
// Scenario: Successful browser OAuth login via NAPI
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_browser_oauth_login() {
    // @step Given the Claude browser OAuth flow is configured with a test server
    let (_temp_dir, _guard) = fspec_home();

    let mock_server = MockServer::start().await;
    let token_body =
        build_claude_token_response_json("at_claude_browser", "rt_claude_browser", 3600);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let (listener, port) = ephemeral_listener().await;
    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();
    let pkce_clone = pkce.clone();
    let issuer_url = mock_server.uri();

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: issuer_url,
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: Some(pkce_clone),
        };
        claude_browser_oauth_login_inner(config).await
    });

    // TEST-042: no sleep; the listener is already bound, so the request
    // waits in its backlog until the login task accepts it.

    // @step When TypeScript calls claude_oauth_browser_login()
    // Simulate the user pasting code#state into the form
    let code_with_state = format!("test_auth_code#{pkce_verifier}");
    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={}", urlencoded(&code_with_state)))
        .send()
        .await
        .expect("Submit request should reach server");
    assert_eq!(submit_resp.status().as_u16(), 200);

    let result = login_handle.await.expect("Task should not panic");
    assert!(
        result.is_ok(),
        "Browser login should succeed, got: {:?}",
        result.err()
    );
    let auth = result.unwrap();

    // @step Then the Promise should resolve with NapiClaudeTokens
    // @step And the tokens should contain access_token, refresh_token, and expires
    assert_eq!(auth.access_token, "at_claude_browser");
    assert_eq!(auth.refresh_token, "rt_claude_browser");
    assert!(auth.expires > 0, "expires should be a positive timestamp");

    let serialized = serde_json::to_value(&auth).unwrap();
    assert!(serialized.get("access_token").is_some());
    assert!(serialized.get("refresh_token").is_some());
    assert!(serialized.get("expires").is_some());
}

// =========================================================================
// Scenario: Browser OAuth login times out
// =========================================================================

#[tokio::test]
#[serial]
async fn test_browser_oauth_login_times_out() {
    // @step Given the Claude browser OAuth flow is configured with a short timeout
    let (_temp_dir, _guard) = fspec_home();

    let (listener, _port) = ephemeral_listener().await;
    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 100, // Very short timeout for test
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    // @step When TypeScript calls claude_oauth_browser_login()
    // @step And no code is submitted before the timeout

    let result = login_handle.await.expect("Task should not panic");

    // @step Then the Promise should reject with an error containing "timed out"
    assert!(result.is_err(), "Browser login should fail with timeout");
    let reason = result.unwrap_err().to_string();
    assert!(
        reason.contains("timed out"),
        "Error reason should contain 'timed out': {reason}"
    );
}
