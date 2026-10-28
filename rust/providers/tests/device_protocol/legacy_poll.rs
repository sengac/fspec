use crate::prov159_support::TestResult;
use crate::support::*;
use codelet_providers::codex::codex_device_auth::{
    poll_device_token, DeviceCodeResponse, PollConfig, PollResult,
};
use serial_test::serial;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn sequence(
    error: &'static str,
    pending: usize,
    interval: u64,
    increment: u64,
) -> TestResult {
    let server = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = attempts.clone();
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(move |request: &wiremock::Request| {
            if !is_poll_request(request) {
                return ResponseTemplate::new(415);
            }
            if counter.fetch_add(1, Ordering::SeqCst) < pending {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"error":error}))
            } else {
                ResponseTemplate::new(200).set_body_json(authorized())
            }
        })
        .mount(&server)
        .await;
    let uri = server.uri();
    let started = std::time::Instant::now();
    let result = poll_device_token(
        &PollConfig {
            issuer_url: &uri,
            timeout_ms: 2000,
            poll_interval_override_ms: Some(interval),
            slow_down_increment_override_ms: Some(increment),
        },
        &DeviceCodeResponse {
            device_auth_id: DEVICE.into(),
            user_code: USER_CODE.into(),
            interval: 5,
        },
    )
    .await?;
    let PollResult::Success {
        authorization_code,
        code_verifier,
    } = result
    else {
        return Err("pending poll did not authorize".into());
    };
    assert_eq!(authorization_code, "SYNTHETIC-auth-code");
    assert_eq!(code_verifier, "SYNTHETIC-verifier");
    let minimum = if error == "slow_down" {
        interval + increment
    } else {
        interval * pending as u64
    };
    assert!(started.elapsed().as_millis() >= u128::from(minimum));
    assert_eq!(attempts.load(Ordering::SeqCst), pending + 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_polling_continues_on_authorization_pending_status() -> TestResult {
    // @step Given a device auth login is in progress with a 5-second polling interval
    // @step When the token polling endpoint returns authorization_pending status
    // @step Then the system should wait for 5 seconds
    // @step And the system should poll the token endpoint again without error
    sequence("authorization_pending", 3, 50, 0).await
}

#[tokio::test]
#[serial]
async fn test_polling_backs_off_on_slow_down_response() -> TestResult {
    // @step Given a device auth login is in progress with a 5-second polling interval
    // @step When the token polling endpoint returns a slow_down error
    // @step Then the polling interval should be increased by 5 seconds to 10 seconds
    // @step And the system should continue polling at the new interval
    sequence("slow_down", 1, 50, 200).await
}

async fn terminal(error: &str, expected: &str) -> TestResult {
    let server = MockServer::start().await;
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with({
            let error = error.to_owned();
            move |request: &wiremock::Request| {
                if is_poll_request(request) {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"error":error}))
                } else {
                    ResponseTemplate::new(415)
                }
            }
        })
        .mount(&server)
        .await;
    let uri = server.uri();
    let result = poll_device_token(
        &PollConfig {
            issuer_url: &uri,
            timeout_ms: 2000,
            poll_interval_override_ms: Some(5),
            slow_down_increment_override_ms: None,
        },
        &DeviceCodeResponse {
            device_auth_id: DEVICE.into(),
            user_code: USER_CODE.into(),
            interval: 5,
        },
    )
    .await?;
    let PollResult::TerminalError { error } = result else {
        return Err("terminal poll authorized".into());
    };
    assert!(error.contains(expected));
    assert_eq!(count(&server, "/api/accounts/deviceauth/token").await, 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_polling_stops_on_expired_token_error() -> TestResult {
    // @step Given a device auth login is in progress
    // @step When the token polling endpoint returns an expired_token error
    // @step Then the device auth flow should terminate with an error
    // @step And the error should indicate the device code has expired
    terminal("expired_token", "expired").await
}

#[tokio::test]
#[serial]
async fn test_polling_stops_on_access_denied_error() -> TestResult {
    // @step Given a device auth login is in progress
    // @step When the token polling endpoint returns an access_denied error
    // @step Then the device auth flow should terminate with an error
    // @step And the error should indicate the user denied authorization
    terminal("access_denied", "denied").await
}
