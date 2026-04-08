#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect,
    clippy::type_complexity
)]
//! Feature: spec/features/claude-headless-login.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-022:
//! Anthropic device auth flow for headless login.
//!
//! All tests are FULL INTEGRATION tests — no mocks of our code. They use:
//! - The real `claude_headless_login()` orchestrator
//! - Real PKCE generation (oauth_crypto.rs generate_pkce)
//! - Real auth persistence via write_claude_auth (with temp dirs)
//! - wiremock for the token endpoint (simulates console.anthropic.com)
//! - Mock async callbacks for code entry (simulates user pasting code)

mod fixtures;

use codelet_providers::claude_auth::ClaudeAuthJson;
use codelet_providers::claude_headless_login::{claude_headless_login, ClaudeHeadlessLoginConfig};
use codelet_providers::oauth_crypto::generate_pkce;
use fixtures::setup_fspec_home;
use serial_test::serial;
use std::future::Future;
use std::pin::Pin;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Create a code-entry callback that immediately returns the given string.
fn immediate_callback(
    code: String,
) -> Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> + Send>
{
    Box::new(move |_authorize_url: String| Box::pin(async move { Ok(code) }))
}

/// Create a code-entry callback that captures the authorize URL it receives.
fn capturing_callback(
    code: String,
    captured_url: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) -> Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> + Send>
{
    Box::new(move |authorize_url: String| {
        captured_url.lock().unwrap().replace(authorize_url);
        Box::pin(async move { Ok(code) })
    })
}

/// Create a code-entry callback that blocks forever (for timeout tests).
fn blocking_callback(
) -> Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> + Send>
{
    Box::new(move |_authorize_url: String| {
        Box::pin(async move {
            // Block indefinitely — will be cancelled by timeout
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            Ok("should_never_reach_this".to_string())
        })
    })
}

// =========================================================================
// Scenario: Successful headless login with code paste
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_headless_login_with_code_paste() {
    // @step Given no Claude credentials exist in claude_auth.json
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");
    assert!(
        !auth_path.exists(),
        "claude_auth.json should not exist initially"
    );

    let mock_server = MockServer::start().await;
    let token_body =
        build_claude_token_response_json("at_headless_happy", "rt_headless_happy", 3600);

    // @step When the user initiates headless Claude login
    // @step Then PKCE codes should be generated and an authorize URL built
    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();

    // Mock the JSON token exchange endpoint
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .and(body_string_contains("grant_type"))
        .and(body_string_contains("authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And the code-entry callback should receive the authorize URL
    let captured_url = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));

    // @step When the callback returns a valid code#state string
    let code_with_state = format!("authcode123#{pkce_verifier}");
    let callback = capturing_callback(code_with_state, captured_url.clone());

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: Some(pkce),
        code_entry_fn: callback,
    };

    let result = claude_headless_login(config).await;

    // @step Then the state should be validated against the PKCE verifier
    // (Internal validation — if state didn't match, we'd get a CSRF error)

    // @step And the authorization code should be exchanged for tokens via JSON POST
    // (Verified by wiremock expect(1))

    // @step And the tokens should be persisted to claude_auth.json with access_token, refresh_token, and expires
    assert!(
        result.is_ok(),
        "Headless login should succeed, got: {:?}",
        result.err()
    );
    let auth = result.unwrap();
    assert_eq!(auth.access_token, "at_headless_happy");
    assert_eq!(auth.refresh_token, "rt_headless_happy");
    assert!(auth.expires > 0, "expires timestamp should be set");

    // Verify the callback received the authorize URL
    let url = captured_url.lock().unwrap().clone();
    assert!(
        url.is_some(),
        "Callback should have received an authorize URL"
    );
    let url = url.unwrap();
    assert!(
        url.contains("claude.ai/oauth/authorize"),
        "URL should be the Claude authorize URL: {url}"
    );
    assert!(
        url.contains("code_challenge"),
        "URL should contain PKCE challenge: {url}"
    );

    // @step And the function should return a ClaudeAuthJson
    // Verify persistence to file
    assert!(
        auth_path.exists(),
        "claude_auth.json should exist after successful login"
    );
    let content = std::fs::read_to_string(&auth_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["access_token"], "at_headless_happy");
    assert_eq!(json["refresh_token"], "rt_headless_happy");
    assert!(json["expires"].as_u64().unwrap() > 0);
}

// =========================================================================
// Scenario: Code paste with mismatched state is rejected as CSRF
// =========================================================================

#[tokio::test]
#[serial]
async fn test_code_paste_with_mismatched_state_is_rejected_as_csrf() {
    // @step Given a headless Claude login is in progress
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;
    // Token endpoint should NOT be called — state validation fails first
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&mock_server)
        .await;

    // @step When the callback returns a code#state string with an incorrect state value
    let callback = immediate_callback("some_code#wrong_state_value".to_string());

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: None, // generates fresh PKCE internally — state will not match "wrong_state_value"
        code_entry_fn: callback,
    };

    let result = claude_headless_login(config).await;

    // @step Then the login should fail with a CSRF state mismatch error
    assert!(result.is_err(), "Login should fail with CSRF error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CSRF") || err_msg.contains("state") || err_msg.contains("mismatch"),
        "Error should mention CSRF or state mismatch: {err_msg}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after CSRF rejection"
    );
}

// =========================================================================
// Scenario: Code without state hash separator is rejected
// =========================================================================

