use super::AttemptAuthority;
use crate::codex::history::CodexHistory;
use rig::http_client::HttpClientExt;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

use crate::test_fixtures::{auth as fixture, network};

#[tokio::test]
#[serial_test::serial]
async fn reset_during_credential_refresh_prevents_the_authorized_retry(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a Codex request is rejected and waits for refreshed credentials
    let isolated = fixture::Fixture::new()?;
    isolated.seed("A")?;
    let issuer = network::RefreshServer::start().await?;
    let backend = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let recorded = attempts.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            recorded.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(401)
        })
        .mount(&backend)
        .await;
    let history = CodexHistory::default();
    let request = http::Request::builder()
        .method("POST")
        .uri(format!("{}/generation-recovery", backend.uri()))
        .extension(AttemptAuthority {
            history: history.clone(),
            generation: history.generation()?,
        })
        .body(b"{}".to_vec())?;
    let client = network::client(&issuer.uri, 3600);
    let task = tokio::spawn(async move {
        client
            .send_streaming(request)
            .await
            .err()
            .map(|error| error.to_string())
    });
    tokio::time::timeout(std::time::Duration::from_secs(4), issuer.started.notified()).await?;
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    // @step When its generation is reset before credential refresh is released
    history.reset()?;
    let reset = history.snapshot()?;
    issuer.release.add_permits(1);
    let error = tokio::time::timeout(std::time::Duration::from_secs(4), task).await??;
    // @step Then the stream fails without another physical inference request
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(issuer.requests()?.len(), 1);
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("generation")),
        "{error:?}"
    );
    // @step And the reset checkpoint remains unchanged
    assert_eq!(history.snapshot()?, reset);
    Ok(())
}
