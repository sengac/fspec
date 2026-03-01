#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/codex-oauth-napi-bindings.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-015:
//! NAPI Bindings for Codex OAuth Flows.
//!
//! These tests verify the NAPI binding layer by testing:
//! - NapiCodexTokens struct conversion from CodexTokens
//! - NapiDeviceAuthStartResult struct fields (all 4: user_code, verification_url, device_auth_id, interval)
//! - Async NAPI function behavior through underlying Rust functions
//! - Error conversion patterns (Rust errors → napi::Error via Error::from_reason)
//! - Two-phase device auth design (start + poll)
//! - Token refresh via wiremock (no production endpoint hits)
//!
//! Tests use:
//! - The real underlying Rust functions from codelet-providers
//! - wiremock for HTTP endpoint simulation
//! - Real JWT construction and account ID extraction
//! - Real auth.json persistence via write_codex_auth (with temp dirs)

mod fixtures;

use codelet_providers::codex::codex_auth::{
    read_codex_auth, write_codex_auth, CodexAuthJson, CodexTokens,
};
use codelet_providers::codex::codex_device_auth::{
    poll_device_token, request_device_code, DeviceCodeResponse, PollConfig, PollResult,
};
use codelet_providers::codex::codex_oauth::{
    exchange_authorization_code, extract_account_id, refresh_access_token_at, CODEX_CLIENT_ID,
};
use codelet_providers::codex::codex_oauth_server::{browser_oauth_login_inner, OAuthServerConfig};
use fixtures::{build_test_jwt, build_token_response_json, setup_codex_home};
use serial_test::serial;
use tokio::net::TcpListener as TokioTcpListener;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// NapiCodexTokens: mirrors the NAPI object struct from codex_oauth.rs.
///
/// Rule [5]: NapiCodexTokens is an #[napi(object)] struct with fields:
/// id_token, access_token, refresh_token, account_id — all strings,
/// matching the Rust CodexTokens struct.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct NapiCodexTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

impl From<CodexTokens> for NapiCodexTokens {
    fn from(tokens: CodexTokens) -> Self {
        Self {
            id_token: tokens.id_token,
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            account_id: tokens.account_id,
        }
    }
}

/// NapiDeviceAuthStartResult: mirrors the NAPI object struct returned by
/// codex_oauth_device_login_start().
///
/// Architecture note [1]: All 4 fields match the production struct exactly.
/// user_code and verification_url are displayed to the user; device_auth_id
/// and interval are passed to codex_oauth_device_login_poll().
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct NapiDeviceAuthStartResult {
    pub user_code: String,
    pub verification_url: String,
    pub device_auth_id: String,
    pub interval: f64,
}

/// Helper: bind a tokio TcpListener to port 0 (OS-assigned) and return it with its port.
async fn ephemeral_listener() -> (TokioTcpListener, u16) {
    let listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Should bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Build CodexTokens from a token response and persist to auth.json.
///
/// Mirrors the `build_and_persist_tokens` helper in the production NAPI
/// binding so tests exercise the same extract → assemble → persist flow.
fn build_and_persist_tokens(
    token_response: &codelet_providers::codex::codex_oauth::TokenRefreshResponse,
    persist_error_context: &str,
) -> NapiCodexTokens {
    let account_id = extract_account_id(
        Some(&token_response.id_token),
        Some(&token_response.access_token),
    )
    .unwrap_or_else(|| panic!("{persist_error_context}: failed to extract account_id"));

    let tokens = CodexTokens {
        id_token: token_response.id_token.clone(),
        access_token: token_response.access_token.clone(),
        refresh_token: token_response.refresh_token.clone(),
        account_id,
    };

    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(tokens.clone()),
        last_refresh: None,
    };
    write_codex_auth(&auth).unwrap_or_else(|e| panic!("{persist_error_context}: {e}"));

    NapiCodexTokens::from(tokens)
}

/// Simulate the error conversion that the NAPI binding would do.
/// Rule [4]: All NAPI functions convert Rust errors to napi::Error via
/// Error::from_reason() — TypeScript sees rejected promises with descriptive
/// error messages.
fn convert_error_to_napi_reason(err: &anyhow::Error) -> String {
    err.to_string()
}

// =========================================================================
// Scenario: Successful browser OAuth login via NAPI
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_browser_oauth_login_via_napi() {
    // @step Given the browser OAuth flow is configured with a test server
    let (_temp_dir, _guard) = setup_codex_home();

    let mock_server = MockServer::start().await;
    let account_id = "acct_napi_browser_test";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_napi_browser", "rt_napi_browser");

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let (listener, port) = ephemeral_listener().await;
    let issuer_url = mock_server.uri();
    let known_state = "napi-browser-test-state".to_string();

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

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // @step When TypeScript calls codex_oauth_browser_login()
    let callback_url = format!(
        "http://127.0.0.1:{port}/auth/callback?code=napi_auth_code&state={known_state}"
    );
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
    let napi_tokens = NapiCodexTokens::from(tokens);

    // @step And the tokens should contain id_token, access_token, refresh_token, and account_id
    assert!(!napi_tokens.id_token.is_empty(), "id_token should be non-empty");
    assert_eq!(napi_tokens.access_token, "at_napi_browser");
    assert_eq!(napi_tokens.refresh_token, "rt_napi_browser");
    assert_eq!(napi_tokens.account_id, account_id);

    let serialized = serde_json::to_value(&napi_tokens).unwrap();
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
    let (_temp_dir, _guard) = setup_codex_home();

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
    let err = result.unwrap_err();
    let napi_reason = convert_error_to_napi_reason(&err);
    assert!(
        napi_reason.contains("timed out"),
        "NAPI error reason should contain 'timed out': {napi_reason}"
    );
}

