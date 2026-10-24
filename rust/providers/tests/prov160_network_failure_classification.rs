//! Feature: spec/features/codex-network-failure-classification.feature

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "../src/codex/network_error.rs"]
pub mod network_error;
#[path = "../src/codex/reconnect.rs"]
pub mod reconnect;
#[path = "fixtures/prov160_invalid_websocket.rs"]
mod websocket_fixture;

use codelet_providers::codex::{
    history::CodexHistory, reconnect::CodexRecoveryHandle,
    refreshing_client::RefreshingCodexClient, websocket::CodexWebSocketHandle, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use rig::streaming::RawStreamingChoice;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::Duration;
use websocket_fixture::{Fault, Server};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn websocket_model(
    url: &str,
    history: CodexHistory,
    recovery: CodexRecoveryHandle,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(
            RefreshingCodexClient::new_api_key().with_websocket(CodexWebSocketHandle::default()),
        )
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_recovery_handle(recovery)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

struct NetworkDownResolver;

impl reqwest::dns::Resolve for NetworkDownResolver {
    fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async { Err(std::io::Error::from(std::io::ErrorKind::NetworkDown).into()) })
    }
}

#[tokio::test]
async fn scenario_network_down_setup_errors_enter_cancellable_offline_waiting_on_either_transport(
) -> TestResult {
    // @step Given HTTP or WebSocket setup reports that the network interface is down
    let http_error = reqwest::Client::builder()
        .no_proxy()
        .dns_resolver(Arc::new(NetworkDownResolver))
        .build()?
        .post("http://network-down.invalid/responses")
        .send()
        .await
        .err()
        .ok_or("network-down setup unexpectedly succeeded")?;
    let socket_error = tokio_tungstenite::tungstenite::Error::Io(std::io::Error::from(
        std::io::ErrorKind::NetworkDown,
    ));
    for error in [
        network_error::http_setup(http_error),
        network_error::socket_setup(socket_error),
    ] {
        // @step When Codex classifies the setup error and applies its recovery policy
        let recovery = reconnect::CodexRecoveryHandle::default();
        let handle = recovery.clone();
        let pending = futures::stream::once(async move {
            let mut policy = reconnect::RecoveryPolicy::new(handle);
            policy.retry(CompletionError::HttpError(error)).await?;
            Ok::<_, CompletionError>(RawStreamingChoice::<()>::Message("retried".into()))
        });
        let mut stream = Box::pin(recovery.clone().track(pending));

        // @step Then it waits for network reception until cancellation instead of terminating
        let observed = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
        assert!(
            observed.is_err(),
            "network-down setup terminated: {observed:?}"
        );
        assert_eq!(
            recovery.status(),
            Some(reconnect::RecoveryStatus::WaitingForNetwork { attempt: 1 })
        );
        drop(stream);
        assert_eq!(
            recovery.status(),
            Some(reconnect::RecoveryStatus::Cancelled)
        );
        assert!(!recovery.is_reconnecting());
    }
    Ok(())
}

#[tokio::test]
async fn scenario_abrupt_websocket_connection_loss_retains_same_transport_recovery() -> TestResult {
    // @step Given an accepted Codex WebSocket response whose connection closes without a closing handshake
    let server = Server::start(Fault::DisconnectThenComplete).await?;
    let history = CodexHistory::default();
    let recovery = CodexRecoveryHandle::default();
    let model = websocket_model(&server.url, history.clone(), recovery.clone())?;
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
        .await?;

    // @step When a replacement WebSocket request completes successfully
    let finals = tokio::time::timeout(Duration::from_secs(2), async {
        let mut finals = 0;
        while let Some(event) = stream.next().await {
            if matches!(event?, rig::streaming::StreamedAssistantContent::Final(_)) {
                finals += 1;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(finals)
    })
    .await??;

    // @step Then bounded stream recovery publishes one completion and checkpoint on the selected transport
    assert_eq!(finals, 1);
    assert_eq!(server.requests.load(Ordering::SeqCst), 2);
    assert_eq!(
        recovery.status(),
        Some(codelet_providers::codex::reconnect::RecoveryStatus::Recovered)
    );
    assert!(history.snapshot()?.is_some());
    Ok(())
}

#[tokio::test]
async fn scenario_permanent_accepted_websocket_failures_never_send_replacement_requests(
) -> TestResult {
    for fault in [Fault::Protocol, Fault::Utf8, Fault::Capacity, Fault::Binary] {
        // @step Given an accepted Codex WebSocket response with invalid protocol or UTF-8 or excessive frame size or a binary message
        let server = Server::start(fault).await?;
        let history = CodexHistory::default();
        let recovery = CodexRecoveryHandle::default();
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let captured = notifications.clone();
        recovery.set_observer(Arc::new(move |status| {
            if let Ok(mut statuses) = captured.lock() {
                statuses.push(status);
            }
        }));
        let model = websocket_model(&server.url, history.clone(), recovery.clone())?;
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
            .await?
            .filter(attempt_events::is_content);

        // @step When Codex reads the invalid response
        let observed = tokio::time::timeout(Duration::from_secs(1), stream.next()).await;

        // @step Then the original failure ends the turn without reconnecting or sending a replacement
        assert!(
            observed.is_ok(),
            "permanent {fault:?} failure entered recovery: {:?}, requests: {}",
            recovery.status(),
            server.requests.load(Ordering::SeqCst)
        );
        let event = observed?.ok_or("missing permanent failure")?;
        assert!(event.is_err(), "permanent {fault:?} failure was accepted");
        assert!(stream.next().await.is_none());
        assert_eq!(recovery.status(), None);
        assert!(notifications
            .lock()
            .map_err(|_| "status unavailable")?
            .is_empty());
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(server.requests.load(Ordering::SeqCst), 1);

        // @step And no completion or checkpoint is published
        assert_eq!(history.snapshot()?, None);
    }
    Ok(())
}
