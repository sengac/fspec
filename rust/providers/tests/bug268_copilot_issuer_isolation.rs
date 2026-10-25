//! Feature: spec/features/copilot-synthetic-issuer-isolation.feature
#![cfg(feature = "test-support")]

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;

use codelet_providers::copilot::test_support::TestIssuerGuard;
use environment::{Environment, TestResult};
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial_test::serial]
async fn synthetic_issuer_traffic_cannot_escape_through_inherited_proxies() -> TestResult {
    // @step Given a validated local synthetic issuer and a different local proxy capture
    let environment = Environment::new()?;
    let issuer = MockServer::start().await;
    let proxy = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"token":"synthetic-token","expires_at":u64::MAX})),
        )
        .mount(&issuer)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&proxy)
        .await;
    let guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step When proxy environment points at that proxy without a loopback exemption
    std::env::set_var("HTTP_PROXY", proxy.uri());
    std::env::set_var("http_proxy", proxy.uri());
    std::env::set_var("NO_PROXY", "");
    std::env::set_var("no_proxy", "");
    let exchange = guard.probe("synthetic-github-token").await?;
    // @step Then the issuer receives the synthetic exchange directly and the proxy receives nothing
    assert_eq!(exchange.token, "synthetic-token");
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("missing issuer capture")?
            .len(),
        1
    );
    assert!(proxy
        .received_requests()
        .await
        .ok_or("missing proxy capture")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unsafe_issuer_destinations_are_rejected_before_requests() -> TestResult {
    // @step Given an isolated temporary Copilot credential root
    let environment = Environment::new()?;
    // @step When test overrides use DNS nonloopback userinfo or fragment destinations
    for url in [
        "http://localhost:1234/token",
        "https://example.com/token",
        "http://10.1.1.1/token",
        "http://user@127.0.0.1:1234/token",
        "http://127.0.0.1:1234/token#fragment",
    ] {
        // @step Then each override is rejected before any issuer or inference request
        assert!(TestIssuerGuard::install(environment.directory.path(), url).is_err());
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn changed_credential_roots_fail_closed_while_a_fixture_is_active() -> TestResult {
    // @step Given an active synthetic issuer bound to a temporary credential root
    let environment = Environment::new()?;
    let issuer = MockServer::start().await;
    let guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    let elsewhere = codelet_test_helpers::setup_temp_directory()?;
    // @step When the credential location changes directly or through a symlink
    std::env::set_var("FSPEC_HOME", elsewhere.path());
    // @step Then exchange fails before contacting the issuer and cannot fall back externally
    assert!(guard.probe("synthetic-github-token").await.is_err());
    #[cfg(unix)]
    {
        let credentials = environment.directory.path().join(".fspec/credentials");
        std::fs::create_dir_all(credentials.parent().ok_or("missing parent")?)?;
        std::os::unix::fs::symlink(elsewhere.path(), &credentials)?;
        std::env::set_var("FSPEC_HOME", &credentials);
        assert!(guard.probe("synthetic-github-token").await.is_err());
    }
    assert!(issuer
        .received_requests()
        .await
        .ok_or("missing capture")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn synthetic_issuer_responses_cannot_select_an_external_inference_origin() -> TestResult {
    // @step Given a local synthetic issuer returns an external inference origin
    let environment = Environment::new()?;
    let issuer = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token":"synthetic-refreshed-copilot","expires_at":u64::MAX,
            "endpoints":{"api":"https://example.com"}
        })))
        .mount(&issuer)
        .await;
    let guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step When the isolated exchange response is validated
    let outcome = guard.probe("synthetic-github-token").await;
    // @step Then the response is rejected without any inference request
    assert!(outcome.is_err());
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("missing capture")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn redirecting_synthetic_issuers_cannot_escape_loopback_fixtures() -> TestResult {
    // @step Given a validated local synthetic issuer redirects to another endpoint
    let environment = Environment::new()?;
    let destination = MockServer::start().await;
    let issuer = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", destination.uri()))
        .mount(&issuer)
        .await;
    let guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step When the seam probes the issuer with a synthetic token
    let outcome = guard.probe("synthetic-github-token").await;
    // @step Then the redirect is rejected without contacting its destination
    assert!(outcome.is_err());
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("missing issuer capture")?
            .len(),
        1
    );
    assert!(destination
        .received_requests()
        .await
        .ok_or("missing destination capture")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_validated_seam_preserves_isolated_synthetic_exchange_behavior() -> TestResult {
    // @step Given a local synthetic issuer and an isolated temporary credential root
    let environment = Environment::new()?;
    let issuer = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token":"synthetic-refreshed-copilot","expires_at":u64::MAX,
            "endpoints":{"api":issuer.uri()}
        })))
        .mount(&issuer)
        .await;
    let guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step When the seam probes that issuer with a synthetic token
    let exchange = guard.probe("synthetic-github-token").await?;
    // @step Then only the local issuer receives that token and returns the synthetic exchange
    assert_eq!(exchange.token, "synthetic-refreshed-copilot");
    assert_eq!(exchange.endpoints_api, issuer.uri());
    let requests = issuer.received_requests().await.ok_or("missing capture")?;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers["authorization"]
        .to_str()?
        .contains("synthetic-github-token"));
    assert!(codelet_providers::copilot::get_copilot_auth_path()
        .starts_with(environment.directory.path()));
    Ok(())
}
