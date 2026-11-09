//! Feature: spec/features/codex-oauth-napi-bindings.feature
use super::*;

// =========================================================================
// Scenario: Successful browser OAuth login via NAPI
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_browser_oauth_login() {
    // @step Given the browser OAuth flow is configured with a test server
    let (_temp_dir, _guard) = codex_home();

    let mock_server = MockServer::start().await;
    let account_id = "acct_browser_flow_test";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_browser_flow", "rt_browser_flow");

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let (listener, port) = ephemeral_listener().await;
    let issuer_url = mock_server.uri();
    let known_state = "browser-flow-test-state".to_string();

    let state_clone = known_state.clone();
    let login_handle = tokio::spawn(async move {
        let config = OAuthServerConfig {
            issuer_url,
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: None,
            state: Some(state_clone),
        };
        browser_oauth_login_inner(config).await
    });

    // TEST-042: no sleep; the listener is already bound, so the request
    // waits in its backlog until the login task accepts it.

    // @step When TypeScript calls codex_oauth_browser_login()
    let callback_url =
        format!("http://127.0.0.1:{port}/auth/callback?code=flow_auth_code&state={known_state}");
    let client = reqwest::Client::new();
    let resp = client
        .get(&callback_url)
        .send()
        .await
        .expect("Should reach server");
    assert_eq!(resp.status().as_u16(), 200);

    let result = login_handle.await.expect("Task should not panic");
    assert!(
        result.is_ok(),
        "Browser login should succeed, got: {:?}",
        result.err()
    );
    let tokens = result.unwrap();

    // @step Then the Promise should resolve with NapiCodexTokens
    // @step And the tokens should contain id_token, access_token, refresh_token, and account_id
    assert!(!tokens.id_token.is_empty(), "id_token should be non-empty");
    assert_eq!(tokens.access_token, "at_browser_flow");
    assert_eq!(tokens.refresh_token, "rt_browser_flow");
    assert_eq!(tokens.account_id, account_id);

    let serialized = serde_json::to_value(&tokens).unwrap();
    assert!(serialized.get("id_token").is_some());
    assert!(serialized.get("access_token").is_some());
    assert!(serialized.get("refresh_token").is_some());
    assert!(serialized.get("account_id").is_some());
}

// =========================================================================
// Scenario: Browser OAuth login times out
// =========================================================================

#[tokio::test]
#[serial]
async fn test_browser_oauth_login_times_out() {
    // @step Given the browser OAuth flow is configured with a short timeout
    let (_temp_dir, _guard) = codex_home();

    let (listener, _port) = ephemeral_listener().await;
    let login_handle = tokio::spawn(async move {
        let config = OAuthServerConfig {
            issuer_url: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 100, // Very short timeout for test
            pkce: None,
            state: None,
        };
        browser_oauth_login_inner(config).await
    });

    // @step When TypeScript calls codex_oauth_browser_login()
    // @step And no callback is received before the timeout

    let result = login_handle.await.expect("Task should not panic");

    // @step Then the Promise should reject with an error containing "timed out"
    assert!(result.is_err(), "Browser login should fail with timeout");
    let reason = result.unwrap_err().to_string();
    assert!(
        reason.contains("timed out"),
        "Error reason should contain 'timed out': {reason}"
    );
}
