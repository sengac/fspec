use crate::prov159_support::{Fixture, TestResult};
use crate::support::*;
use codelet_providers::codex::codex_device_auth::{device_auth_login, request_device_code};
use serial_test::serial;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn test_device_auth_flow_times_out() -> TestResult {
    // @step Given a device auth login is in progress with a short timeout
    let fixture = Fixture::new()?;
    let server = MockServer::start().await;
    compatible_start(&server).await;
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error":"authorization_pending"})),
        )
        .mount(&server)
        .await;
    // @step When the timeout expires without the user completing authorization
    let error = device_auth_login(config(&server, 100))
        .await
        .err()
        .ok_or("pending login succeeded")?
        .to_string();
    // @step Then the device auth flow should terminate with a timeout error
    assert!(
        error.contains("timed out") || error.contains("timeout"),
        "{error}"
    );
    // @step And no tokens should be persisted
    assert!(!fixture.directory.path().join("auth.json").exists());
    assert_eq!(count(&server, "/oauth/token").await, 0);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_usercode_endpoint_network_failure() -> TestResult {
    // @step When the user initiates device auth login
    let fixture = Fixture::new()?;
    let server = MockServer::start().await;
    let mut configuration = config(&server, 1000);
    // @step And the usercode endpoint is unreachable
    configuration.issuer_url = "http://127.0.0.1:1".into();
    let error = device_auth_login(configuration)
        .await
        .err()
        .ok_or("unreachable login succeeded")?
        .to_string();
    // @step Then the device auth flow should terminate immediately with a network error
    assert!(error.contains("connect"), "{error}");
    // @step And no polling should be attempted
    assert!(!error.contains("polling"));
    assert!(server
        .received_requests()
        .await
        .ok_or("recording disabled")?
        .is_empty());
    assert!(!fixture.directory.path().join("auth.json").exists());
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_request_device_code_sends_correct_request() -> TestResult {
    let server = MockServer::start().await;
    let mut response = usercode();
    response["interval"] = serde_json::json!(10);
    official_start(&server, response).await;
    let code = request_device_code(&server.uri()).await?;
    assert_eq!(code.device_auth_id, DEVICE);
    assert_eq!(code.user_code, USER_CODE);
    assert_eq!(code.interval, 10);
    assert_eq!(count(&server, "/api/accounts/deviceauth/usercode").await, 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_request_device_code_handles_server_error() -> TestResult {
    let server = MockServer::start().await;
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(500).set_body_string(SECRET))
        .mount(&server)
        .await;
    let error = request_device_code(&server.uri())
        .await
        .err()
        .ok_or("server failure succeeded")?
        .to_string();
    assert!(error.contains("500"), "{error}");
    assert!(!error.contains(SECRET));
    Ok(())
}
