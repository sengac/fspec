use codelet_providers::{
    codex::{
        history::CodexHistory, memory_policy, refreshing_client::RefreshingCodexClient,
        CodexResponsesModel,
    },
    request_admission::{AdmissionLimits, RequestAdmissionError},
    selected_request::SelectedRequestPreparer,
};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn limits() -> AdmissionLimits {
    AdmissionLimits {
        context_capacity: Some(10000),
        output_reservation: Some(100),
    }
}

#[tokio::test]
async fn reset_during_transient_rejection_prevents_a_stale_physical_retry() -> TestResult {
    for status in [429, 503] {
        // @step Given a Codex request receives a transient rejection
        let server = MockServer::start().await;
        let attempts = Arc::new(AtomicUsize::new(0));
        let captured = attempts.clone();
        let rejected = Arc::new(tokio::sync::Notify::new());
        let notify = rejected.clone();
        Mock::given(method("POST"))
            .respond_with(move |_: &wiremock::Request| {
                if captured.fetch_add(1, Ordering::SeqCst) == 0 {
                    notify.notify_one();
                    ResponseTemplate::new(status).insert_header("retry-after", "1")
                } else {
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(fixture::terminal("completed", None, None))
                }
            })
            .mount(&server)
            .await;
        let history = CodexHistory::default();
        let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .http_client(RefreshingCodexClient::new_api_key())
            .build()?;
        let model = CodexResponsesModel::new(client, "shared-model")
            .with_history(history.clone())
            .with_request_admission(limits());
        let stream = model
            .stream(
                CompletionRequestBuilder::new(model.clone(), "old generation")
                    .max_tokens(100)
                    .build(),
            )
            .await?;
        let task = tokio::spawn(async move {
            let mut stream = stream;
            let mut errors = Vec::new();
            let mut finals = 0;
            while let Some(event) = stream.next().await {
                match event {
                    Err(error) => errors.push(error.to_string()),
                    Ok(rig::streaming::StreamedAssistantContent::Final(_)) => finals += 1,
                    _ => {}
                }
            }
            (errors, finals)
        });
        // @step When its generation is reset before the retry is released
        tokio::time::timeout(std::time::Duration::from_secs(3), rejected.notified()).await?;
        history.reset()?;
        let reset = history.snapshot()?;
        let (errors, finals) =
            tokio::time::timeout(std::time::Duration::from_secs(4), task).await??;
        // @step Then the stream fails without sending another physical request
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "stale retry after {status}: {errors:?}"
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(finals, 0);
        // @step And the reset checkpoint remains unchanged
        assert_eq!(history.snapshot()?, reset);
    }
    Ok(())
}

#[tokio::test]
async fn server_side_truncation_is_rejected_before_transport() -> TestResult {
    // @step Given a full DAG Codex request configured for automatic truncation
    let server =
        fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(limits());
    let request = CompletionRequestBuilder::new(model.clone(), "complete DAG")
        .max_tokens(100)
        .additional_params(json!({"truncation":"auto"}))
        .build();
    // @step When the request is prepared for streaming or completion
    let stream_error = model
        .stream(request.clone())
        .await
        .err()
        .ok_or("auto truncation stream accepted")?;
    let completion_error = model
        .completion(request)
        .await
        .err()
        .ok_or("auto truncation completion accepted")?;
    // @step Then a typed policy failure occurs without a physical request
    for error in [stream_error, completion_error] {
        assert!(matches!(error, CompletionError::RequestError(ref policy)
            if policy.downcast_ref::<memory_policy::NativeMemoryUnsupported>().is_some()));
    }
    assert!(server
        .requests
        .lock()
        .map_err(|_| "capture lock")?
        .is_empty());
    // @step And the committed checkpoint remains unchanged
    assert_eq!(history.snapshot()?, before);
    Ok(())
}

#[tokio::test]
async fn under_reserved_wire_output_is_rejected_before_transport() -> TestResult {
    // @step Given a Codex request whose wire output allowance exceeds its reservation
    let server =
        fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(limits());
    // @step When the physical stream is consumed
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "complete DAG")
                .max_tokens(5000)
                .build(),
        )
        .await?;
    let result = stream.next().await.ok_or("admission result absent")?;
    // @step Then a typed admission failure occurs without a physical request
    assert!(
        matches!(result, Err(CompletionError::RequestError(ref error))
        if error.downcast_ref::<RequestAdmissionError>().is_some()),
        "{result:?}"
    );
    assert!(stream.next().await.is_none());
    assert!(server
        .requests
        .lock()
        .map_err(|_| "capture lock")?
        .is_empty());
    // @step And the committed checkpoint remains unchanged
    assert_eq!(history.snapshot()?, before);
    Ok(())
}

#[tokio::test]
async fn selected_request_certification_rejects_under_reserved_output() -> TestResult {
    // @step Given a selected provider request whose wire output allowance exceeds its reservation
    let server = fixture::Server::start(vec![], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(limits());
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(5000)
        .build();
    let preparer = SelectedRequestPreparer::new(model, template, limits());
    // @step When its next generation certificate is prepared
    let result = preparer.prepare(&[rig::message::Message::user("complete DAG")], 1);
    // @step Then typed admission rejects the unchanged request before any physical send
    assert!(
        matches!(
            result,
            Err(RequestAdmissionError::OutputReservationMismatch {
                allowance: 5000,
                reservation: 100
            })
        ),
        "under-reserved selected request accepted: {result:?}"
    );
    assert_eq!(history.snapshot()?, before);
    assert!(server
        .requests
        .lock()
        .map_err(|_| "capture lock")?
        .is_empty());
    Ok(())
}
