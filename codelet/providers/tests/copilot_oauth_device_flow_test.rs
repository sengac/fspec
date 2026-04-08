#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/github-copilot-oauth-device-flow-token-storage.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-054:
//! GitHub Copilot OAuth device flow & token storage.
//!
//! All tests are FULL INTEGRATION tests — no mocks of our code. They use:
//! - The real `copilot_device_auth_login()` orchestrator
//! - The real `request_device_code()` function
//! - The real `poll_device_token()` function
//! - Real `copilot_auth.json` persistence (with temp dirs via setup_fspec_home)
//! - wiremock for all HTTP endpoints (simulates github.com / GitHub Enterprise)

mod fixtures;

use codelet_providers::copilot::{
    copilot_device_auth_login, get_copilot_auth_path, normalize_enterprise_domain,
    poll_device_token, CopilotDeploymentType, CopilotDeviceAuthConfig, CopilotDeviceCodeResponse,
    CopilotPollConfig, CopilotPollResult,
};
use fixtures::setup_fspec_home;
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Scenario: Login to github.com Copilot deployment via OAuth device flow
// =========================================================================

#[tokio::test]
#[serial]
async fn test_login_to_github_com_copilot_deployment_via_oauth_device_flow() {
    // @step Given I have an active GitHub Copilot subscription on github.com
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step And no existing github-copilot credential exists on disk
    let auth_path = get_copilot_auth_path();
    assert!(
        !auth_path.exists(),
        "copilot_auth.json should not exist initially"
    );

    let mock_server = MockServer::start().await;

    // @step When I run `codelet auth login github-copilot`
    // @step And I select deploymentType "github.com" at the CLI prompt
    // @step And the CLI displays a device code and the URL "https://github.com/login/device"
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc_test_github_com_happy",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And I enter the device code in my browser and approve the request
    // @step Then the polling loop should exchange the device code for an access_token
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_test_token_github_com",
            "token_type": "bearer",
            "scope": "read:user"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let displayed = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let displayed_clone = displayed.clone();

    let config = CopilotDeviceAuthConfig {
        host_url: mock_server.uri(),
        deployment_type: CopilotDeploymentType::GitHubCom,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        authorization_pending_safety_margin_override_ms: Some(0),
        display_fn: Some(Box::new(move |user_code: &str, verification_uri: &str| {
            displayed_clone
                .lock()
                .unwrap()
                .push(format!("{user_code}|{verification_uri}"));
        })),
    };

    let result = copilot_device_auth_login(config).await;
    assert!(
        result.is_ok(),
        "Device auth login should succeed, got: {:?}",
        result.err()
    );
    let auth = result.unwrap();
    assert_eq!(auth.github_oauth_token, "ghu_test_token_github_com");
    assert!(auth.copilot_token.is_none());
    assert!(auth.copilot_token_expires_at.is_none());
    assert_eq!(auth.enterprise_url, None);

    // Verify the user was shown the user_code + verification URL
    let msgs = displayed.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].contains("ABCD-1234"), "Should display user_code");
    assert!(
        msgs[0].contains("https://github.com/login/device"),
        "Should display verification URL"
    );

    // @step And a credential should be persisted at "~/.fspec/credentials/copilot_auth.json"
    assert!(
        auth_path.exists(),
        "copilot_auth.json should exist after successful login"
    );

    // @step And the credential file permissions should be 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&auth_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "copilot_auth.json must be 0600, got {mode:o}");
    }

    // @step And the credential should contain the GitHub OAuth token under github_oauth_token
    let content = std::fs::read_to_string(&auth_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["github_oauth_token"], "ghu_test_token_github_com");

    // @step And the new-schema copilot_token fields are absent until the first exchange
    assert!(
        parsed.get("copilot_token").is_none()
            || parsed["copilot_token"].is_null(),
        "copilot_token must be absent on fresh login"
    );
    assert!(
        parsed.get("copilot_token_expires_at").is_none()
            || parsed["copilot_token_expires_at"].is_null(),
        "copilot_token_expires_at must be absent on fresh login"
    );
}

