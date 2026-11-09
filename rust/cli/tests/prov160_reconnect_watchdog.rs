#![doc = "Feature: spec/features/codex-reconnect-watchdog.feature"]

use codelet_cli::interactive::provider_recovery::{
    should_retry_network_error, wait_for_stall_deadline,
};
use codelet_cli::interactive::MAX_NETWORK_RETRIES;
use codelet_providers::codex::{
    reconnect::{CodexNetworkFailure, RecoveryStatus},
    refreshing_client::RefreshingCodexClient,
    session::CodexSession,
    CodexResponsesModel,
};
use codelet_providers::session::ProviderSession;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use std::time::Duration;

#[path = "../../providers/tests/fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn model(
    url: &str,
    session: &ProviderSession,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let recovery = session
        .extension::<CodexSession>("codex")
        .ok_or("Codex session missing")?
        .recovery
        .clone();
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_recovery_handle(recovery)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

#[tokio::test]
async fn provider_offline_waiting_is_not_mistaken_for_a_stalled_generation() -> TestResult {
    // @step Given a Codex session intentionally waiting for network reception
    let sessions = ProviderSession::for_provider("codex");
    let ordinary = ProviderSession::for_provider("openai");
    let recovery = &sessions
        .extension::<CodexSession>("codex")
        .ok_or("Codex session missing")?
        .recovery;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let offline_url = format!("http://{}", listener.local_addr()?);
    drop(listener);
    let offline = model(&offline_url, &sessions)?;
    let mut stream = offline
        .stream(CompletionRequestBuilder::new(offline.clone(), "original turn").build())
        .await?
        .filter(attempt_events::is_content);
    // @step When offline waiting outlasts the generic generation stall deadline
    {
        let pending = stream.next();
        tokio::pin!(pending);
        // BUG-313: the first attempt can take a few hundred milliseconds to
        // start in debug builds; first reach the offline-waiting state...
        // (TEST-042: polled between yields, no waiting window.)
        codelet_test_helpers::pending_until(
            "the offline request waits for the network",
            &mut pending,
            || {
                matches!(
                    recovery.status(),
                    Some(RecoveryStatus::WaitingForNetwork { .. })
                )
            },
        )
        .await
        .map_err(|failure| format!("offline request unexpectedly ended: {failure}"))?;
        // ...then let that wait outlast the generic stall deadline: until
        // recovery reaches its second offline attempt (a 5 s network backoff,
        // far past the 20 ms stall deadline), neither the request nor the
        // generic watchdog may end. TEST-042: the test waits on that event.
        let watchdog = wait_for_stall_deadline(&sessions, "codex", Duration::from_millis(20));
        tokio::pin!(watchdog);
        codelet_test_helpers::deadlock_guard("the second offline attempt", async {
            tokio::select! {
                event = &mut pending => Err(format!("offline request unexpectedly ended: {event:?}")),
                _ = &mut watchdog => Err("provider offline wait fired generic stall watchdog".to_string()),
                () = async {
                    while !matches!(
                        recovery.status(),
                        Some(RecoveryStatus::WaitingForNetwork { attempt }) if attempt >= 2
                    ) {
                        codelet_test_helpers::yield_now().await;
                    }
                } => Ok(()),
            }
        })
        .await??;
    }
    // @step Then the caller keeps polling cancellably until recovery while ordinary provider stall detection remains enabled
    assert!(matches!(
        recovery.status(),
        Some(RecoveryStatus::WaitingForNetwork { .. })
    ));
    codelet_test_helpers::deadlock_guard(
        "wait_for_stall_deadline(&ordinary, 'openai', Duration::fr...",
        wait_for_stall_deadline(&ordinary, "openai", Duration::from_millis(20)),
    )
    .await?;
    codelet_test_helpers::deadlock_guard(
        "wait_for_stall_deadline(&sessions, 'openai', Duration::fr...",
        wait_for_stall_deadline(&sessions, "openai", Duration::from_millis(20)),
    )
    .await?;
    drop(stream);
    assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
    codelet_test_helpers::deadlock_guard(
        "wait_for_stall_deadline(&sessions, 'codex', Duration::fro...",
        wait_for_stall_deadline(&sessions, "codex", Duration::from_millis(20)),
    )
    .await?;

    let server = transport::Server::start(vec![transport::text(); 6], false).await?;
    let provider = model(&server.url, &sessions)?;
    let mut stream = provider
        .stream(CompletionRequestBuilder::new(provider.clone(), "original turn").build())
        .await?;
    let failure = codelet_test_helpers::deadlock_guard("the awaited block", async {
        while let Some(event) = stream.next().await {
            if let Err(error) = event {
                return Ok::<_, Box<dyn std::error::Error>>(error);
            }
        }
        Err("exhausted response unexpectedly completed".into())
    })
    .await??;
    let wrapped = anyhow::Error::from(failure).context("connection reset by peer");
    assert!(wrapped
        .chain()
        .any(|source| source.downcast_ref::<CodexNetworkFailure>().is_some()));
    assert!(!should_retry_network_error("codex", wrapped.as_ref()));
    assert!(!should_retry_network_error("openai", wrapped.as_ref()));
    assert!(!should_retry_network_error(
        "codex",
        anyhow::anyhow!("connection reset by peer").as_ref()
    ));
    assert!(should_retry_network_error(
        "openai",
        anyhow::anyhow!("connection reset by peer").as_ref()
    ));
    assert_eq!(MAX_NETWORK_RETRIES, 3);
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|error| error.to_string())?
            .len(),
        6
    );
    Ok(())
}
