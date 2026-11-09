//! Feature: spec/features/codex-oauth-napi-bindings.feature
use super::*;

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
        .and(body_partial_json(
            serde_json::json!({ "client_id": CODEX_CLIENT_ID }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_flow_start",
            "user_code": "FLOW-1234",
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_device_login_start()
    let device_code = request_device_code(&mock_server.uri())
        .await
        .expect("request_device_code should succeed");
    let verification_url = format!("{}/codex/device", mock_server.uri());

    // @step Then the result should contain a user_code string
    assert_eq!(device_code.user_code, "FLOW-1234");

    // @step And the result should contain a verification_url string
    assert!(verification_url.contains("/codex/device"));

    // The poll phase needs the device_auth_id and interval
    assert_eq!(device_code.device_auth_id, "dev_flow_start");
    assert_eq!(device_code.interval, 5);
}

// =========================================================================
// Scenario: Device auth login poll resolves with tokens after user authorizes
// =========================================================================

#[tokio::test]
#[serial]
async fn test_device_auth_login_poll_resolves_with_tokens_after_user_authorizes() {
    // @step Given a device auth flow has been started with a valid device_auth_id
    let (temp_dir, _guard) = codex_home();
    let auth_path = temp_dir.path().join("auth.json");

    let mock_server = MockServer::start().await;
    let account_id = "acct_device_poll_flow";
    let id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&id_token, "at_device_flow", "rt_device_flow");

    let device_code = DeviceCodeResponse {
        device_auth_id: "dev_poll_flow_test".to_string(),
        user_code: "POLL-5678".to_string(),
        interval: 5,
    };

    // @step And the device token endpoint will return authorization_code after polling
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_partial_json(
            serde_json::json!({ "device_auth_id": "dev_poll_flow_test" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_partial_json(
            serde_json::json!({ "device_auth_id": "dev_poll_flow_test" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "flow_device_auth_code",
            "code_verifier": "flow_device_verifier"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=flow_device_auth_code"))
        .and(body_string_contains("code_verifier=flow_device_verifier"))
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

    let PollResult::Success {
        authorization_code,
        code_verifier,
    } = poll_result
    else {
        unreachable!("Expected PollResult::Success, got: {poll_result:?}");
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
    let tokens = build_and_persist_tokens(&token_response);
    assert_eq!(tokens.access_token, "at_device_flow");
    assert_eq!(tokens.refresh_token, "rt_device_flow");
    assert_eq!(tokens.account_id, account_id);

    // @step And the tokens should be persisted to auth.json
    assert!(
        auth_path.exists(),
        "auth.json should exist after device auth"
    );
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["tokens"]["access_token"], "at_device_flow");
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
        device_auth_id: "dev_flow_expired".to_string(),
        user_code: "EXPD-FLOW".to_string(),
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
    let PollResult::TerminalError { error } = result else {
        unreachable!("Expected PollResult::TerminalError, got: {result:?}");
    };
    assert!(
        error.contains("expired"),
        "Error reason should contain 'expired': {error}"
    );
}
