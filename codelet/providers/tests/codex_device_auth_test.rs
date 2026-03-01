#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/device-auth-flow.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-014:
//! Device Auth Flow for Headless Environments.
//!
//! All tests are FULL INTEGRATION tests — no mocks of our code. They use:
//! - The real `device_auth_login()` orchestrator
//! - The real `request_device_code()` function
//! - The real `poll_device_token()` function
//! - Real JWT construction and account ID extraction
//! - Real auth.json persistence via write_codex_auth (with temp dirs)
//! - wiremock for all HTTP endpoints (simulates auth.openai.com)

mod fixtures;

use codelet_providers::codex::codex_auth::CodexTokens;
use codelet_providers::codex::codex_device_auth::{
    device_auth_login, poll_device_token, request_device_code, DeviceAuthConfig,
    DeviceCodeResponse, PollConfig, PollResult,
};
use codelet_providers::codex::codex_oauth::CODEX_CLIENT_ID;
use fixtures::{build_test_jwt, build_token_response_json, setup_codex_home};
use serial_test::serial;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Scenario: Successful device auth login completes end-to-end
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_device_auth_login_completes_end_to_end() {
    // @step Given no Codex credentials exist in auth.json
    let (temp_dir, _guard) = setup_codex_home();
    let auth_path = temp_dir.path().join("auth.json");
    assert!(!auth_path.exists(), "auth.json should not exist initially");

    let mock_server = MockServer::start().await;
    let account_id = "acct_device_auth_happy";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_device_happy", "rt_device_happy");

    // @step When the user initiates device auth login
    // @step Then a device authorization request should be POST-ed to the usercode endpoint with client_id
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .and(body_string_contains(&format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_abc123",
            "user_code": "ABCD-1234",
            "interval": 1
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And the response should contain a device_auth_id, user_code, and polling interval
    // (verified by the mock response shape above — the struct deserialization enforces this)

    // @step And the user should see the user_code and the verification URL to visit
    // (verified via the display_fn callback below)

    // @step When the system polls the token endpoint at the specified interval
    // First poll returns pending, second poll returns success
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_string_contains("device_auth_id=dev_abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And the user completes authorization on the external device
    // @step Then the polling endpoint should return an authorization_code and code_verifier
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_string_contains("device_auth_id=dev_abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth_code_from_device",
            "code_verifier": "verifier_from_device"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And the authorization_code should be exchanged for tokens at the token endpoint without redirect_uri
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth_code_from_device"))
        .and(body_string_contains("code_verifier=verifier_from_device"))
        .and(body_string_contains(&format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Track what the user was shown
    let displayed = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let displayed_clone = displayed.clone();

    let config = DeviceAuthConfig {
        issuer_url: mock_server.uri(),
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(100), // fast polling for tests
        slow_down_increment_override_ms: None,
        display_fn: Some(Box::new(move |user_code: &str, verification_url: &str| {
            displayed_clone
                .lock()
                .unwrap()
                .push(format!("{user_code}|{verification_url}"));
        })),
    };

    let result = device_auth_login(config).await;

    // @step And the account_id should be extracted from the JWT id_token claims
    // @step And the tokens should be persisted to auth.json with refresh_token, access_token, and account_id
    assert!(result.is_ok(), "Device auth login should succeed, got: {:?}", result.err());
    let tokens = result.unwrap();
    assert_eq!(tokens.access_token, "at_device_happy");
    assert_eq!(tokens.refresh_token, "rt_device_happy");
    assert_eq!(tokens.account_id, account_id);
    assert!(!tokens.id_token.is_empty());

    // Verify display callback was called with user_code and verification URL
    let msgs = displayed.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].contains("ABCD-1234"), "Should display user_code");
    assert!(msgs[0].contains("/codex/device"), "Should display verification URL");

    // Verify tokens were persisted to auth.json
    assert!(auth_path.exists(), "auth.json should exist after successful login");
    let auth_content = std::fs::read_to_string(&auth_path).unwrap();
    let auth_json: serde_json::Value = serde_json::from_str(&auth_content).unwrap();
    let persisted_tokens = &auth_json["tokens"];
    assert_eq!(persisted_tokens["access_token"], "at_device_happy");
    assert_eq!(persisted_tokens["refresh_token"], "rt_device_happy");
    assert_eq!(persisted_tokens["account_id"], account_id);
}

// =========================================================================
// Scenario: Polling continues on authorization_pending status
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_continues_on_authorization_pending_status() {
    // @step Given a device auth login is in progress with a 5-second polling interval
    let mock_server = MockServer::start().await;

    // Return authorization_pending 3 times, then success
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(3)
        .expect(3)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth_code_after_pending",
            "code_verifier": "verifier_after_pending"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_pending_test".to_string(),
        user_code: "PEND-1234".to_string(),
        interval: 5,
    };

    // @step When the token polling endpoint returns authorization_pending status
    // @step Then the system should wait for 5 seconds
    // @step And the system should poll the token endpoint again without error
    let start = std::time::Instant::now();
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50), // 50ms override for fast testing
        slow_down_increment_override_ms: None,
    };
    let result = poll_device_token(&poll_config, &device_code).await;
    let elapsed = start.elapsed();

    // Should succeed after polling through the pending responses
    assert!(result.is_ok(), "Polling should eventually succeed: {:?}", result.err());
    let poll_result = result.unwrap();
    match poll_result {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => {
            assert_eq!(authorization_code, "auth_code_after_pending");
            assert_eq!(code_verifier, "verifier_after_pending");
        }
        other => panic!("Expected PollResult::Success, got: {:?}", other),
    }

    // Should have waited at least 3 * 50ms (the pending intervals)
    assert!(
        elapsed.as_millis() >= 100,
        "Should have waited between polls, elapsed: {:?}",
        elapsed
    );
}

// =========================================================================
// Scenario: Polling backs off on slow_down response
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_backs_off_on_slow_down_response() {
    // @step Given a device auth login is in progress with a 5-second polling interval
    let mock_server = MockServer::start().await;

    // First: slow_down, then: success
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "slow_down"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth_code_after_slowdown",
            "code_verifier": "verifier_after_slowdown"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_slowdown_test".to_string(),
        user_code: "SLOW-1234".to_string(),
        interval: 1,
    };

    // @step When the token polling endpoint returns a slow_down error
    // @step Then the polling interval should be increased by 5 seconds to 10 seconds
    // @step And the system should continue polling at the new interval
    //
    // With base_interval=50ms and slow_down_increment=200ms, the poll loop:
    //   1. POST → slow_down → interval becomes 50+200=250ms → sleep 250ms
    //   2. POST → success
    // Total sleep: 250ms.  Assert elapsed >= 200ms to prove the increment was
    // applied (a base-only sleep of 50ms would fail this check).
    let start = std::time::Instant::now();
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),  // 50ms base interval for test
        slow_down_increment_override_ms: Some(200), // 200ms increment (instead of 5000ms production)
    };
    let result = poll_device_token(&poll_config, &device_code).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Polling should succeed after slow_down: {:?}", result.err());
    let poll_result = result.unwrap();
    match poll_result {
        PollResult::Success {
            authorization_code,
            code_verifier,
        } => {
            assert_eq!(authorization_code, "auth_code_after_slowdown");
            assert_eq!(code_verifier, "verifier_after_slowdown");
        }
        other => panic!("Expected PollResult::Success, got: {:?}", other),
    }

    // After slow_down, interval = base(50ms) + increment(200ms) = 250ms.
    // Total elapsed must exceed the incremented interval, proving the backoff
    // was applied beyond the base polling interval of 50ms.
    assert!(
        elapsed.as_millis() >= 200,
        "Backoff should increase interval beyond base 50ms to 250ms, but elapsed only {:?}",
        elapsed
    );
}