// =========================================================================
// Scenario: Device auth login start returns user code and verification URL
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_login_start_returns_user_code_and_verification_url() {
    // @step Given the device auth usercode endpoint is available
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .and(body_string_contains(format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_napi_start",
            "user_code": "NAPI-1234",
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_device_login_start()
    let device_code = request_device_code(&mock_server.uri())
        .await
        .expect("request_device_code should succeed");

    // Build NapiDeviceAuthStartResult the same way the NAPI binding would
    let verification_url = format!("{}/codex/device", mock_server.uri());
    let napi_result = NapiDeviceAuthStartResult {
        user_code: device_code.user_code.clone(),
        verification_url: verification_url.clone(),
        device_auth_id: device_code.device_auth_id.clone(),
        interval: device_code.interval as f64,
    };

    // @step Then the result should contain a user_code string
    assert_eq!(napi_result.user_code, "NAPI-1234");
    assert!(!napi_result.user_code.is_empty());

    // @step And the result should contain a verification_url string
    assert!(napi_result.verification_url.contains("/codex/device"));
    assert!(!napi_result.verification_url.is_empty());

    // Verify all 4 fields serialize correctly for NAPI transport
    let serialized = serde_json::to_value(&napi_result).unwrap();
    assert!(serialized.get("user_code").is_some());
    assert!(serialized.get("verification_url").is_some());
    assert!(serialized.get("device_auth_id").is_some());
    assert!(serialized.get("interval").is_some());
    assert_eq!(napi_result.device_auth_id, "dev_napi_start");
    assert_eq!(napi_result.interval, 5.0);
}

// =========================================================================
// Scenario: Device auth login poll resolves with tokens after user authorizes
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_login_poll_resolves_with_tokens_after_user_authorizes() {
    // @step Given a device auth flow has been started with a valid device_auth_id
    let (temp_dir, _guard) = setup_codex_home();
    let auth_path = temp_dir.path().join("auth.json");

    let mock_server = MockServer::start().await;
    let account_id = "acct_napi_device_poll";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_napi_device", "rt_napi_device");

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_napi_poll_test".to_string(),
        user_code: "POLL-5678".to_string(),
        interval: 5,
    };

    // @step And the device token endpoint will return authorization_code after polling
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_string_contains("device_auth_id=dev_napi_poll_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_string_contains("device_auth_id=dev_napi_poll_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "napi_device_auth_code",
            "code_verifier": "napi_device_verifier"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=napi_device_auth_code"))
        .and(body_string_contains("code_verifier=napi_device_verifier"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_device_login_poll with the device_auth_id and interval
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
    };
    let poll_result = poll_device_token(&poll_config, &device_code)
        .await
        .expect("poll_device_token should succeed");

    let (authorization_code, code_verifier) = match poll_result {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => (authorization_code, code_verifier),
        other => panic!("Expected PollResult::Success, got: {:?}", other),
    };

    let token_response = exchange_authorization_code(
        &uri,
        &authorization_code,
        &code_verifier,
        None, // Device auth never uses redirect_uri
    )
    .await
    .expect("Token exchange should succeed");

    // @step Then the Promise should resolve with NapiCodexTokens
    let napi_tokens =
        build_and_persist_tokens(&token_response, "Device auth token persist failed");
    assert_eq!(napi_tokens.access_token, "at_napi_device");
    assert_eq!(napi_tokens.refresh_token, "rt_napi_device");
    assert_eq!(napi_tokens.account_id, account_id);

    // @step And the tokens should be persisted to auth.json
    assert!(auth_path.exists(), "auth.json should exist after device auth");
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["tokens"]["access_token"], "at_napi_device");
    assert_eq!(persisted_json["tokens"]["account_id"], account_id);
}

// =========================================================================
// Scenario: Device auth polling fails with expired token
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_polling_fails_with_expired_token() {
    // @step Given a device auth flow has been started
    let mock_server = MockServer::start().await;

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_napi_expired".to_string(),
        user_code: "EXPD-NAPI".to_string(),
        interval: 5,
    };

    // @step And the device token endpoint will return expired_token error
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "expired_token"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_device_login_poll
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
    };
    let result = poll_device_token(&poll_config, &device_code)
        .await
        .expect("poll_device_token should return Ok with terminal error");

    // @step Then the Promise should reject with an error containing "expired"
    match result {
        PollResult::TerminalError { error } => {
            assert!(
                error.contains("expired"),
                "NAPI error reason should contain 'expired': {error}"
            );
        }
        other => panic!("Expected PollResult::TerminalError, got: {:?}", other),
    }
}

