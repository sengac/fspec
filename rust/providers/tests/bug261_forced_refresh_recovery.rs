#![doc = "Feature: spec/features/codex-forced-refresh-recovery.feature"]

#[path = "prov159_support/mod.rs"]
pub mod auth;

#[path = "support/bug261_transport.rs"]
mod transport;

use auth::TestResult;
use codelet_providers::codex::{
    codex_oauth::{rewrite_codex_url, CODEX_API_ENDPOINT},
    reconnect::{CodexNetworkFailure, CodexRecoveryHandle, NetworkFailure, RecoveryStatus},
    refreshing_client::RefreshingCodexClient,
};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::http_client::HttpClientExt;
use std::time::Duration;
use transport::{api, model};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[test]
#[serial_test::serial]
fn missing_synthetic_credential_homes_return_controlled_fixture_errors() -> TestResult {
    // @step Given an isolated synthetic credential home that has been removed
    let isolated = auth::Fixture::new()?;
    std::fs::remove_dir_all(isolated.directory.path())?;
    // @step When the reused fixture attempts to resolve its keychain
    let result = isolated.keychain();
    // @step Then it returns a filesystem error instead of panicking
    assert_eq!(
        result.err().ok_or("missing home accepted")?.kind(),
        std::io::ErrorKind::NotFound
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn forced_refresh_network_failures_enter_cancellable_recovery() -> TestResult {
    // @step Given a real inference request rejected with unauthorized credentials
    let isolated = auth::Fixture::new()?;
    for deadline in [false, true] {
        isolated.seed("A")?;
        let api = api().await;
        let issuer = MockServer::start().await;
        Mock::given(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
            .mount(&issuer)
            .await;
        let offline = std::net::TcpListener::bind("127.0.0.1:0")?;
        let offline_url = format!("http://{}", offline.local_addr()?);
        drop(offline);
        let issuer_url = if deadline { issuer.uri() } else { offline_url };
        let recovery = CodexRecoveryHandle::default();
        let model = model(&api.uri(), &issuer_url, recovery.clone())?;
        // @step When forced refresh encounters an offline issuer or setup deadline
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "original task").build())
            .await?;
        let terminal = tokio::time::timeout(Duration::from_millis(600), async {
            while let Some(event) = stream.next().await {
                event?;
            }
            Ok::<_, CompletionError>(())
        })
        .await;
        api.verify().await;
        // @step Then the original request enters typed recovery and cancellation stops it promptly
        assert!(
            terminal.is_err(),
            "refresh ended instead of recovering: {terminal:?}"
        );
        assert_eq!(
            recovery.status(),
            Some(if deadline {
                RecoveryStatus::RetryingStream { attempt: 1 }
            } else {
                RecoveryStatus::WaitingForNetwork { attempt: 1 }
            })
        );
        drop(stream);
        assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn forced_refresh_retains_typed_failure_causes_at_the_http_boundary() -> TestResult {
    // @step Given a loopback inference request whose path cannot be rewritten externally
    let isolated = auth::Fixture::new()?;
    for deadline in [false, true] {
        isolated.seed("A")?;
        let api = MockServer::start().await;
        Mock::given(path("/recovery"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&api)
            .await;
        let url = format!("{}/recovery", api.uri());
        assert_eq!(rewrite_codex_url(&url), url);
        let issuer = MockServer::start().await;
        Mock::given(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
            .mount(&issuer)
            .await;
        let offline = std::net::TcpListener::bind("127.0.0.1:0")?;
        let offline_url = format!("http://{}", offline.local_addr()?);
        drop(offline);
        let issuer_url = if deadline { issuer.uri() } else { offline_url };
        let client = RefreshingCodexClient::new_oauth(
            "access-A".into(),
            "refresh-A".into(),
            "A".into(),
            Some(3600),
            issuer_url,
        )
        .with_idle_timeout(Duration::from_millis(250));
        // @step When unauthorized inference forces refresh against an offline or stalled issuer
        let request = http::Request::builder()
            .method("POST")
            .uri(&url)
            .body(b"{}".to_vec())?;
        let outcome = client.send_streaming(request).await;
        api.verify().await;
        let error = outcome.err().ok_or("refresh unexpectedly succeeded")?;
        // @step Then the original typed connection or setup deadline cause survives
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        let mut failure = None;
        while let Some(error) = source {
            if let Some(error) = error.downcast_ref::<NetworkFailure>() {
                failure = Some(*error);
                break;
            }
            source = error.source();
        }
        assert!(
            matches!(
                (deadline, failure),
                (false, Some(NetworkFailure::Connection))
                    | (true, Some(NetworkFailure::SetupDeadline))
            ),
            "lost typed refresh cause: {error}"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn credential_rejection_remains_terminal_after_unauthorized_inference() -> TestResult {
    // @step Given a real inference request rejected with unauthorized credentials
    let isolated = auth::Fixture::new()?;
    for response in [
        ResponseTemplate::new(401),
        ResponseTemplate::new(200).set_body_string("{}"),
    ] {
        isolated.seed("A")?;
        let api = api().await;
        let issuer = MockServer::start().await;
        Mock::given(path("/oauth/token"))
            .respond_with(response)
            .expect(1)
            .mount(&issuer)
            .await;
        let recovery = CodexRecoveryHandle::default();
        let model = model(&api.uri(), &issuer.uri(), recovery.clone())?;
        // @step When the issuer rejects refresh or returns malformed credentials
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "task").build())
            .await?;
        let error = tokio::time::timeout(Duration::from_secs(1), async {
            while let Some(event) = stream.next().await {
                if let Err(error) = event {
                    return Ok::<_, Box<dyn std::error::Error>>(error);
                }
            }
            Err("rejected refresh succeeded".into())
        })
        .await??;
        // @step Then the actionable credential error terminates without network recovery or secrets
        assert!(error.to_string().contains("credential"), "{error}");
        assert!(!error.to_string().contains("access-A"));
        assert!(!error.to_string().contains("refresh-A"));
        assert!(!recovery.is_reconnecting());
        api.verify().await;
        issuer.verify().await;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn repeated_refresh_deadlines_retain_bounded_retry_behavior() -> TestResult {
    // @step Given unauthorized inference followed by an issuer that repeatedly exceeds setup deadlines
    let isolated = auth::Fixture::new()?;
    isolated.seed("A")?;
    let api = api().await;
    let issuer = MockServer::start().await;
    Mock::given(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&issuer)
        .await;
    let recovery = CodexRecoveryHandle::default();
    let model = model(&api.uri(), &issuer.uri(), recovery.clone())?;
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "original task").build())
        .await?;
    // @step When the original request exhausts its existing stream retry budget
    let error = tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(event) = stream.next().await {
            if let Err(error) = event {
                return Ok::<_, Box<dyn std::error::Error>>(error);
            }
        }
        Err("deadline exhaustion succeeded".into())
    })
    .await??;
    // @step Then the typed network failure reports five retries without replaying inference
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    let mut failure = None;
    while let Some(error) = source {
        if let Some(error) = error.downcast_ref::<CodexNetworkFailure>() {
            failure = Some(error.source);
            break;
        }
        source = error.source();
    }
    assert!(
        matches!(failure, Some(NetworkFailure::SetupDeadline)),
        "{error}"
    );
    assert_eq!(recovery.status(), Some(RecoveryStatus::Failed));
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("issuer requests missing")?
            .len(),
        6
    );
    api.verify().await;
    issuer.verify().await;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn approved_test_transport_is_restricted_to_numeric_loopback_http() -> TestResult {
    // @step Given the approved test only endpoint injection seam
    let isolated = auth::Fixture::new()?;
    isolated.seed("A")?;
    let client = RefreshingCodexClient::new_oauth(
        "access-A".into(),
        "refresh-A".into(),
        "A".into(),
        Some(3600),
        "http://127.0.0.1:1".into(),
    );
    // @step When local and nonlocal endpoint candidates are configured
    for endpoint in [
        "https://example.invalid/responses",
        "http://example.invalid/responses",
        "http://192.0.2.1/responses",
        "http://localhost/responses",
        "https://127.0.0.1/responses",
    ] {
        assert!(
            client.clone().with_test_endpoint(endpoint).is_err(),
            "{endpoint}"
        );
    }
    // @step Then only numeric loopback HTTP is accepted and the shipping rewrite policy stays unchanged
    for endpoint in [
        "http://127.0.0.1:1234/responses",
        "http://[::1]:1234/responses",
    ] {
        let local = client.clone().with_test_endpoint(endpoint)?;
        assert_eq!(local.test_request_url(CODEX_API_ENDPOINT), endpoint);
    }
    assert_eq!(
        rewrite_codex_url("http://127.0.0.1:1234/responses"),
        CODEX_API_ENDPOINT
    );
    Ok(())
}
