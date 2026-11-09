//! Feature: spec/features/builtin-provider-nonstreaming-rejection-evidence.feature
//!
//! CMPCT-071: a nonstreaming OpenAI Chat model keeps the physical HTTP rejection
//! evidence. Moved out of cmpct065_openai_usage_evidence.rs so each test file
//! maps one feature (COV-075); the endpoint helpers are copied from there.

use axum::{Router, routing::post};
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Model = openai::completion::CompletionModel;
type Event = Result<
    StreamedAssistantContent<openai::completion::streaming::StreamingCompletionResponse>,
    CompletionError,
>;

struct Endpoint {
    model: Model,
    requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn endpoint_with_status(
    body: String,
    streaming: bool,
    status: axum::http::StatusCode,
) -> Result<Endpoint, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let content_type = if streaming {
        "text/event-stream"
    } else {
        "application/json"
    };
    let router = Router::new().route(
        "/chat/completions",
        post(move || {
            let body = body.clone();
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, Ordering::SeqCst);
                (status, [("content-type", content_type)], body)
            }
        }),
    );
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    let model = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(format!("http://{address}"))
        .build()?
        .completion_model("selected-model")
        .with_stream(streaming);
    Ok(Endpoint {
        model,
        requests,
        task,
    })
}

async fn collect(model: &Model) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "retained suffix").build();
    let mut response = match model.stream(request).await {
        Ok(response) => response,
        Err(error) => return Ok(vec![Err(error)]),
    };
    Ok(
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut events = Vec::new();
            while let Some(event) = response.next().await {
                events.push(event);
            }
            events
        })
        .await?,
    )
}

/// Feature: spec/features/builtin-provider-nonstreaming-rejection-evidence.feature
#[tokio::test]
async fn nonstreaming_openai_chat_preserves_physical_http_rejection_evidence() -> TestResult {
    // @step Given a selected nonstreaming OpenAI Chat model whose physical endpoint rejects requests
    for (status, body) in [
        (
            400,
            r#"{"error":{"code":"context_length_exceeded","message":"too long"}}"#,
        ),
        (
            401,
            r#"{"error":{"code":"invalid_api_key","message":"context_length_exceeded is not the code"}}"#,
        ),
        (400, "context_length_exceeded is only unstructured text"),
    ] {
        let endpoint = endpoint_with_status(
            body.into(),
            false,
            axum::http::StatusCode::from_u16(status)?,
        )
        .await?;
        // @step When requests receive structured overflow, authentication, and plain text rejection bodies
        let events = collect(&endpoint.model).await?;
        // @step Then each stream exposes the original typed HTTP status and exact body
        assert!(
            matches!(events.first(), Some(Err(CompletionError::HttpError(
            rig::http_client::Error::InvalidStatusCodeWithMessage(actual_status, actual_body)
        ))) if actual_status.as_u16() == status && actual_body == body),
            "physical rejection lost its typed status or body"
        );
        // @step And no rejected stream emits a successful final response
        assert_eq!(events.len(), 1);
        assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    }
    Ok(())
}
