#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/claude-oauth-browser-login.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-021:
//! Anthropic OAuth browser callback server and CSRF state validation.
//!
//! All tests are FULL INTEGRATION tests — no mocks of our code. They use:
//! - The real `claude_browser_oauth_login_inner()` orchestrator
//! - The real `parse_authorization_code()` + `exchange_authorization_code()` from claude_oauth.rs
//! - Real PKCE generation (oauth_crypto.rs generate_pkce)
//! - Real auth persistence via write_claude_auth (with temp dirs)
//! - wiremock for the token endpoint (simulates console.anthropic.com)
//! - Real TCP listeners for server tests (port 0 = ephemeral)

mod fixtures;

use codelet_providers::claude_auth::{read_claude_auth, write_claude_auth, ClaudeAuthJson};
use codelet_providers::claude_oauth::calculate_expiry;
use codelet_providers::claude_oauth_server::{
    claude_browser_oauth_login_inner, ClaudeOAuthServerConfig, CLAUDE_OAUTH_TIMEOUT_MS,
};
use codelet_providers::oauth_crypto::generate_pkce;
use fixtures::setup_fspec_home;
use serial_test::serial;
use tokio::net::TcpListener as TokioTcpListener;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: bind a tokio TcpListener to port 0 (OS-assigned) and return it with its port.
async fn ephemeral_listener() -> (TokioTcpListener, u16) {
    let listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Should bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Build a Claude token response JSON body for wiremock.
fn build_claude_token_response_json(
    access_token: &str,
    refresh_token: &str,
    expires_in: u64,
) -> serde_json::Value {
    serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_in": expires_in
    })
}

/// Simple percent-encoding for form data values
fn urlencoded(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                format!("{}", b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

// =========================================================================
// Scenario: Successful browser OAuth login with code paste
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_browser_oauth_login_with_code_paste() {
    // @step Given no existing Claude credentials are available
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");
    assert!(
        !auth_path.exists(),
        "claude_auth.json should not exist initially"
    );

    // @step When I initiate Claude browser OAuth login
    let pkce = generate_pkce();

    let mock_server = MockServer::start().await;
    let token_body = build_claude_token_response_json("at_happy_path", "rt_happy_path", 3600);

    // @step Then the OAuth server should start on an ephemeral port
    let (listener, port) = ephemeral_listener().await;

    // @step And a PKCE code verifier and S256 challenge should be generated
    assert!(pkce.verifier.len() >= 43);
    assert_eq!(pkce.challenge_method, "S256");

    // @step And the browser should open to the Claude authorize URL with PKCE parameters
    // (open_browser=false in test; URL construction verified by PROV-020 tests)

    // Mock the token exchange endpoint (JSON POST, not form-encoded)
    let pkce_verifier = pkce.verifier.clone();
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

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

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step And the server should serve a form page with the authorize URL as a clickable link
    let client = reqwest::Client::new();
    let form_resp = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("Form page request should reach server");
    assert_eq!(form_resp.status().as_u16(), 200);
    let form_body = form_resp.text().await.unwrap();
    assert!(
        form_body.contains("claude.ai/oauth/authorize"),
        "Form page should contain authorize URL link: {form_body}"
    );
    assert!(
        form_body.contains("<form"),
        "Form page should contain a form element: {form_body}"
    );

    // @step When the user submits an authorization code with valid state via the form
    let code_with_state = format!("test_auth_code#{pkce_verifier}");
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={}", urlencoded(&code_with_state)))
        .send()
        .await
        .expect("Submit request should reach server");

    // @step Then the code should be parsed from "code#state" format
    // @step And the state should match the PKCE verifier
    // (verified internally by the server — submit returns success HTML)
    assert_eq!(submit_resp.status().as_u16(), 200);
    let submit_body = submit_resp.text().await.unwrap();
    assert!(
        submit_body.contains("Authorization Successful"),
        "Success page should show: {submit_body}"
    );

    // @step And the code should be exchanged for tokens via JSON POST to the Anthropic token endpoint
    // (verified by wiremock expect(1))

    // @step And the tokens should be persisted to claude_auth.json with access_token, refresh_token, and expires
    // @step And the OAuth server should shut down
    // @step And the login function should return the Claude tokens
    let result = login_handle.await.expect("Task should not panic");
    assert!(
        result.is_ok(),
        "Login should succeed, got: {:?}",
        result.err()
    );

    let auth = result.unwrap();
    assert_eq!(auth.access_token, "at_happy_path");
    assert_eq!(auth.refresh_token, "rt_happy_path");
    assert!(auth.expires > 0, "expires timestamp should be set");

    // Verify tokens were persisted
    assert!(
        auth_path.exists(),
        "claude_auth.json should exist after successful login"
    );
    let auth_content = std::fs::read_to_string(&auth_path).unwrap();
    let auth_json: serde_json::Value = serde_json::from_str(&auth_content).unwrap();
    assert_eq!(auth_json["access_token"], "at_happy_path");
    assert_eq!(auth_json["refresh_token"], "rt_happy_path");
    assert!(auth_json["expires"].as_u64().unwrap() > 0);
}

// =========================================================================
// Scenario: Code paste with mismatched state is rejected as CSRF
// =========================================================================

#[tokio::test]
#[serial]
async fn test_code_paste_with_mismatched_state_is_rejected_as_csrf() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;
    // Token endpoint should NOT be called — state validation fails first
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&mock_server)
        .await;

    let (listener, port) = ephemeral_listener().await;
    let issuer_url = mock_server.uri();

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: issuer_url,
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: None, // generates fresh PKCE internally
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When the user submits a code with state that does not match the PKCE verifier
    let code_with_wrong_state = "test_auth_code#wrong_state_value";
    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={}", urlencoded(code_with_wrong_state)))
        .send()
        .await
        .expect("Submit request should reach server");

    // @step Then the server should return an HTML error page with CSRF warning
    assert_eq!(submit_resp.status().as_u16(), 400);
    let body = submit_resp.text().await.unwrap();
    assert!(
        body.contains("CSRF") || body.contains("state"),
        "Error page should mention CSRF or state: {body}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    let result = login_handle.await.expect("Task should not panic");
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after CSRF rejection"
    );

    // @step And the OAuth server should shut down
    // (handle completed)

    // @step And the login function should return a CSRF error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CSRF") || err_msg.contains("state"),
        "Error should mention CSRF or state: {err_msg}"
    );
}