// =========================================================================
// Scenario: Token refresh returns new tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_returns_new_tokens() {
    // @step Given valid OAuth tokens exist in auth.json
    let (temp_dir, _guard) = setup_codex_home();
    let auth_path = temp_dir.path().join("auth.json");

    let original_tokens = CodexTokens {
        id_token: "original_id_token".to_string(),
        access_token: "original_access_token".to_string(),
        refresh_token: "rt_valid_for_refresh".to_string(),
        account_id: "acct_original".to_string(),
    };
    let original_auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(original_tokens),
        last_refresh: None,
    };
    write_codex_auth(&original_auth).expect("Should write initial auth");
    assert!(auth_path.exists());

    // @step And the token endpoint accepts refresh_token requests
    let mock_server = MockServer::start().await;
    let account_id = "acct_refreshed";
    let new_id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&new_id_token, "at_refreshed", "rt_refreshed");

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=rt_valid_for_refresh"))
        .and(body_string_contains(format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_refresh_token with a valid refresh token
    let token_response = refresh_access_token_at(&mock_server.uri(), "rt_valid_for_refresh")
        .await
        .expect("refresh_access_token_at should succeed");

    // @step Then the Promise should resolve with NapiCodexTokens containing a new access_token
    let napi_tokens = build_and_persist_tokens(&token_response, "Token refresh persist failed");
    assert_eq!(napi_tokens.access_token, "at_refreshed");
    assert_eq!(napi_tokens.refresh_token, "rt_refreshed");
    assert_eq!(napi_tokens.account_id, account_id);

    // @step And the refreshed tokens should be persisted to auth.json
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["tokens"]["access_token"], "at_refreshed");
    assert_eq!(persisted_json["tokens"]["refresh_token"], "rt_refreshed");
    assert_eq!(persisted_json["tokens"]["account_id"], account_id);
}

// =========================================================================
// Scenario: Token refresh fails with invalid refresh token
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_fails_with_invalid_refresh_token() {
    // @step Given the token endpoint rejects the refresh_token
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("invalid_grant: refresh token is invalid"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_refresh_token with an invalid refresh token
    let result = refresh_access_token_at(&mock_server.uri(), "rt_completely_invalid").await;

    // @step Then the Promise should reject with an error describing the failure
    assert!(result.is_err(), "Token refresh should fail with invalid token");
    let err = result.unwrap_err();
    let napi_reason = convert_error_to_napi_reason(&err);
    assert!(
        napi_reason.contains("401"),
        "Error should mention HTTP 401 status: {napi_reason}"
    );
    assert!(
        napi_reason.contains("invalid_grant"),
        "Error should contain server error body: {napi_reason}"
    );
}

// =========================================================================
// Scenario: Get tokens returns stored tokens from auth.json
// =========================================================================

#[test]
#[serial]
fn test_get_tokens_returns_stored_tokens_from_auth_json() {
    // @step Given valid OAuth tokens exist in auth.json
    let (_temp_dir, _guard) = setup_codex_home();

    let stored_tokens = CodexTokens {
        id_token: "stored_id_token_abc".to_string(),
        access_token: "stored_access_token_xyz".to_string(),
        refresh_token: "stored_refresh_token_123".to_string(),
        account_id: "acct_stored_test".to_string(),
    };
    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(stored_tokens),
        last_refresh: None,
    };
    write_codex_auth(&auth).expect("Should write auth.json");

    // @step When TypeScript calls codex_oauth_get_tokens()
    let loaded = read_codex_auth().expect("read_codex_auth should succeed");
    assert!(loaded.is_some(), "Should find auth.json");

    let loaded_auth = loaded.unwrap();
    let codex_tokens = loaded_auth.tokens.expect("Should have tokens");
    let napi_tokens = NapiCodexTokens::from(codex_tokens);

    // @step Then the result should be NapiCodexTokens with all 4 fields populated
    assert_eq!(napi_tokens.id_token, "stored_id_token_abc");
    assert_eq!(napi_tokens.access_token, "stored_access_token_xyz");
    assert_eq!(napi_tokens.refresh_token, "stored_refresh_token_123");
    assert_eq!(napi_tokens.account_id, "acct_stored_test");

    let serialized = serde_json::to_value(&napi_tokens).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().len(),
        4,
        "NapiCodexTokens should have exactly 4 fields"
    );
}

// =========================================================================
// Scenario: Get tokens returns null when no auth.json exists
// =========================================================================

#[test]
#[serial]
fn test_get_tokens_returns_null_when_no_auth_json_exists() {
    // @step Given no auth.json file exists
    let (temp_dir, _guard) = setup_codex_home();
    let auth_path = temp_dir.path().join("auth.json");
    assert!(!auth_path.exists(), "auth.json should not exist initially");

    // @step When TypeScript calls codex_oauth_get_tokens()
    let loaded = read_codex_auth().expect("read_codex_auth should not error on missing file");

    // @step Then the result should be null
    assert!(
        loaded.is_none(),
        "Should return None (null in TypeScript) when no auth.json exists"
    );
}
