use axum::{Router, routing::post};
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequestBuilder, GetTokenUsage, Usage,
};
use rig::providers::{gemini, openai};
use rig::streaming::StreamedAssistantContent;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Endpoint {
    url: String,
    requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn endpoint(body: String, streaming: bool) -> Result<Endpoint, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let content_type = if streaming {
        "text/event-stream"
    } else {
        "application/json"
    };
    let router = Router::new().fallback(post(move || {
        let body = body.clone();
        let observed = observed.clone();
        async move {
            observed.fetch_add(1, Ordering::SeqCst);
            ([("content-type", content_type)], body)
        }
    }));
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Endpoint {
        url: format!("http://{address}"),
        requests,
        task,
    })
}

#[derive(Default)]
struct Observed {
    errors: Vec<String>,
    finals: Vec<Option<Usage>>,
    usage: Vec<Usage>,
}

async fn collect<M: CompletionModel>(model: &M) -> Result<Observed, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "retained evidence").build();
    let mut observed = Observed::default();
    let mut response = match model.stream(request).await {
        Ok(response) => response,
        Err(error) => {
            observed.errors.push(error.to_string());
            return Ok(observed);
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(event) = response.next().await {
            match event {
                Err(error) => observed.errors.push(error.to_string()),
                Ok(StreamedAssistantContent::Final(response)) => {
                    observed.finals.push(response.token_usage())
                }
                Ok(StreamedAssistantContent::Usage(usage)) => observed.usage.push(usage),
                _ => {}
            }
        }
    })
    .await?;
    Ok(observed)
}

fn assert_rejected(observed: &Observed, message: &str) {
    assert!(
        observed.errors.iter().any(|error| error.contains(message)),
        "expected {message}, received {:?}",
        observed.errors
    );
    assert!(
        observed.finals.is_empty(),
        "invalid evidence cannot complete a response"
    );
    assert!(
        observed.usage.is_empty(),
        "invalid evidence cannot publish measured usage"
    );
}

fn sse(response: Value) -> String {
    format!("data: {response}\n\n")
}

fn chat_body(usage: Value, streaming: bool) -> String {
    if streaming {
        format!(
            "{}data: [DONE]\n\n",
            sse(json!({"choices":[],"usage":usage}))
        )
    } else {
        json!({"id":"physical-response","object":"chat.completion","created":0,
            "model":"selected-model","choices":[{"index":0,"message":{
                "role":"assistant","content":"retained response"},"finish_reason":"stop"}],
            "usage":usage})
        .to_string()
    }
}

fn responses_body(usage: Value) -> String {
    sse(json!({"type":"response.completed","sequence_number":1,
        "response":{"id":"physical-response","object":"response","created_at":0,
            "status":"completed","model":"selected-model","output":[],"usage":usage}}))
}

fn gemini_body(usage: Value) -> String {
    sse(
        json!({"candidates":[{"content":{"role":"model","parts":[{"text":"retained response"}]},
        "finishReason":"STOP","index":0}],"usageMetadata":usage}),
    )
}

fn chat_invalid_usage() -> Vec<Value> {
    vec![
        json!({"prompt_tokens":10,"completion_tokens":2,"total_tokens":11}),
        json!({"prompt_tokens":10,"completion_tokens":2,"total_tokens":12,"prompt_tokens_details":{"cached_tokens":11}}),
        json!({"prompt_tokens":10,"completion_tokens":2,"total_tokens":12,"completion_tokens_details":{"reasoning_tokens":3}}),
        json!({"prompt_tokens":10,"total_tokens":9}),
    ]
}