// =========================================================================
// Scenario: Login times out after 5 minutes without code submission
// =========================================================================

#[tokio::test]
#[serial]
async fn test_login_times_out_after_5_minutes_without_code_submission() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (_temp_dir, _guard) = setup_fspec_home();

    let (listener, _port) = ephemeral_listener().await;

    // @step When the timeout elapses without receiving a code submission
    // (Using 100ms timeout for test speed)
    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 100, // 100ms timeout for test speed
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    // Verify production timeout is 5 minutes
    assert_eq!(
        CLAUDE_OAUTH_TIMEOUT_MS, 300_000,
        "Production timeout should be 5 minutes"
    );

    // @step Then the OAuth server should shut down cleanly
    let result = login_handle.await.expect("Task should not panic");

    // @step And the login function should return a timeout error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("timed out") || err_msg.contains("timeout"),
        "Error should mention timeout: {err_msg}"
    );
}

// =========================================================================
// Scenario: Token exchange fails after valid state validation
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_exchange_fails_after_valid_state_validation() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;

    // @step And the token exchange POST to the Anthropic token endpoint returns an error
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();
    let (listener, port) = ephemeral_listener().await;
    let issuer_url = mock_server.uri();
    let pkce_clone = pkce.clone();

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

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When the user submits a code with valid state
    let code_with_state = format!("test_auth_code#{pkce_verifier}");
    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={}", urlencoded(&code_with_state)))
        .send()
        .await
        .expect("Submit request should reach server");

    // @step Then the server should return an HTML error page
    assert_eq!(submit_resp.status().as_u16(), 400);
    let submit_body = submit_resp.text().await.unwrap();
    assert!(
        submit_body.contains("Token exchange failed")
            || submit_body.contains("Authorization Failed"),
        "Error page should indicate exchange failure: {submit_body}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    let result = login_handle.await.expect("Task should not panic");
    assert!(
        !auth_path.exists(),
        "No claude_auth.json on exchange failure"
    );

    // @step And the login function should return the exchange error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("400") || err_msg.contains("failed") || err_msg.contains("invalid_grant"),
        "Error should mention exchange failure: {err_msg}"
    );
}

// =========================================================================
// Scenario: User cancels OAuth flow via cancel route
// =========================================================================

#[tokio::test]
#[serial]
async fn test_user_cancels_oauth_flow_via_cancel_route() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (_temp_dir, _guard) = setup_fspec_home();

    let (listener, port) = ephemeral_listener().await;

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When a request is made to the /cancel route
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/cancel"))
        .send()
        .await
        .expect("Cancel request should reach server");

    // @step Then the server should return a cancel confirmation page
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("Login Cancelled") || body.contains("Cancelled"),
        "Cancel page should show cancellation: {body}"
    );

    // @step And the OAuth server should shut down
    let result = login_handle.await.expect("Task should not panic");

    // @step And the login function should return a cancellation error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("cancelled") || err_msg.contains("cancel"),
        "Error should mention cancellation: {err_msg}"
    );
}

// =========================================================================
// Scenario: Code without state hash separator is rejected
// =========================================================================

