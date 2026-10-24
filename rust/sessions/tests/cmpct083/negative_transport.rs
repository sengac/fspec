use super::{
    fixture::{Fixture, TestResult},
    runtime::{self, Output},
};
use codelet_providers::codex::{
    history::CheckpointError,
    refreshing_client::RefreshingCodexClient,
    session::CodexSession,
    websocket::{CodexWebSocketHandle, WebSocketUpgradeRejected},
    CodexResponsesModel,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn run(fixture: &Fixture, row: &str, output: &Output) -> TestResult {
    let durable_before_injection = fixture.bytes()?;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(426))
        .mount(&server)
        .await;
    let mut inner = fixture.background.inner.lock().await;
    let history = inner
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex session extension")?
        .history
        .clone();
    if row == "incompatible checkpoint lineage" {
        let mut incompatible = history.snapshot()?.ok_or("committed Codex checkpoint")?;
        incompatible.model = Some("different-model".into());
        history.restore(Some(incompatible))?;
    }
    assert_eq!(
        durable_before_injection,
        fixture.bytes()?,
        "negative fixture changed the committed manifest before the selected request"
    );
    let checkpoint = history.snapshot()?;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("offline-gate-key")
        .http_client(if row.contains("websocket") {
            RefreshingCodexClient::new_api_key().with_websocket(CodexWebSocketHandle::default())
        } else {
            RefreshingCodexClient::new_api_key()
        })
        .base_url(server.uri())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model")
        .with_history(history.clone())
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        });
    let failure = runtime::stream(model, &mut inner, fixture.background.id, output)
        .await
        .err()
        .ok_or("selected transport or checkpoint must fail")?;
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(failure.as_ref());
    let mut typed = false;
    while let Some(error) = source {
        typed |= if row.contains("websocket") {
            error.downcast_ref::<WebSocketUpgradeRejected>().is_some()
        } else {
            matches!(
                error.downcast_ref::<CheckpointError>(),
                Some(CheckpointError::Lineage(_))
            )
        };
        source = error.source();
    }
    assert!(typed, "{row} lost its typed runtime failure: {failure:?}");
    assert_eq!(checkpoint, history.snapshot()?, "{row} changed checkpoint");
    let requests = server.received_requests().await.ok_or("request capture")?;
    assert!(
        !requests.iter().any(|request| request.method == "POST"),
        "transport/lineage fallback request"
    );
    assert_eq!(requests.len(), usize::from(row.contains("websocket")));
    assert!(
        inner.ensure_compaction_inference_ready().is_err(),
        "runtime transport/lineage failure left ordinary inference enabled"
    );
    Ok(())
}