/// Feature: spec/features/builtin-provider-terminal-usage-evidence.feature
#[tokio::test]
async fn inconsistent_chat_usage_cannot_complete_either_request_mode() -> TestResult {
    // @step Given physical OpenAI Chat responses contain inconsistent totals or subset measurements
    for usage in chat_invalid_usage() {
        for streaming in [true, false] {
            let endpoint = endpoint(chat_body(usage.clone(), streaming), streaming).await?;
            let model = openai::CompletionsClient::<reqwest::Client>::builder()
                .api_key("test-key")
                .base_url(&endpoint.url)
                .build()?
                .completion_model("selected-model")
                .with_stream(streaming);
            // @step When streaming nonstreaming and direct adapters consume those responses
            let observed = collect(&model).await?;
            // @step Then each reports inconsistent usage without publishing a successful terminal response
            assert_rejected(&observed, "inconsistent OpenAI usage");
            assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
            if !streaming {
                let request =
                    CompletionRequestBuilder::new(model.clone(), "retained evidence").build();
                assert!(matches!(model.completion(request).await,
                    Err(CompletionError::ResponseError(message)) if message == "inconsistent OpenAI usage"));
                assert_eq!(endpoint.requests.load(Ordering::SeqCst), 2);
            }
        }
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-terminal-usage-evidence.feature
#[tokio::test]
async fn responses_terminal_usage_must_be_present_and_consistent() -> TestResult {
    // @step Given physical OpenAI Responses streams omit terminal usage or contain inconsistent measurements
    for usage in [
        Value::Null,
        json!({"input_tokens":10,"output_tokens":2,"total_tokens":11,"output_tokens_details":{"reasoning_tokens":0}}),
        json!({"input_tokens":10,"output_tokens":2,"total_tokens":12,"input_tokens_details":{"cached_tokens":11},"output_tokens_details":{"reasoning_tokens":0}}),
        json!({"input_tokens":10,"output_tokens":2,"total_tokens":12,"output_tokens_details":{"reasoning_tokens":3}}),
    ] {
        let missing = usage.is_null();
        let endpoint = endpoint(responses_body(usage), true).await?;
        let model = openai::Client::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(&endpoint.url)
            .build()?
            .completion_model("selected-model");
        // @step When the selected adapter consumes the complete stream
        let observed = collect(&model).await?;
        // @step Then it reports missing or inconsistent usage without measured zero or terminal success
        assert_rejected(
            &observed,
            if missing {
                "missing Responses usage"
            } else {
                "inconsistent Responses usage"
            },
        );
        assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-terminal-usage-evidence.feature
#[tokio::test]
async fn gemini_terminal_usage_must_be_present_and_consistent() -> TestResult {
    // @step Given physical Gemini streams omit terminal usage or contain inconsistent measurements
    for usage in [
        Value::Null,
        json!({"promptTokenCount":10,"candidatesTokenCount":2,"totalTokenCount":11}),
        json!({"promptTokenCount":10,"candidatesTokenCount":2,"totalTokenCount":12,"cachedContentTokenCount":11}),
        json!({"promptTokenCount":10,"candidatesTokenCount":-1,"totalTokenCount":9}),
    ] {
        let missing = usage.is_null();
        let endpoint = endpoint(gemini_body(usage), true).await?;
        let model = gemini::Client::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(&endpoint.url)
            .build()?
            .completion_model("selected-model");
        // @step When the selected adapter consumes the complete stream
        let observed = collect(&model).await?;
        // @step Then it reports missing or inconsistent usage without measured zero or terminal success
        assert_rejected(
            &observed,
            if missing {
                "missing Gemini usage"
            } else {
                "inconsistent Gemini usage"
            },
        );
        assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-terminal-usage-evidence.feature
#[tokio::test]
async fn explicit_measured_zero_completes_responses_and_gemini_streams() -> TestResult {
    // @step Given physical Responses and Gemini streams explicitly measure zero usage
    let endpoint = endpoint(
        responses_body(json!({"input_tokens":0,"output_tokens":0,
        "total_tokens":0,"output_tokens_details":{"reasoning_tokens":0}})),
        true,
    )
    .await?;
    let model = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&endpoint.url)
        .build()?
        .completion_model("selected-model");
    // @step When the selected adapters consume the complete streams
    let responses = collect(&model).await?;
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    let endpoint = self::endpoint(
        gemini_body(json!({"promptTokenCount":0,
        "candidatesTokenCount":0,"totalTokenCount":0})),
        true,
    )
    .await?;
    let model = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&endpoint.url)
        .build()?
        .completion_model("selected-model");
    let gemini = collect(&model).await?;
    assert_eq!(endpoint.requests.load(Ordering::SeqCst), 1);
    // @step Then each returns measured zero usage and exactly one successful terminal response
    for observed in [responses, gemini] {
        assert!(observed.errors.is_empty(), "{:?}", observed.errors);
        assert_eq!(observed.finals.len(), 1);
        let usage = observed.finals[0].ok_or("missing measured zero")?;
        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, None);
        assert_eq!(observed.usage.len(), 1);
        assert_eq!(observed.usage[0], usage);
    }
    Ok(())
}
