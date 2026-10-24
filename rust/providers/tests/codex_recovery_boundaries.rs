//! Feature: spec/features/codex-recovery-boundaries.feature

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "prov159_support/mod.rs"]
mod auth;
#[path = "fixtures/prov148.rs"]
mod transport;

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
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use transport::TestResult;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

fn oauth_model(
    url: &str,
    issuer: &str,
    recovery: CodexRecoveryHandle,
    websocket: bool,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let mut client = RefreshingCodexClient::new_oauth(
        "access-A".into(),
        "refresh-A".into(),
        "A".into(),
        Some(0),
        issuer.into(),
    );
    if websocket {
        client = client.with_websocket(CodexWebSocketHandle::default());
    }
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(client)
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_recovery_handle(recovery)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

/// @step Given expired valid Codex credentials and an unavailable credential issuer
/// @step When the selected transport attempts the original request
/// @step Then the session waits for network reception without inference or added user input
/// @step And cancelling the request clears offline recovery promptly
#[tokio::test]
#[serial_test::serial]
async fn expired_oauth_credentials_wait_cancellably_for_an_offline_issuer_on_either_transport(
) -> TestResult {
    let isolated = auth::Fixture::new()?;
    for websocket in [false, true] {
        isolated.seed("A")?;
        let api = transport::Server::start(Vec::new(), false).await?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        drop(listener);
        let issuer = format!("http://{address}");
        let recovery = CodexRecoveryHandle::default();
        let model = oauth_model(&api.url, &issuer, recovery.clone(), websocket)?;
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
            .await?
            .filter(attempt_events::is_content);
        let pending = tokio::time::timeout(Duration::from_millis(150), stream.next()).await;
        assert!(
            pending.is_err(),
            "offline refresh ended the turn: {pending:?}"
        );
        assert_eq!(
            recovery.status(),
            Some(RecoveryStatus::WaitingForNetwork { attempt: 1 })
        );
        assert!(api
            .requests
            .lock()
            .map_err(|_| "requests poisoned")?
            .is_empty());
        drop(stream);
        assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
        assert!(!recovery.is_reconnecting());
    }
    Ok(())
}

/// @step Given expired Codex credentials whose issuer rejects refresh or returns malformed credentials
/// @step When the session attempts its original request
/// @step Then the credential failure ends the turn without recovery or inference requests
#[tokio::test]
#[serial_test::serial]
async fn rejected_or_malformed_refreshed_credentials_never_enter_offline_waiting() -> TestResult {
    let isolated = auth::Fixture::new()?;
    for response in [
        ResponseTemplate::new(401),
        ResponseTemplate::new(200).set_body_string("{}"),
    ] {
        isolated.seed("A")?;
        let issuer = MockServer::start().await;
        Mock::given(path("/oauth/token"))
            .respond_with(response)
            .expect(1)
            .mount(&issuer)
            .await;
        let api = transport::Server::start(Vec::new(), false).await?;
        let recovery = CodexRecoveryHandle::default();
        let model = oauth_model(&api.url, &issuer.uri(), recovery.clone(), false)?;
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
            .await?;
        let error = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(event) = stream.next().await {
                if let Err(error) = event {
                    return Ok::<_, Box<dyn std::error::Error>>(error);
                }
            }
            Err("invalid credentials unexpectedly succeeded".into())
        })
        .await??;
        assert!(error.to_string().contains("credential"), "{error}");
        assert!(!recovery.is_reconnecting());
        assert!(api
            .requests
            .lock()
            .map_err(|_| "requests poisoned")?
            .is_empty());
        issuer.verify().await;
    }
    Ok(())
}

/// @step Given an ordinary or reconnected Codex response commits a tool call
/// @step When the consumer receives that call and stops before the final stream event
/// @step Then transport recovery has already succeeded without a cancellation notification
#[tokio::test]
async fn committed_tools_finish_transport_recovery_before_local_execution() -> TestResult {
    for reconnecting in [false, true] {
        let terminal = transport::event(json!({"type":"response.completed","response":{
            "id":"committed-response", "status":"completed", "end_turn":true,
            "output":[{"type":"function_call", "id":"item-1", "call_id":"call-1",
                "name":"inspect", "arguments":"{}"}],
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
        }}));
        let mut bodies = Vec::new();
        if reconnecting {
            bodies.push(transport::text());
        }
        bodies.push(terminal);
        let server = transport::Server::start(bodies, false).await?;
        let history = CodexHistory::default();
        let recovery = CodexRecoveryHandle::default();
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let observed = statuses.clone();
        recovery.set_observer(Arc::new(move |status| {
            if let Ok(mut statuses) = observed.lock() {
                statuses.push(status);
            }
        }));
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_recovery_handle(recovery.clone());
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "inspect").build())
            .await?;
        tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(event) = stream.next().await {
                if matches!(event?, StreamedAssistantContent::ToolCall(_)) {
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
            }
            Err("committed tool missing".into())
        })
        .await??;
        assert!(history.snapshot()?.is_some());
        let expected = reconnecting.then_some(RecoveryStatus::Recovered);
        assert_eq!(recovery.status(), expected);
        drop(stream);
        assert_eq!(recovery.status(), expected);
        assert!(!statuses
            .lock()
            .map_err(|_| "statuses poisoned")?
            .iter()
            .any(|status| status.contains("cancelled")));
    }
    Ok(())
}