#[tokio::test]
#[serial]
async fn test_code_without_state_hash_separator_is_rejected() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let (listener, port) = ephemeral_listener().await;

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When the user submits a code without a "#" separator
    let code_without_hash = "abc123nohashhere";
    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={code_without_hash}"))
        .send()
        .await
        .expect("Submit request should reach server");

    // @step Then the server should return an HTML error page indicating missing state
    assert_eq!(submit_resp.status().as_u16(), 400);
    let body = submit_resp.text().await.unwrap();
    assert!(
        body.contains("state") || body.contains("missing") || body.contains("Authorization Failed"),
        "Error page should indicate missing state: {body}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    let result = login_handle.await.expect("Task should not panic");
    assert!(!auth_path.exists(), "No claude_auth.json should exist");

    // @step And the login function should return a missing state error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("state") || err_msg.contains("missing"),
        "Error should mention missing state: {err_msg}"
    );
}

// =========================================================================
// Scenario: Browser fails to open but server still shows form with link
// =========================================================================

#[tokio::test]
#[serial]
async fn test_browser_fails_to_open_but_server_still_shows_form_with_link() {
    // @step Given the browser open command will fail
    // (open_browser=false simulates this — the server should still function)
    let (_temp_dir, _guard) = setup_fspec_home();

    let (listener, port) = ephemeral_listener().await;

    // @step When I initiate Claude browser OAuth login
    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false, // simulates browser failure
            timeout_ms: 10_000,
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step Then the OAuth server should start and serve the form page
    let client = reqwest::Client::new();
    let form_resp = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("Form page should be accessible");

    assert_eq!(form_resp.status().as_u16(), 200);
    let form_body = form_resp.text().await.unwrap();

    // @step And the form page should contain the authorize URL as a clickable link
    assert!(
        form_body.contains("claude.ai/oauth/authorize"),
        "Form should contain authorize URL: {form_body}"
    );
    assert!(
        form_body.contains("href="),
        "Form should have a clickable link: {form_body}"
    );

    // @step And the server should log a warning with the authorize URL
    // (Logging verified by tracing subscriber in integration tests — not asserted here)

    // @step And the login flow should continue waiting for code submission
    // Server is still running — cancel it to clean up
    let cancel_resp = client
        .get(format!("http://127.0.0.1:{port}/cancel"))
        .send()
        .await
        .expect("Cancel should work");
    assert_eq!(cancel_resp.status().as_u16(), 200);

    let result = login_handle.await.expect("Task should not panic");
    assert!(result.is_err()); // cancelled
}

// =========================================================================
// Scenario: Server handles 404 requests without shutting down
// =========================================================================

#[tokio::test]
#[serial]
async fn test_server_handles_404_requests_without_shutting_down() {
    // @step Given the Claude OAuth server is running and waiting for code submission
    let (_temp_dir, _guard) = setup_fspec_home();

    let (listener, port) = ephemeral_listener().await;

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // @step When a request is made to an unknown path like "/favicon.ico"
    let resp_404 = client
        .get(format!("http://127.0.0.1:{port}/favicon.ico"))
        .send()
        .await
        .expect("404 request should reach server");

    // @step Then the server should return a 404 response
    assert_eq!(resp_404.status().as_u16(), 404);

    // @step And the server should remain running and accept further requests
    // Verify by sending another request (cancel to clean up)
    let resp_cancel = client
        .get(format!("http://127.0.0.1:{port}/cancel"))
        .send()
        .await
        .expect("Cancel should work after 404");
    assert_eq!(resp_cancel.status().as_u16(), 200);

    let result = login_handle.await.expect("Task should not panic");
    assert!(result.is_err()); // cancelled
}

// =========================================================================
// Scenario: Claude auth persistence writes correct JSON structure
// =========================================================================

#[tokio::test]
#[serial]
async fn test_claude_auth_persistence_writes_correct_json_structure() {
    // @step Given a successful token exchange has returned tokens
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let expires = calculate_expiry(3600);
    let auth = ClaudeAuthJson {
        access_token: "test_access_token".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        expires,
    };

    // @step When the tokens are persisted to claude_auth.json
    write_claude_auth(&auth)
        .await
        .expect("write_claude_auth should succeed");

    // @step Then the file should exist at the codelet config directory
    assert!(auth_path.exists(), "claude_auth.json should exist");

    let content = std::fs::read_to_string(&auth_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // @step And the JSON should contain "access_token" with the access token value
    assert_eq!(json["access_token"], "test_access_token");

    // @step And the JSON should contain "refresh_token" with the refresh token value
    assert_eq!(json["refresh_token"], "test_refresh_token");

    // @step And the JSON should contain "expires" as a millisecond timestamp in the future
    let persisted_expires = json["expires"].as_u64().unwrap();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(
        persisted_expires > now_ms,
        "expires {persisted_expires} should be in the future (now: {now_ms})"
    );

    // Also verify read_claude_auth round-trips correctly
    let read_back = read_claude_auth()
        .await
        .expect("read_claude_auth should succeed");
    assert!(read_back.is_some(), "Should find persisted auth");
    let read_auth = read_back.unwrap();
    assert_eq!(read_auth.access_token, "test_access_token");
    assert_eq!(read_auth.refresh_token, "test_refresh_token");
    assert_eq!(read_auth.expires, expires);
}