#[tokio::test]
#[serial]
async fn test_code_without_state_hash_separator_is_rejected() {
    // @step Given a headless Claude login is in progress
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;
    // Token endpoint should NOT be called
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&mock_server)
        .await;

    // @step When the callback returns a code without a hash separator
    let callback = immediate_callback("code_without_hash_separator".to_string());

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: None,
        code_entry_fn: callback,
    };

    let result = claude_headless_login(config).await;

    // @step Then the login should fail with a missing state error
    assert!(
        result.is_err(),
        "Login should fail with missing state error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("state") || err_msg.contains("missing") || err_msg.contains("#"),
        "Error should mention missing state: {err_msg}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after missing state"
    );
}

// =========================================================================
// Scenario: Headless login times out when callback blocks
// =========================================================================

#[tokio::test]
#[serial]
async fn test_headless_login_times_out_when_callback_blocks() {
    // @step Given a headless Claude login is configured with a short timeout
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    // @step When the code-entry callback blocks without returning
    let callback = blocking_callback();

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: "http://127.0.0.1:1".to_string(), // won't be reached
        timeout_ms: 200,                                       // very short timeout
        pkce: None,
        code_entry_fn: callback,
    };

    let result = claude_headless_login(config).await;

    // @step Then the login should fail with a timeout error
    assert!(result.is_err(), "Login should fail with timeout");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("timed out") || err_msg.contains("timeout"),
        "Error should mention timeout: {err_msg}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after timeout"
    );
}

// =========================================================================
// Scenario: Token exchange failure after valid state validation
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_exchange_failure_after_valid_state_validation() {
    // @step Given a headless Claude login is in progress
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;

    // Token exchange will fail with 400
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();

    // @step When the callback returns a valid code#state string
    let code_with_state = format!("valid_code#{pkce_verifier}");
    let callback = immediate_callback(code_with_state);

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: Some(pkce),
        code_entry_fn: callback,
    };

    // @step And the state validates successfully
    // @step But the token exchange endpoint returns an error
    let result = claude_headless_login(config).await;

    // @step Then the login should fail with a token exchange error
    assert!(result.is_err(), "Login should fail with exchange error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("400")
            || err_msg.contains("failed")
            || err_msg.contains("exchange")
            || err_msg.contains("invalid_grant"),
        "Error should mention exchange failure: {err_msg}"
    );

    // @step And no tokens should be persisted to claude_auth.json
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after exchange failure"
    );
}

// =========================================================================
// Scenario: Empty code string is rejected before validation
// =========================================================================

#[tokio::test]
#[serial]
async fn test_empty_code_string_is_rejected_before_validation() {
    // @step Given a headless Claude login is in progress
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let mock_server = MockServer::start().await;
    // Token endpoint should NOT be called — empty check fails first
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&mock_server)
        .await;

    // @step When the callback returns an empty string
    let callback = immediate_callback(String::new());

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: None,
        code_entry_fn: callback,
    };

    let result = claude_headless_login(config).await;

    // @step Then the login should fail with a descriptive error about empty code
    assert!(result.is_err(), "Login should fail with empty code error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("empty") || err_msg.contains("Empty") || err_msg.contains("no code"),
        "Error should mention empty code: {err_msg}"
    );

    // @step And no state validation or token exchange should be attempted
    // (Verified by wiremock expect(0) — no token exchange calls)
    assert!(
        !auth_path.exists(),
        "No claude_auth.json should exist after empty code"
    );
}

// =========================================================================
// Scenario: Headless login produces same ClaudeAuthJson output as browser OAuth
// =========================================================================

#[tokio::test]
#[serial]
async fn test_headless_login_produces_same_claude_auth_json_output_as_browser_oauth() {
    // @step Given a headless Claude login completes successfully
    let (_temp_dir, _guard) = setup_fspec_home();

    let mock_server = MockServer::start().await;
    let token_body = build_claude_token_response_json("at_struct_test", "rt_struct_test", 7200);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();
    let code_with_state = format!("struct_test_code#{pkce_verifier}");
    let callback = immediate_callback(code_with_state);

    let config = ClaudeHeadlessLoginConfig {
        token_endpoint_base: mock_server.uri(),
        timeout_ms: 10_000,
        pkce: Some(pkce),
        code_entry_fn: callback,
    };

    // @step When the tokens are returned
    let result = claude_headless_login(config).await;
    assert!(
        result.is_ok(),
        "Headless login should succeed: {:?}",
        result.err()
    );
    let auth = result.unwrap();

    // @step Then the output should be a ClaudeAuthJson with access_token, refresh_token, and expires
    assert!(
        !auth.access_token.is_empty(),
        "access_token should be non-empty"
    );
    assert!(
        !auth.refresh_token.is_empty(),
        "refresh_token should be non-empty"
    );
    assert!(auth.expires > 0, "expires should be a positive timestamp");

    // Verify specific values
    assert_eq!(auth.access_token, "at_struct_test");
    assert_eq!(auth.refresh_token, "rt_struct_test");

    // @step And the output should be identical in structure to browser OAuth login output from PROV-021
    // Verify the type IS ClaudeAuthJson — same struct used by browser OAuth (PROV-021)
    // This is a compile-time check: the function returns Result<ClaudeAuthJson>
    let _: ClaudeAuthJson = auth;

    // Verify all three fields are present in serialized form (identical structure to browser OAuth)
    let serialized = serde_json::to_value(&auth).unwrap();
    assert!(
        serialized.get("access_token").is_some(),
        "Should have access_token"
    );
    assert!(
        serialized.get("refresh_token").is_some(),
        "Should have refresh_token"
    );
    assert!(serialized.get("expires").is_some(), "Should have expires");

    // Verify expires is in the future (calculated from expires_in=7200 seconds)
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(
        auth.expires > now_ms,
        "expires {} should be in the future (now: {})",
        auth.expires,
        now_ms
    );
}
