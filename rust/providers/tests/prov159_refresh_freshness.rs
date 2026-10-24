//! Feature: spec/features/codex-refreshing-client.feature
//! Feature: spec/features/prov159-refresh-freshness.feature

#[path = "prov159_support/network.rs"]
mod network;
mod prov159_support;

use network::{bounded, client, send, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn scenario_a6_expired_jwt_overrides_stale_future_expiry_metadata() -> TestResult {
    // @step Given an external rotation has an expired JWT and future stored expiry metadata
    // @step When an existing client sends another request
    // @step Then the issuer is contacted once and the refreshed credentials are persisted
    use base64::Engine;
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::start().await?;
    let cached = client(&issuer.uri, 0);
    let path = fixture.directory.path().join("auth.json");
    let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let claims = serde_json::json!({"exp": chrono::Utc::now().timestamp() - 60});
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
    document["tokens"]["access_token"] = format!("e30.{payload}.signature").into();
    document["tokens"]["refresh_token"] = "refresh-A-external".into();
    document["fspec_expires_at"] = (chrono::Utc::now().timestamp() + 7200).into();
    std::fs::write(&path, document.to_string())?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    issuer.release.add_permits(1);
    bounded(send(&cached, &api.uri())).await??;
    assert_eq!(issuer.requests().len(), 1);
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(stored["tokens"]["refresh_token"], "refresh-A-rotated");
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a5_unchanged_tokens_still_publish_renewed_freshness() -> TestResult {
    // @step Given two clients share expired credentials and the issuer returns identical token strings with renewed lifetime
    // @step When both clients send requests in sequence
    // @step Then both API requests succeed with only one issuer refresh
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::with_body(
        serde_json::json!({
            "id_token": "id-A", "access_token": "access-A",
            "refresh_token": "refresh-A", "expires_in": 3600
        })
        .to_string(),
    )
    .await?;
    let first = client(&issuer.uri, 0);
    let second = client(&issuer.uri, 0);
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    issuer.release.add_permits(2);
    bounded(send(&first, &api.uri())).await??;
    bounded(send(&second, &api.uri())).await??;
    assert_eq!(issuer.requests().len(), 1);
    assert_eq!(
        api.received_requests()
            .await
            .ok_or("API recording disabled")?
            .len(),
        2
    );
    Ok(())
}
