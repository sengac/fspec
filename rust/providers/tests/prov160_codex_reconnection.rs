//! Feature: spec/features/codex-network-reconnection.feature
#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "prov159_support/mod.rs"]
mod auth;
#[path = "fixtures/prov148.rs"]
mod fixture;

use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use fixture::{collect, event, terminal, text, tool, Server, TestResult};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    providers::openai,
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn model(
    url: &str,
    history: CodexHistory,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(
            RefreshingCodexClient::new_api_key().with_idle_timeout(Duration::from_millis(80)),
        )
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_accepted_sse_interruptions_discard_uncommitted_tool_calls() -> TestResult {
    // @step Given a Codex response containing partial text and uncommitted tool calls
    let server = Server::start(
        vec![
            [text(), tool()].concat(),
            terminal("completed", Some(true), None),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let writes = Arc::new(AtomicUsize::new(0));
    let recorded = writes.clone();
    history.set_writer(Arc::new(move |_| {
        recorded.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }))?;
    // @step When the accepted SSE stream disconnects and its replacement completes
    let observed = tokio::time::timeout(
        Duration::from_secs(4),
        collect(model(&server.url, history.clone())?),
    )
    .await??;
    // @step Then only the successful attempt publishes a checkpoint and dispatches its tools
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.tools, 0);
    assert_eq!(observed.finals.len(), 1);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    let requests = server.requests.lock().map_err(|_| "capture unavailable")?;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    assert!(!requests[1].to_string().contains("Continue"));
    assert!(history.snapshot()?.is_some());
    Ok(())
}

#[tokio::test]
async fn scenario_five_stream_retries_exhaust_without_checkpoint_publication() -> TestResult {
    // @step Given six Codex accepted responses that disconnect before completion
    let server = Server::start(vec![text(); 6], false).await?;
    let history = CodexHistory::default();
    // @step When Codex consumes its five stream retries
    let observed = tokio::time::timeout(
        Duration::from_secs(15),
        collect(model(&server.url, history.clone())?),
    )
    .await??;
    // @step Then a typed exhausted failure ends the turn without completion or checkpoint mutation
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture unavailable")?
            .len(),
        6
    );
    assert_eq!(observed.errors.len(), 1);
    assert!(
        observed.errors[0].contains("five stream retries"),
        "{observed:?}"
    );
    assert!(observed.finals.is_empty());
    assert_eq!(observed.text, "");
    assert_eq!(observed.attempts, ["started", "aborted"].repeat(6));
    assert_eq!(history.snapshot()?, None);
    Ok(())
}

#[tokio::test]
async fn scenario_idle_deadlines_consume_the_stream_budget_rather_than_waiting_offline(
) -> TestResult {
    // @step Given a Codex server that repeatedly sends partial text and stalls
    let server = Server::start(vec![text(); 6], true).await?;
    // @step When raw network idle deadlines expire despite UI polling
    let observed = tokio::time::timeout(
        Duration::from_secs(15),
        collect(model(&server.url, CodexHistory::default())?),
    )
    .await??;
    // @step Then five bounded replacements occur and abandoned partial text is retracted on exhaustion
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture unavailable")?
            .len(),
        6
    );
    assert_eq!(observed.text, "");
    assert_eq!(observed.errors.len(), 1);
    assert!(observed.errors[0].contains("idle deadline"), "{observed:?}");
    assert!(observed.finals.is_empty());
    Ok(())
}

#[tokio::test]
async fn scenario_permanent_failures_never_enter_network_recovery() -> TestResult {
    // @step Given Codex requests rejected by authentication or permanent errors or malformed events
    for status in [400, 401, 403, 404, 500] {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
        // @step When the provider reports those failures
        let observed = tokio::time::timeout(
            Duration::from_secs(2),
            collect(model(&server.uri(), CodexHistory::default())?),
        )
        .await??;
        // @step Then no replacement or offline waiting occurs
        assert_eq!(observed.errors.len(), 1);
        assert!(observed.finals.is_empty());
        server.verify().await;
    }
    let server = Server::start(
        vec![event(json!({"type":"response.output_text.delta"}))],
        false,
    )
    .await?;
    let observed = collect(model(&server.url, CodexHistory::default())?).await?;
    assert_eq!(observed.errors.len(), 1);
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture unavailable")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_recovery_revalidates_generation_and_credential_authority() -> TestResult {
    // @step Given a Codex request waiting to reconnect
    let server =
        Server::start(vec![text(), terminal("completed", Some(true), None)], false).await?;
    let history = CodexHistory::default();
    let model = model(&server.url, history.clone())?;
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?
        .filter(attempt_events::is_content);
    stream.next().await.ok_or("missing partial text")??;
    // @step When its committed generation or credential authority changes during the wait
    let progress = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(
        progress.is_err(),
        "interruption should wait for recovery, not terminate"
    );
    history.reset()?;
    let previous = history.snapshot()?;
    // @step Then the stale attempt sends nothing and returns a terminal authority failure
    let error = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await?
        .ok_or("missing failure")?
        .err()
        .ok_or("stale generation succeeded")?;
    assert!(error.to_string().contains("generation"), "{error}");
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture unavailable")?
            .len(),
        1
    );
    assert_eq!(history.snapshot()?, previous);
    let isolated = auth::Fixture::new()?;
    isolated.seed("A")?;
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .expect(1)
        .mount(&server)
        .await;
    let client = RefreshingCodexClient::new_oauth(
        "access-A".into(),
        "refresh-A".into(),
        "A".into(),
        Some(3600),
        server.uri(),
    );
    let request = http::Request::builder()
        .method("POST")
        .uri(format!("{}/authority-check", server.uri()))
        .body(Vec::<u8>::new())?;
    use rig::http_client::HttpClientExt;
    let pending = client.send_streaming(request);
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    isolated.seed("B")?;
    let error = tokio::time::timeout(Duration::from_secs(2), &mut pending)
        .await?
        .err()
        .ok_or("changed account succeeded")?;
    assert!(
        error.to_string().contains("authentication changed"),
        "{error}"
    );
    server.verify().await;
    Ok(())
}
