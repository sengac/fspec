//! Feature: spec/features/codex-refreshing-client.feature
//! Feature: spec/features/prov159-refresh-policy.feature

#[path = "prov159_support/expiry.rs"]
mod expiry;
#[path = "prov159_support/network.rs"]
mod network;
mod prov159_support;

use network::{bounded, client, send, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use wiremock::MockServer;

#[tokio::test]
#[serial]
async fn scenario_a4_stale_construction_cannot_become_an_unbound_client() -> TestResult {
    // @step When a client constructed with stale account A credentials sends a request
    // @step Then the request fails without contacting the API
    // @step Given persisted credentials belong to account B
    let fixture = Fixture::new()?;
    fixture.seed("B")?;
    let api = MockServer::start().await;
    let stale = client(&api.uri(), 3600);
    assert!(send(&stale, &api.uri()).await.is_err());
    assert!(api
        .received_requests()
        .await
        .ok_or("API recording disabled")?
        .is_empty());
    Ok(())
}

#[cfg(feature = "test-support")]
#[tokio::test]
#[serial]
async fn scenario_a14_shared_refresh_reuses_fresh_persisted_credentials() -> TestResult {
    // @step Given persisted account A credentials have an hour of remaining lifetime
    // @step When the shared refresh entry point is called twice
    // @step Then both calls reuse the persisted tokens without contacting the issuer
    use codelet_providers::codex::codex_auth::refresh_codex_tokens;
    use codelet_providers::codex::oauth_test_support::{configure, OAuthTestConfig};
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let path = fixture.directory.path().join("auth.json");
    let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    document["fspec_expires_at"] = (chrono::Utc::now().timestamp() + 3600).into();
    std::fs::write(&path, document.to_string())?;
    let issuer = MockServer::start().await;
    let _configuration = configure(OAuthTestConfig {
        issuer: issuer.uri(),
        callback_address: "127.0.0.1:0".parse()?,
        state: "unused".into(),
    });
    for _ in 0..2 {
        let refreshed = bounded(refresh_codex_tokens("refresh-A")).await??;
        assert_eq!(refreshed.access_token, "access-A");
        assert_eq!(refreshed.refresh_token, "refresh-A");
    }
    assert!(issuer
        .received_requests()
        .await
        .ok_or("issuer recording disabled")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a6_expired_external_jwt_cannot_gain_freshness_from_last_refresh() -> TestResult {
    // @step Given an existing client observes an external rotation with an expired access token
    // @step When the client prepares another request
    // @step Then the rotated credentials are refreshed once with renewed expiry and unrelated fields preserved
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
    std::fs::write(&path, document.to_string())?;
    let api = MockServer::start().await;
    let url = api.uri();
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .mount(&api)
        .await;
    let (sent, released) = tokio::join!(send(&cached, &url), async {
        bounded(issuer.started.notified()).await?;
        issuer.release.add_permits(1);
        Ok::<_, tokio::time::error::Elapsed>(())
    });
    released?;
    sent?;
    assert_eq!(issuer.requests().len(), 1);
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(stored["tokens"]["refresh_token"], "refresh-A-rotated");
    assert!(
        stored["fspec_expires_at"]
            .as_i64()
            .ok_or("missing expiry")?
            > chrono::Utc::now().timestamp() + 3500
    );
    assert_eq!(stored["unrelated"]["keep"], true);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_unrepresentable_token_lifetimes_use_conservative_freshness() -> TestResult {
    // @step Given explicit, persisted and refreshed credentials have an unrepresentable lifetime
    // @step When clients prepare their token expiry
    // @step Then expiry preparation succeeds without panicking
    // @step And the credentials expire within the conservative one-hour fallback
    expiry::unrepresentable_lifetimes().await
}
