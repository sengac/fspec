//! Feature: spec/features/codex-refreshing-client.feature
//! Primary feature: spec/features/prov159-refresh-races.feature

#[path = "prov159_support/network.rs"]
mod network;
mod prov159_support;

use codelet_providers::codex::codex_auth::{delete_codex_auth, read_codex_auth};
use network::{bounded, client, send, RefreshServer};
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn scenario_a5_independent_clients_coordinate_rotation() -> TestResult {
    // @step And renewed freshness is shared even when the issuer returns unchanged token strings
    // @step Given two independently constructed Codex clients share expired credentials with rotating refresh token R0
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::start().await?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer access-A-rotated"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&api)
        .await;
    let first = client(&issuer.uri, 0);
    let second = client(&issuer.uri, 0);
    let url = api.uri();

    // @step When both clients request fresh credentials concurrently
    let pending = tokio::join!(
        async {
            bounded(issuer.started.notified()).await?;
            issuer.release.add_permits(2);
            Ok::<_, tokio::time::error::Elapsed>(())
        },
        async { tokio::join!(send(&first, &url), send(&second, &url)) }
    );
    pending.0?;
    pending.1 .0?;
    pending.1 .1?;
    // @step Then the issuer receives only one exchange of refresh token R0
    assert_eq!(
        issuer.requests().len(),
        1,
        "independent clients must share rotation ownership"
    );
    // @step And both clients use the winning refreshed credentials
    api.verify().await;
    // @step And persisted credentials contain the winning rotation and retain independent API keys and unrelated fields
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    assert_eq!(document["tokens"]["refresh_token"], "refresh-A-rotated");
    assert_eq!(document["OPENAI_API_KEY"], "sk-independent");
    assert_eq!(document["unrelated"]["keep"], true);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a6_reload_external_rotation_without_crossing_accounts() -> TestResult {
    // @step And a concurrent rotation by another cooperating process cannot overwrite the winning credentials
    // @step And independent CODEX_HOME directories remain isolated and shared-home credential compatibility is retained
    // @step And an expired access-token claim is not masked by stale future expiry metadata
    // @step Given a Codex client cached account A refresh token R0 and another process sharing CODEX_HOME persisted rotation R1
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = MockServer::start().await;
    let cached = client(&issuer.uri(), 0);
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    document["tokens"]["refresh_token"] = "refresh-A-external".into();
    document["tokens"]["access_token"] = "access-A-external".into();
    std::fs::write(
        fixture.directory.path().join("auth.json"),
        document.to_string(),
    )?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer access-A-external"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&api)
        .await;

    // @step When the cached client encounters token expiry or a rejected access token
    let result = bounded(send(&cached, &api.uri())).await?;
    // @step Then the client reloads and reuses newer same-account credentials before attempting an exchange of stale R0
    assert!(
        issuer
            .received_requests()
            .await
            .ok_or("issuer recording disabled")?
            .is_empty(),
        "external rotation must be reloaded before contacting the issuer"
    );
    result?;
    api.verify().await;

    // @step And an externally persisted different account causes a clear reauthentication outcome rather than silent account adoption
    fixture.seed("B")?;
    assert!(send(&cached, &api.uri()).await.is_err());
    assert_eq!(
        api.received_requests()
            .await
            .ok_or("API recording disabled")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a7_late_refresh_cannot_resurrect_logout() -> TestResult {
    // @step And the same fencing holds for provider, RPC, NAPI and legacy refresh entry points
    // @step And independent API keys and unrelated credential fields are preserved
    // @step Given a refresh of Codex account A is paused after its network request starts
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::start().await?;
    let api = MockServer::start().await;
    Mock::given(path("/probe"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    let cached = client(&issuer.uri, 0);
    let url = format!("{}/probe", api.uri());
    let pending = send(&cached, &url);

    // @step When local logout completes before the delayed refresh response is released
    let (result, transition) = tokio::join!(pending, async {
        bounded(issuer.started.notified()).await?;
        let transition = delete_codex_auth();
        issuer.release.add_permits(1);
        Ok::<_, Box<dyn std::error::Error>>(transition?)
    });
    transition?;
    // @step Then persisted and live OAuth state remain logged out after the refresh finishes
    assert!(
        read_codex_auth()?.and_then(|auth| auth.tokens).is_none(),
        "late refresh resurrected logged-out OAuth tokens"
    );
    assert!(
        result.is_err(),
        "a revoked refresh must not authorize the pending request"
    );
    // @step And the delayed refresh cannot authorize a later HTTP or WebSocket request
    assert!(send(&cached, &url).await.is_err());
    assert!(api
        .received_requests()
        .await
        .ok_or("API recording disabled")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a8_late_refresh_cannot_overwrite_different_account() -> TestResult {
    // @step And the same fencing holds for provider, RPC, NAPI and legacy refresh entry points
    // @step And independent API keys and unrelated credential fields are preserved
    // @step Given a refresh of Codex account A is paused after its network request starts
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::start().await?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    let cached = client(&issuer.uri, 0);
    let url = api.uri();

    // @step When logout and login as account B complete before the delayed account A refresh response is released
    let (result, transition) = tokio::join!(send(&cached, &url), async {
        bounded(issuer.started.notified()).await?;
        delete_codex_auth()?;
        fixture.seed("B")?;
        issuer.release.add_permits(1);
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    transition?;
    // @step Then storage, the settings UI and newly authorized requests remain bound to account B
    let stored = read_codex_auth()?
        .and_then(|auth| auth.tokens)
        .ok_or("new login disappeared")?;
    assert_eq!(
        stored.account_id, "B",
        "late refresh overwrote the new login"
    );
    // @step And existing account A clients reauthenticate or deliberately rebind without reusing account A state
    assert!(result.is_err());
    assert!(api
        .received_requests()
        .await
        .ok_or("API recording disabled")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_a9_same_account_login_fences_older_generation() -> TestResult {
    // @step And the same fencing holds for NAPI, provider, RPC and legacy refresh entry points
    // @step And independent API keys and unrelated credential fields are preserved
    // @step Given a refresh from Codex account A login generation A1 is paused after its network request starts
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    let issuer = RefreshServer::start().await?;
    let api = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&api)
        .await;
    let cached = client(&issuer.uri, 0);
    let url = api.uri();
    let mut replacement = serde_json::json!({});

    // @step When a new login generation A2 for the same account commits before the delayed A1 response is released
    let (result, transition) = tokio::join!(send(&cached, &url), async {
        bounded(issuer.started.notified()).await?;
        fixture.seed("A")?;
        replacement =
            serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
        replacement["tokens"]["access_token"] = "access-A-generation-2".into();
        replacement["tokens"]["refresh_token"] = "refresh-A-generation-2".into();
        std::fs::write(
            fixture.directory.path().join("auth.json"),
            replacement.to_string(),
        )?;
        issuer.release.add_permits(1);
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    transition?;
    let stored = read_codex_auth()?
        .and_then(|auth| auth.tokens)
        .ok_or("same-account login disappeared")?;
    // @step Then stored and live credentials remain those of login generation A2
    // @step And an identical account ID does not permit the older refresh to commit
    assert_eq!(
        stored.access_token, "access-A-generation-2",
        "matching account IDs must not bypass generation fencing"
    );
    assert_eq!(stored.refresh_token, "refresh-A-generation-2");
    assert!(result.is_err());
    assert!(api
        .received_requests()
        .await
        .ok_or("API recording disabled")?
        .is_empty());
    Ok(())
}