// =========================================================================
// Scenario: Login to GitHub Enterprise Copilot deployment with normalized enterprise URL
// =========================================================================

#[tokio::test]
#[serial]
async fn test_login_to_github_enterprise_with_normalized_enterprise_url() {
    // @step Given I have an active GitHub Copilot subscription on a GitHub Enterprise instance
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step And no existing github-copilot credential exists on disk
    let auth_path = get_copilot_auth_path();
    assert!(!auth_path.exists());

    let mock_server = MockServer::start().await;

    // @step When I run `codelet auth login github-copilot`
    // @step And I select deploymentType "enterprise" at the CLI prompt
    // @step And I enter "https://ghe.example.com/" at the enterpriseUrl prompt
    let normalized = normalize_enterprise_domain("https://ghe.example.com/");

    // @step Then the enterprise URL should be normalized to "ghe.example.com" (scheme and trailing slash stripped)
    assert_eq!(normalized, "ghe.example.com");

    // @step And the device code flow should POST to "https://ghe.example.com/login/device/code"
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc_test_enterprise",
            "user_code": "ENT-9999",
            "verification_uri": "https://ghe.example.com/login/device",
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step And the polling loop should POST to "https://ghe.example.com/login/oauth/access_token"
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_test_token_enterprise",
            "token_type": "bearer",
            "scope": "read:user"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = CopilotDeviceAuthConfig {
        host_url: mock_server.uri(),
        deployment_type: CopilotDeploymentType::Enterprise {
            host: normalized.clone(),
        },
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        authorization_pending_safety_margin_override_ms: Some(0),
        display_fn: None,
    };

    let result = copilot_device_auth_login(config).await;
    assert!(
        result.is_ok(),
        "Enterprise login should succeed, got: {:?}",
        result.err()
    );

    // @step And a credential should be persisted with the enterpriseUrl field set to "ghe.example.com"
    let auth = result.unwrap();
    assert_eq!(auth.enterprise_url.as_deref(), Some("ghe.example.com"));

    let content = std::fs::read_to_string(&auth_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["enterprise_url"], "ghe.example.com");
    assert_eq!(parsed["github_oauth_token"], "ghu_test_token_enterprise");
}

// =========================================================================
// Scenario: Polling loop handles authorization_pending by sleeping interval
//           plus 3 second safety margin
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_handles_authorization_pending_with_safety_margin() {
    // @step Given I have started a `codelet auth login github-copilot` session
    let mock_server = MockServer::start().await;

    // @step And the device code has been issued with a polling interval of 5 seconds
    let device_code = CopilotDeviceCodeResponse {
        device_code: "dc_pending_test".to_string(),
        user_code: "PEND-1234".to_string(),
        verification_uri: "https://github.com/login/device".to_string(),
        interval: 5,
    };

    // @step When the polling endpoint returns "authorization_pending"
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_after_pending",
            "token_type": "bearer",
            "scope": "read:user"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step Then the polling loop should sleep for 8 seconds (5 second interval + 3 second safety margin)
    // (Test: assert that at least base + safety margin elapsed, using small overrides)
    //
    // With base_interval=50ms and safety_margin=200ms, the loop:
    //   1. POST → authorization_pending → sleep 50+200=250ms
    //   2. POST → success
    // Assert elapsed >= 200ms to prove safety margin was applied beyond base 50ms.
    let start = std::time::Instant::now();
    let uri = mock_server.uri();
    let config = CopilotPollConfig {
        host_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: None,
        authorization_pending_safety_margin_override_ms: Some(200),
    };
    let result = poll_device_token(&config, &device_code).await;
    let elapsed = start.elapsed();

    // @step And the polling loop should retry the access_token request
    // @step And polling should continue until the user approves the device code or the code expires
    assert!(
        result.is_ok(),
        "Polling should eventually succeed: {:?}",
        result.err()
    );
    let poll_result = result.unwrap();
    match poll_result {
        CopilotPollResult::Success { access_token } => {
            assert_eq!(access_token, "ghu_after_pending");
        }
        other => panic!("Expected CopilotPollResult::Success, got: {other:?}"),
    }

    assert!(
        elapsed.as_millis() >= 200,
        "Should sleep at least safety margin (200ms) beyond base interval, elapsed: {elapsed:?}"
    );
}

