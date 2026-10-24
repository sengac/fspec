//! Feature: spec/features/codex-dns-reconnection.feature
#[path = "fixtures/attempt_events.rs"]
mod attempt_events;

use codelet_providers::codex::{
    history::CodexHistory,
    reconnect::{CodexRecoveryHandle, RecoveryStatus},
    refreshing_client::RefreshingCodexClient,
    websocket::CodexWebSocketHandle,
    CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use std::time::Duration;

#[tokio::test]
async fn scenario_dns_failures_enter_cancellable_offline_waiting_on_either_transport(
) -> Result<(), Box<dyn std::error::Error>> {
    for websocket in [false, true] {
        // @step Given a Codex session whose selected transport cannot resolve its hostname
        let hostname = format!("{}.invalid", "unresolvable".repeat(30));
        let mut client = RefreshingCodexClient::new_api_key();
        if websocket {
            client = client.with_websocket(CodexWebSocketHandle::default());
        }
        let client = openai::Client::<RefreshingCodexClient>::builder()
            .api_key("test-key")
            .base_url(format!("http://{hostname}"))
            .http_client(client)
            .build()?;
        let history = CodexHistory::default();
        let recovery = CodexRecoveryHandle::default();
        let model = CodexResponsesModel::new(client, "shared-model")
            .with_history(history.clone())
            .with_recovery_handle(recovery.clone())
            .with_request_admission(AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            });

        // @step When the session attempts its original request
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
            .await?
            .filter(attempt_events::is_content);
        let result = tokio::time::timeout(Duration::from_secs(1), stream.next()).await;

        // @step Then it waits for network reception until cancellation without publishing a checkpoint
        assert!(
            result.is_err(),
            "DNS outage terminated the turn: {result:?}"
        );
        assert!(matches!(
            recovery.status(),
            Some(RecoveryStatus::WaitingForNetwork { attempt: 1 })
        ));
        drop(stream);
        assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
        assert_eq!(history.snapshot()?, None);
    }
    Ok(())
}
