//! Feature: spec/features/native-codex-device-code-protocol-compatibility.feature
pub mod prov159_support;
#[path = "device_protocol/support.rs"]
pub mod support;

use codelet_providers::codex::auth_lifecycle::cancel_login;
use codelet_providers::codex::codex_device_auth::{device_auth_login, request_device_code};
use prov159_support::{Fixture, TestResult};
use serde_json::json;
use serial_test::serial;
use support::*;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn scenario_official_usercode_response_formats_are_accepted() -> TestResult {
    // @step Given official usercode aliases and string missing or numeric polling intervals
    let parser = MockServer::start().await;
    Mock::given(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id": DEVICE, "user_code": USER_CODE, "interval": " 7 "
        })))
        .mount(&parser)
        .await;
    let parsed = request_device_code(&parser.uri()).await?;
    assert_eq!(parsed.user_code, USER_CODE);
    assert_eq!(parsed.interval, 7);
    for field in ["user_code", "usercode"] {
        for interval in [Some(json!(" 7 ")), Some(json!(7)), None] {
            let server = MockServer::start().await;
            let mut response = json!({"device_auth_id": DEVICE, field: USER_CODE});
            if let Some(interval) = &interval {
                response["interval"] = interval.clone();
            }
            official_start(&server, response).await;
            // @step When the native provider requests a device code using JSON
            let code = request_device_code(&server.uri()).await?;
            // @step Then the code and normalized interval are returned without weakening malformed input checks
            assert_eq!(code.device_auth_id, DEVICE);
            assert_eq!(code.user_code, USER_CODE);
            assert_eq!(code.interval, if interval.is_some() { 7 } else { 0 });
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_pending_http_statuses_do_not_require_json_bodies() -> TestResult {
    // @step Given a native device login whose polling endpoint returns empty pending HTTP403 and HTTP404
    let fixture = Fixture::new()?;
    fixture.seed("SYNTHETIC-existing")?;
    let server = MockServer::start().await;
    compatible_start(&server).await;
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = attempts.clone();
    Mock::given(path("/api/accounts/deviceauth/token"))
        .respond_with(move |request: &wiremock::Request| {
            if !is_poll_request(request) {
                return ResponseTemplate::new(415);
            }
            match counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) {
                0 => ResponseTemplate::new(403),
                1 => ResponseTemplate::new(404),
                _ => ResponseTemplate::new(200).set_body_json(authorized()),
            }
        })
        .mount(&server)
        .await;
    official_exchange(&server).await;
    // @step When authorization later succeeds with a valid polling response
    let result = device_auth_login(config(&server, 2000)).await?;
    // @step Then both original identifiers accompany each JSON poll and callback exchange persists valid tokens
    assert_eq!(result.account_id, "SYNTHETIC-account");
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(count(&server, "/oauth/token").await, 1);
    assert!(
        std::fs::read_to_string(fixture.directory.path().join("auth.json"))?
            .contains("SYNTHETIC-refresh")
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_malformed_and_terminal_responses_cannot_authorize_exchange() -> TestResult {
    // @step Given malformed usercode or poll responses and terminal HTTP or protocol failures
    let fixture = Fixture::new()?;
    fixture.seed("SYNTHETIC-existing")?;
    let auth = fixture.directory.path().join("auth.json");
    let original = std::fs::read(&auth)?;
    for defect in [
        "http-success-body",
        "unknown-error",
        "malformed-poll",
        "empty-poll",
        "bad-interval",
        "bad-usercode",
    ] {
        let server = MockServer::start().await;
        if defect == "bad-interval" || defect == "bad-usercode" {
            let mut response =
                json!({"device_auth_id": DEVICE, "user_code": USER_CODE, "interval": 0});
            if defect == "bad-interval" {
                response["interval"] = json!(-1);
            } else {
                response["user_code"] = json!(false);
            }
            Mock::given(path("/api/accounts/deviceauth/usercode"))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
        } else {
            compatible_start(&server).await;
        }
        let response = match defect {
            "http-success-body" => ResponseTemplate::new(500).set_body_json(authorized()),
            "unknown-error" => ResponseTemplate::new(200).set_body_json(json!({"error": SECRET})),
            "malformed-poll" => ResponseTemplate::new(200).set_body_string(SECRET),
            _ => ResponseTemplate::new(200).set_body_json(json!({})),
        };
        Mock::given(path("/api/accounts/deviceauth/token"))
            .respond_with(response)
            .mount(&server)
            .await;
        Mock::given(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tokens()))
            .mount(&server)
            .await;
        // @step When native device authorization processes those responses
        let result = device_auth_login(config(&server, 1000)).await;
        // @step Then it fails explicitly without exchange persistence or secret disclosure
        let exchanges = count(&server, "/oauth/token").await;
        assert_eq!(exchanges, 0, "{defect} reached token exchange");
        assert_eq!(
            std::fs::read(&auth)?,
            original,
            "{defect} replaced credentials"
        );
        let error = result
            .err()
            .ok_or("terminal response authorized login")?
            .to_string();
        assert!(!error.contains(SECRET));
        assert_eq!(count(&server, "/oauth/token").await, 0, "{defect}");
        assert_eq!(std::fs::read(&auth)?, original, "{defect}");
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_timeout_and_cancellation_preserve_credential_ownership() -> TestResult {
    // @step Given a pending native device login and existing synthetic credentials
    let fixture = Fixture::new()?;
    let auth = fixture.directory.path().join("auth.json");
    for cancel in [false, true] {
        fixture.seed("SYNTHETIC-existing")?;
        let original = std::fs::read(&auth)?;
        let server = MockServer::start().await;
        compatible_start(&server).await;
        Mock::given(path("/api/accounts/deviceauth/token"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        // @step When the operation times out or is cancelled during authorization
        let pending = tokio::spawn(device_auth_login(config(
            &server,
            if cancel { 2000 } else { 60 },
        )));
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while count(&server, "/api/accounts/deviceauth/token").await == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        if cancel {
            cancel_login();
        }
        let error = pending
            .await?
            .err()
            .ok_or("pending login succeeded")?
            .to_string();
        assert!(
            error
                .to_lowercase()
                .contains(if cancel { "cancel" } else { "timed out" }),
            "{error}"
        );
        // @step Then no late completion replaces credentials and a fresh attempt can proceed
        assert_eq!(count(&server, "/oauth/token").await, 0);
        assert_eq!(std::fs::read(&auth)?, original);
        server.reset().await;
        compatible_start(&server).await;
        official_poll(&server).await;
        official_exchange(&server).await;
        assert_eq!(
            device_auth_login(config(&server, 2000)).await?.account_id,
            "SYNTHETIC-account"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_failed_exchange_or_missing_account_identity_cannot_replace_credentials(
) -> TestResult {
    // @step Given a successful device poll and existing synthetic credentials
    let fixture = Fixture::new()?;
    fixture.seed("SYNTHETIC-existing")?;
    let auth = fixture.directory.path().join("auth.json");
    let original = std::fs::read(&auth)?;
    for missing_account in [false, true] {
        let server = MockServer::start().await;
        compatible_start(&server).await;
        Mock::given(path("/api/accounts/deviceauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(authorized()))
            .mount(&server)
            .await;
        let mut response = tokens();
        response["id_token"] = json!("SYNTHETIC-invalid-jwt");
        Mock::given(path("/oauth/token"))
            .respond_with(if missing_account {
                ResponseTemplate::new(200).set_body_json(response)
            } else {
                ResponseTemplate::new(400).set_body_string(SECRET)
            })
            .mount(&server)
            .await;
        // @step When token exchange fails or returns no usable account identity
        let error = device_auth_login(config(&server, 2000))
            .await
            .err()
            .ok_or("invalid exchange succeeded")?
            .to_string();
        // @step Then the native login fails and existing credential bytes remain unchanged
        assert!(!error.contains(SECRET));
        assert_eq!(count(&server, "/oauth/token").await, 1);
        assert_eq!(std::fs::read(&auth)?, original);
    }
    Ok(())
}