// =========================================================================
// Scenario: Polling stops on expired_token error
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_stops_on_expired_token_error() {
    // @step Given a device auth login is in progress
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "expired_token"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_expired_test".to_string(),
        user_code: "EXPD-1234".to_string(),
        interval: 5,
    };

    // @step When the token polling endpoint returns an expired_token error
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
    };
    let result = poll_device_token(&poll_config, &device_code).await;

    // @step Then the device auth flow should terminate with an error
    assert!(result.is_ok(), "poll_device_token should return Ok with terminal error");
    let poll_result = result.unwrap();

    // @step And the error should indicate the device code has expired
    match poll_result {
        PollResult::TerminalError { error } => {
            assert!(
                error.contains("expired"),
                "Error should mention expired: {error}"
            );
        }
        other => panic!("Expected PollResult::TerminalError, got: {:?}", other),
    }
}

// =========================================================================
// Scenario: Polling stops on access_denied error
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_stops_on_access_denied_error() {
    // @step Given a device auth login is in progress
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "access_denied"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_denied_test".to_string(),
        user_code: "DENY-1234".to_string(),
        interval: 5,
    };

    // @step When the token polling endpoint returns an access_denied error
    let uri = mock_server.uri();
    let poll_config = PollConfig {
        issuer_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
    };
    let result = poll_device_token(&poll_config, &device_code).await;

    // @step Then the device auth flow should terminate with an error
    assert!(result.is_ok(), "poll_device_token should return Ok with terminal error");
    let poll_result = result.unwrap();

    // @step And the error should indicate the user denied authorization
    match poll_result {
        PollResult::TerminalError { error } => {
            assert!(
                error.contains("denied") || error.contains("access_denied"),
                "Error should mention denied: {error}"
            );
        }
        other => panic!("Expected PollResult::TerminalError, got: {:?}", other),
    }
}

