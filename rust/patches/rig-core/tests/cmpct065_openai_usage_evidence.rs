use axum::{Router, routing::post};
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
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

async fn endpoint(body: String, streaming: bool) -> Result<Endpoint, Box<dyn std::error::Error>> {
    endpoint_with_status(body, streaming, axum::http::StatusCode::OK).await
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

fn body(streaming: bool, measured: bool) -> String {
    let mut response = if streaming {
        json!({"choices":[{"delta":{"content":"retained response"},"finish_reason":"stop"}]})
    } else {
        json!({"id":"physical-response","object":"chat.completion","created":0,
            "model":"selected-model","choices":[{"index":0,"message":{
                "role":"assistant","content":"retained response"},"finish_reason":"stop"}]})
    };
    if measured {
        response["usage"] = json!({"prompt_tokens":0,"completion_tokens":0,"total_tokens":0});
    }
    if streaming {
        format!("data: {response}\n\ndata: [DONE]\n\n")
    } else {
        response.to_string()
    }
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

fn assert_missing(events: &[Event]) {
    assert!(
        events.iter().any(|event| matches!(event,
        Err(CompletionError::ResponseError(message)) if message == "missing OpenAI usage")),
        "missing provider evidence must remain explicit"
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            Ok(StreamedAssistantContent::Usage(_) | StreamedAssistantContent::Final(_))
        )),
        "missing usage must not become measured zero or successful completion"
    );
}

/// Feature: spec/features/builtin-provider-openai-usage-evidence.feature
#[tokio::test]
async fn streaming_chat_missing_usage_is_rejected() -> TestResult {
    // @step Given a physical OpenAI Chat stream omits usage measurements
    let endpoint = endpoint(body(true, false), true).await?;
    // @step When the selected adapter consumes the complete response
    let events = collect(&endpoint.model).await?;
    // @step Then it reports missing usage without a measured zero or successful final
    assert_missing(&events);
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    Ok(())
}

/// Feature: spec/features/builtin-provider-structured-stream-rejection.feature
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

/// Feature: spec/features/builtin-provider-openai-usage-evidence.feature
#[tokio::test]
async fn nonstreaming_chat_missing_usage_is_rejected() -> TestResult {
    // @step Given a physical nonstreaming OpenAI Chat response omits usage measurements
    let endpoint = endpoint(body(false, false), false).await?;
    // @step When the selected streaming adapter consumes the complete response
    let events = collect(&endpoint.model).await?;
    // @step Then it reports missing usage without a measured zero or successful final
    assert_missing(&events);
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    Ok(())
}

/// Feature: spec/features/builtin-provider-openai-usage-evidence.feature
#[tokio::test]
async fn direct_chat_missing_usage_is_rejected() -> TestResult {
    // @step Given a physical direct OpenAI Chat response omits usage measurements
    let endpoint = endpoint(body(false, false), false).await?;
    let request = CompletionRequestBuilder::new(endpoint.model.clone(), "retained suffix").build();
    // @step When the selected adapter completes the request
    let response = endpoint.model.completion(request).await;
    // @step Then it reports missing usage instead of measured zero occupancy
    assert!(
        matches!(response, Err(CompletionError::ResponseError(message))
        if message == "missing OpenAI usage")
    );
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    Ok(())
}

/// Feature: spec/features/builtin-provider-openai-usage-evidence.feature
#[tokio::test]
async fn explicit_measured_zero_remains_valid_at_every_chat_boundary() -> TestResult {
    // @step Given physical OpenAI Chat responses explicitly report zero input output and total usage
    for streaming in [true, false] {
        let endpoint = endpoint(body(streaming, true), streaming).await?;
        // @step When streaming nonstreaming and direct adapters consume those responses
        let events = collect(&endpoint.model).await?;
        // @step Then each returns measured zero usage without inventing missing evidence
        let mut finals = 0;
        for event in events {
            match event? {
                StreamedAssistantContent::Usage(usage) => assert_eq!(usage.total_tokens, 0),
                StreamedAssistantContent::Final(response) => {
                    let usage = response.token_usage().ok_or("missing measured zero")?;
                    assert_eq!(usage.total_tokens, 0);
                    finals += 1;
                }
                _ => {}
            }
        }
        assert_eq!(finals, 1);
        assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    }
    let endpoint = endpoint(body(false, true), false).await?;
    let request = CompletionRequestBuilder::new(endpoint.model.clone(), "retained suffix").build();
    let response = endpoint.model.completion(request).await?;
    assert_eq!(response.usage.total_tokens, 0);
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    Ok(())
}
