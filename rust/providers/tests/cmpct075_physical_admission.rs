//! Feature: spec/features/codex-physical-request-admission.feature

use codelet_providers::{codex::history::CodexHistory, request_admission::AdmissionLimits};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn terminal(status: &str, text: &str, end_turn: bool) -> String {
    let mut response = fixture::baseline::response();
    response["status"] = json!(status);
    response["end_turn"] = json!(end_turn);
    response["output"][0]["content"][0]["text"] = json!(text);
    if status == "incomplete" {
        response["incomplete_details"] = json!({"reason":"interrupted"});
    }
    fixture::event(json!({"type":format!("response.{status}"),"response":response}))
}

async fn blocked_growth(status: &str) -> TestResult {
    let history = CodexHistory::default();
    history.transition_generation(7)?;
    history.commit(&json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"hello"}]}]}), &[])?;
    let previous = history.snapshot()?;
    let text = "retained measured output ".repeat(2000);
    let server = fixture::Server::start(
        vec![
            terminal(status, &text, false),
            terminal("completed", "must not send", true),
        ],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(2000),
            output_reservation: Some(100),
        });
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    let mut output = String::new();
    let mut usage = Vec::new();
    let mut final_count = 0;
    let mut blocked = false;
    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamedAssistantContent::Text(text)) => output.push_str(&text.text),
            Ok(StreamedAssistantContent::Usage(measured)) => usage.push(measured.total_tokens),
            Ok(StreamedAssistantContent::Final(_)) => final_count += 1,
            Err(rig::completion::CompletionError::RequestError(error)) => {
                blocked = error
                    .downcast_ref::<codelet_providers::request_admission::RequestAdmissionError>()
                    .is_some();
            }
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    assert!(
        blocked,
        "continuation must return a typed admission failure"
    );
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?
            .len(),
        1,
        "oversized physical continuation reached network"
    );
    assert_eq!(output, text);
    assert_eq!(usage, vec![20]);
    assert_eq!(final_count, 0);
    assert_eq!(history.snapshot()?, previous);
    Ok(())
}

#[tokio::test]
async fn completed_continuation_growth_is_blocked_before_the_oversized_send() -> TestResult {
    // @step Given an admitted initial request with explicit limits
    // @step And a completed non-final response appends enough assistant output to exceed capacity
    // @step When Codex attempts its internal continuation
    // @step Then no oversized request reaches the transport
    // @step And measured usage and output remain observable without a final completion
    // @step And the committed provider checkpoint remains unchanged
    blocked_growth("completed").await
}

#[tokio::test]
async fn interrupted_continuation_growth_is_blocked_before_the_oversized_send() -> TestResult {
    // @step Given an admitted initial request with explicit limits
    // @step And an interrupted incomplete response appends enough assistant output to exceed capacity
    // @step When Codex attempts its internal continuation
    // @step Then no oversized request reaches the transport
    // @step And measured usage and output remain observable without a final completion
    // @step And the committed provider checkpoint remains unchanged
    blocked_growth("incomplete").await
}

#[tokio::test]
async fn fitting_continuation_preserves_the_full_selected_request() -> TestResult {
    // @step Given an admitted initial request containing instructions schemas and required reminders
    let server = fixture::Server::start(
        vec![
            terminal("completed", "prior output", false),
            terminal("completed", "final output", true),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    history.transition_generation(7)?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    let request = CompletionRequestBuilder::new(
        model.clone(),
        "<system-reminder>required identity</system-reminder>",
    )
    .max_tokens(100)
    .preamble("mandatory instructions".into())
    .tools(vec![ToolDefinition {
        name: "inspect".into(),
        description: "schema retained".into(),
        parameters: json!({"type":"object","properties":{"path":{"type":"string"}}}),
    }])
    .build();
    // @step And replayable assistant output that fits the explicit remaining capacity
    let mut stream = model.stream(request).await?;
    // @step When Codex sends an internal continuation
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step Then both captured requests preserve the selected model generation and fixed context
    let requests = server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(request["model"], "shared-model");
        let bytes = serde_json::to_vec(request)?;
        assert!(bytes.len() + 100 <= 10000);
        let text = request.to_string();
        for required in [
            "mandatory instructions",
            "required identity",
            "schema retained",
        ] {
            assert_eq!(text.matches(required).count(), 1);
        }
        assert!(request.get("previous_response_id").is_none());
    }
    // @step And the continuation contains the prior output exactly once
    assert_eq!(requests[1].to_string().matches("prior output").count(), 1);
    assert_eq!(history.generation()?, 7);
    Ok(())
}

#[tokio::test]
async fn same_transport_transient_retry_preserves_admitted_content() -> TestResult {
    use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    // @step Given an admitted continuation request and one declared transient transport failure
    let server = MockServer::start().await;
    let index = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .respond_with(
            move |_: &wiremock::Request| match index.fetch_add(1, Ordering::SeqCst) {
                0 => ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(terminal("completed", "replayed once", false)),
                1 => ResponseTemplate::new(503)
                    .insert_header("retry-after", "0")
                    .set_body_string("transient"),
                _ => ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(terminal("completed", "done", true)),
            },
        )
        .mount(&server)
        .await;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model =
        CodexResponsesModel::new(client, "shared-model").with_request_admission(AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    // @step When the selected transport retries the continuation
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(100)
                .build(),
        )
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step Then each captured physical send preserves identical admitted bytes and lineage
    let requests = server
        .received_requests()
        .await
        .ok_or("missing captured requests")?;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].body, requests[2].body);
    assert_eq!(requests[1].url, requests[2].url);
    let body: Value = serde_json::from_slice(&requests[2].body)?;
    assert_eq!(body.to_string().matches("replayed once").count(), 1);
    Ok(())
}