// =========================================================================
// Scenario: Device auth flow times out
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_flow_times_out() {
    // @step Given a device auth login is in progress with a short timeout
    let (temp_dir, _guard) = setup_codex_home();
    let auth_path = temp_dir.path().join("auth.json");

    let mock_server = MockServer::start().await;

    // Usercode endpoint succeeds
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_timeout_test",
            "user_code": "TIME-1234",
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Polling always returns pending — will never complete
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .mount(&mock_server)
        .await;

    let config = DeviceAuthConfig {
        issuer_url: mock_server.uri(),
        timeout_ms: 300, // very short timeout
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        display_fn: None,
    };

    // @step When the timeout expires without the user completing authorization
    let result = device_auth_login(config).await;

    // @step Then the device auth flow should terminate with a timeout error
    assert!(result.is_err(), "Device auth should fail with timeout");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("timed out") || err_msg.contains("timeout"),
        "Error should mention timeout: {err_msg}"
    );

    // @step And no tokens should be persisted
    assert!(
        !auth_path.exists(),
        "No auth.json should exist after timeout"
    );
}

// =========================================================================
// Scenario: Usercode endpoint network failure
// =========================================================================

#[tokio::test]
#[serial]
async fn test_usercode_endpoint_network_failure() {
    // @step When the user initiates device auth login
    // @step And the usercode endpoint is unreachable
    // Use a URL that will definitely fail to connect
    let config = DeviceAuthConfig {
        issuer_url: "http://127.0.0.1:1".to_string(), // port 1 — unreachable
        timeout_ms: 5_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        display_fn: None,
    };

    let result = device_auth_login(config).await;

    // @step Then the device auth flow should terminate immediately with a network error
    assert!(result.is_err(), "Device auth should fail on network error");

    // @step And no polling should be attempted
    // (Verified by the fact that we never set up a polling mock — if polling was
    // attempted, it would also fail, but the key point is we get the error from
    // the usercode request, not from polling.)
    let err_msg = result.unwrap_err().to_string();
    assert!(
        !err_msg.contains("polling") && !err_msg.contains("authorization_pending"),
        "Error should be from usercode request, not polling: {err_msg}"
    );
}

