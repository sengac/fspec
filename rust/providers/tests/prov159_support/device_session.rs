//! Feature: spec/features/provider-settings-oauth-login.feature

use crate::test_fixtures::auth as support;

use super::*;
use crate::codex::oauth_test_support::{configure, OAuthTestConfig};
use serial_test::serial;
use support::{Fixture, TestResult};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn scenario_a13_pending_start_expires_and_request_consumes_budget() -> TestResult {
    let _fixture = Fixture::new()?;
    let issuer = MockServer::start().await;
    let _config = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(30))
                .set_body_json(serde_json::json!({
                    "device_auth_id": "device", "user_code": "ABCD", "interval": 1
                })),
        )
        .mount(&issuer)
        .await;
    let before_request = tokio::time::Instant::now();
    start_device_login().await?;
    let home = normalized_home(&auth_storage::home());
    let recorded_start = PENDING
        .lock()
        .map_err(|_| "pending lock poisoned")?
        .get(&home)
        .ok_or("pending session missing")?
        .started;
    let request_consumed_budget =
        recorded_start <= before_request + std::time::Duration::from_millis(5);
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_millis(OAUTH_TIMEOUT_MS + 1)).await;
    tokio::task::yield_now().await;
    let expired = !PENDING
        .lock()
        .map_err(|_| "pending lock poisoned")?
        .contains_key(&home);
    tokio::time::resume();
    cancel_pending(&auth_storage::home());
    assert!(
        expired,
        "abandoned device start retained its pending attempt past the deadline"
    );
    assert!(
        request_consumed_budget,
        "usercode request time was excluded from operation budget"
    );
    Ok(())
}