// =========================================================================
// Scenario: Polling loop handles slow_down by increasing interval per RFC 8628 §3.5
// =========================================================================

#[tokio::test]
#[serial]
async fn test_polling_handles_slow_down_per_rfc_8628() {
    // @step Given I have started a `codelet auth login github-copilot` session
    let mock_server = MockServer::start().await;

    // @step And the device code has been issued with a polling interval of 5 seconds
    let device_code = CopilotDeviceCodeResponse {
        device_code: "dc_slowdown_test".to_string(),
        user_code: "SLOW-1234".to_string(),
        verification_uri: "https://github.com/login/device".to_string(),
        interval: 5,
    };

    // @step When the polling endpoint returns "slow_down" with a server-provided interval of 10 seconds
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "slow_down",
            "interval": 10,
            "error_description": "Polling too frequently"
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_after_slowdown",
            "token_type": "bearer",
            "scope": "read:user"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step Then the polling loop should adopt the server-provided interval of 10 seconds
    // @step And the polling loop should add a 5 second backoff per RFC 8628 §3.5
    // @step And subsequent polls should use the new interval
    //
    // With slow_down_increment=200ms and a small base, the loop:
    //   1. POST → slow_down → server-provided interval added + 200ms increment
    //   2. POST → success
    // Assert elapsed >= 200ms to prove the backoff increment was applied.
    let start = std::time::Instant::now();
    let uri = mock_server.uri();
    let config = CopilotPollConfig {
        host_url: &uri,
        timeout_ms: 30_000,
        poll_interval_override_ms: Some(50),
        slow_down_increment_override_ms: Some(200),
        authorization_pending_safety_margin_override_ms: Some(0),
    };
    let result = poll_device_token(&config, &device_code).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Polling should succeed after slow_down: {:?}",
        result.err()
    );
    let poll_result = result.unwrap();
    match poll_result {
        CopilotPollResult::Success { access_token } => {
            assert_eq!(access_token, "ghu_after_slowdown");
        }
        other => panic!("Expected CopilotPollResult::Success, got: {other:?}"),
    }

    assert!(
        elapsed.as_millis() >= 200,
        "Slow_down backoff should add at least the increment (200ms), elapsed: {elapsed:?}"
    );
}

// =========================================================================
// Scenario: Logout deletes the github-copilot credential file
// =========================================================================

#[tokio::test]
#[serial]
async fn test_logout_deletes_the_github_copilot_credential_file() {
    use codelet_providers::copilot::{write_copilot_auth, CopilotAuthJson};

    // @step Given I am logged in to github-copilot with a credential at "~/.fspec/credentials/copilot_auth.json"
    let (_temp_dir, _guard) = setup_fspec_home();

    let auth =
        CopilotAuthJson::from_github_oauth_token("ghu_logged_in".to_string(), None);
    write_copilot_auth(&auth)
        .await
        .expect("Should be able to write initial auth");

    let auth_path = get_copilot_auth_path();
    assert!(
        auth_path.exists(),
        "Setup: auth file should exist before logout"
    );

    // @step When I run `codelet auth logout github-copilot`
    use codelet_providers::copilot::auth::delete_copilot_auth;
    delete_copilot_auth().await.expect("Logout should succeed");

    // @step Then the file "~/.fspec/credentials/copilot_auth.json" should be deleted
    assert!(
        !auth_path.exists(),
        "auth file should be deleted after logout"
    );

    // @step And opening the codelet TUI should show github-copilot as unauthenticated
    use codelet_providers::copilot::read_copilot_auth_sync;
    let read_result = read_copilot_auth_sync().expect("Read should not error on missing file");
    assert!(
        read_result.is_none(),
        "Reading missing auth file should return None"
    );
}
