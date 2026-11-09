//! Feature: spec/features/codex-network-idle-deadline.feature
#[path = "fixtures/prov148.rs"]
pub mod fixture;

use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use fixture::*;
use rig::providers::openai;
use std::time::Duration;

fn model(server: &Server) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(&server.url)
        .http_client(
            RefreshingCodexClient::new_api_key().with_idle_timeout(Duration::from_millis(80)),
        )
        .build()?;
    Ok(
        CodexResponsesModel::new(client, "shared-model").with_request_admission(
            codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            },
        ),
    )
}

#[tokio::test]
async fn network_idle_deadline_ignores_ui_ticks() -> TestResult {
    // @step Given a Codex server that sends partial text then stalls
    // @step When the caller polls alongside frequent UI ticks
    // @step Then abandoned partial text is retracted and repeated network idle deadlines exhaust the bounded stream retries
    let server = Server::start(vec![text(); 6], true).await?;
    let collect = collect(model(&server)?);
    tokio::pin!(collect);
    // TEST-042: the UI ticks are wakeups between polls (yields), not a
    // wall-clock interval; only the deadlock guard ends a collection that
    // the ticks kept alive.
    let observed = codelet_test_helpers::deadlock_guard("the network idle deadlines", async {
        loop {
            tokio::select! {
                result = &mut collect => break result,
                () = codelet_test_helpers::yield_now() => {},
            }
        }
    })
    .await
    .map_err(|_| "UI ticks prevented deadline")??;
    assert_eq!(observed.text, "");
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture unavailable")?
            .len(),
        6
    );
    assert_eq!(observed.errors.len(), 1);
    assert!(observed.errors[0].contains("idle deadline"));
    assert!(observed.finals.is_empty());
    Ok(())
}

#[tokio::test]
async fn tool_execution_and_other_providers_retain_their_policies() -> TestResult {
    // @step Given a completed Codex response followed by a long-running tool
    // @step When time passes beyond the network deadline
    // @step Then the tool is not timed out and other providers retain generic recovery
    let server = Server::start(
        vec![[tool(), terminal("completed", Some(true), None)].concat()],
        true,
    )
    .await?;
    let observed = collect(model(&server)?).await?;
    assert_eq!(observed.tools, 1);
    // TEST-042: the collection has finished; nothing can time it out later.
    assert!(observed.errors.is_empty());
    let server = Server::start(vec![text()], true).await?;
    let native = server.models()?.1;
    // TEST-042: no waiting window; 32 poll rounds and the generic provider
    // is still waiting on its open stream.
    let mut collecting = Box::pin(collect(native));
    let mut rounds = 0;
    let waiting = codelet_test_helpers::pending_until("32 poll rounds", &mut collecting, || {
        rounds += 1;
        rounds > 32
    })
    .await;
    assert!(waiting.is_ok(), "{waiting:?}");
    Ok(())
}