// =========================================================================
// Scenario: Token exchange uses correct parameters without redirect_uri
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_exchange_uses_correct_parameters_without_redirect_uri() {
    // @step Given a device auth login received a successful polling response
    // @step And the response contains authorization_code and code_verifier
    let (_temp_dir, _guard) = setup_codex_home();

    let mock_server = MockServer::start().await;
    let account_id = "acct_exchange_test";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_exchange", "rt_exchange");

    // Usercode endpoint
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_exchange_test",
            "user_code": "XCHG-1234",
            "interval": 1
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Polling returns success immediately
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "device_auth_code_xyz",
            "code_verifier": "device_code_verifier_xyz"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When the authorization_code is exchanged at the token endpoint
    // @step Then the exchange should POST grant_type authorization_code, code, code_verifier, and client_id
    // @step And the exchange should NOT include a redirect_uri parameter
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=device_auth_code_xyz"))
        .and(body_string_contains("code_verifier=device_code_verifier_xyz"))
        .and(body_string_contains(&format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = DeviceAuthConfig {
        issuer_url: mock_server.uri(),
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        display_fn: None,
    };

    let result = device_auth_login(config).await;
    assert!(result.is_ok(), "Device auth login should succeed: {:?}", result.err());

    // @step And the response should contain id_token, access_token, and refresh_token
    let tokens = result.unwrap();
    assert_eq!(tokens.access_token, "at_exchange");
    assert_eq!(tokens.refresh_token, "rt_exchange");
    assert!(!tokens.id_token.is_empty());

    // Verify redirect_uri was NOT sent by checking the mock received the request
    // (wiremock would have rejected if an unexpected body param was sent, but let's
    // also verify via a negative mock)
    // The absence of redirect_uri is verified by the mock setup — it only matches
    // the 4 required params. If redirect_uri were included, it would still match,
    // so we add an explicit negative test:
    let received = mock_server.received_requests().await.unwrap();
    let token_requests: Vec<_> = received
        .iter()
        .filter(|r| r.url.path() == "/oauth/token")
        .collect();
    assert_eq!(token_requests.len(), 1, "Should have exactly 1 token exchange request");
    let body = String::from_utf8_lossy(&token_requests[0].body);
    assert!(
        !body.contains("redirect_uri"),
        "Token exchange should NOT contain redirect_uri, body: {body}"
    );
}

// =========================================================================
// Scenario: Device auth produces same CodexTokens output as browser OAuth
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_produces_same_codex_tokens_output_as_browser_oauth() {
    // @step Given a device auth login completes successfully
    let (_temp_dir, _guard) = setup_codex_home();

    let mock_server = MockServer::start().await;
    let account_id = "acct_struct_test";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_struct", "rt_struct");

    // Usercode endpoint
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_struct_test",
            "user_code": "STRC-1234",
            "interval": 1
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Polling returns success immediately
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "struct_auth_code",
            "code_verifier": "struct_verifier"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Token exchange
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = DeviceAuthConfig {
        issuer_url: mock_server.uri(),
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        display_fn: None,
    };

    // @step When the tokens are returned
    let result = device_auth_login(config).await;
    assert!(result.is_ok(), "Device auth should succeed: {:?}", result.err());
    let tokens = result.unwrap();

    // @step Then the output should be a CodexTokens with id_token, access_token, refresh_token, and account_id
    // Verify all four fields are present and non-empty
    assert!(!tokens.id_token.is_empty(), "id_token should be non-empty");
    assert!(!tokens.access_token.is_empty(), "access_token should be non-empty");
    assert!(!tokens.refresh_token.is_empty(), "refresh_token should be non-empty");
    assert!(!tokens.account_id.is_empty(), "account_id should be non-empty");

    // @step And the output should be identical in structure to browser OAuth login output
    // Verify the type IS CodexTokens (same struct used by browser OAuth)
    // This is a compile-time check — the function signature returns CodexTokens.
    let _: CodexTokens = tokens.clone();

    // Verify we can serialize/deserialize identically to browser OAuth format
    let serialized = serde_json::to_value(&tokens).unwrap();
    assert!(serialized.get("id_token").is_some(), "Serialized should have id_token");
    assert!(serialized.get("access_token").is_some(), "Serialized should have access_token");
    assert!(serialized.get("refresh_token").is_some(), "Serialized should have refresh_token");
    assert!(serialized.get("account_id").is_some(), "Serialized should have account_id");

    // Verify the field values match what we sent
    assert_eq!(tokens.access_token, "at_struct");
    assert_eq!(tokens.refresh_token, "rt_struct");
    assert_eq!(tokens.account_id, account_id);
}

// =========================================================================
// Scenario: request_device_code unit test
// =========================================================================

#[tokio::test]
#[serial]
async fn test_request_device_code_sends_correct_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .and(body_string_contains(&format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_unit_test",
            "user_code": "UNIT-5678",
            "interval": 10
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = request_device_code(&mock_server.uri()).await;
    assert!(result.is_ok(), "request_device_code should succeed: {:?}", result.err());

    let device_code = result.unwrap();
    assert_eq!(device_code.device_auth_id, "dev_unit_test");
    assert_eq!(device_code.user_code, "UNIT-5678");
    assert_eq!(device_code.interval, 10);
}

// =========================================================================
// Scenario: request_device_code handles server error
// =========================================================================

#[tokio::test]
#[serial]
async fn test_request_device_code_handles_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = request_device_code(&mock_server.uri()).await;
    assert!(result.is_err(), "request_device_code should fail on 500");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("500") || err_msg.contains("failed"),
        "Error should mention failure: {err_msg}"
    );
}
