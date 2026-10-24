//! Feature: spec/features/gemini-contentless-terminal-metadata.feature

use axum::{Router, routing::post};
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage, Usage};
use rig::providers::gemini;
use rig::streaming::StreamedAssistantContent;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Endpoint {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn endpoint(body: String) -> Result<Endpoint, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let router = Router::new().fallback(post(move || {
        let body = body.clone();
        async move { ([("content-type", "text/event-stream")], body) }
    }));
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Endpoint {
        url: format!("http://{address}"),
        task,
    })
}

#[derive(Default, Debug)]
struct Observed {
    errors: Vec<String>,
    finals: Vec<gemini::streaming::StreamingCompletionResponse>,
    usage: Vec<Usage>,
    text: String,
}

async fn collect(body: String) -> Result<Observed, Box<dyn std::error::Error>> {
    let endpoint = endpoint(body).await?;
    let model = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&endpoint.url)
        .build()?
        .completion_model("selected-model");
    let request = CompletionRequestBuilder::new(model.clone(), "terminal metadata").build();
    let mut response = model.stream(request).await?;
    let mut observed = Observed::default();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(event) = response.next().await {
            match event {
                Err(error) => observed.errors.push(error.to_string()),
                Ok(StreamedAssistantContent::Final(response)) => observed.finals.push(response),
                Ok(StreamedAssistantContent::Usage(usage)) => observed.usage.push(usage),
                Ok(StreamedAssistantContent::Text(text)) => observed.text.push_str(&text.text),
                _ => {}
            }
        }
    })
    .await?;
    Ok(observed)
}

fn sse(frame: Value) -> String {
    format!("data: {frame}\n\n")
}

fn terminal(content: Option<Value>, reason: &str, usage: Option<Value>) -> String {
    let mut candidate = json!({"finishReason": reason, "index": 0});
    if let Some(content) = content {
        candidate["content"] = content;
    }
    let mut frame = json!({"candidates": [candidate]});
    if let Some(usage) = usage {
        frame["usageMetadata"] = usage;
    }
    sse(frame)
}

fn measured_usage() -> Value {
    json!({"promptTokenCount": 10, "candidatesTokenCount": 2, "thoughtsTokenCount": 3,
        "cachedContentTokenCount": 4, "totalTokenCount": 15})
}

fn content_shapes() -> [Option<Value>; 3] {
    [
        None,
        Some(Value::Null),
        Some(json!({"role": "model", "parts": []})),
    ]
}

#[tokio::test]
async fn contentless_terminal_candidates_preserve_finish_reasons_and_measured_usage() -> TestResult
{
    // @step Given Gemini streams emit text followed by terminal candidates with absent null or empty content
    let prefix = sse(json!({"candidates": [{"content": {
        "role": "model", "parts": [{"text": "retained response"}]
    }}]}));
    for content in content_shapes() {
        for (wire, normalized) in [
            ("STOP", "end_turn"),
            ("MAX_TOKENS", "max_tokens"),
            ("SAFETY", "content_filter"),
            ("RECITATION", "content_filter"),
            ("LANGUAGE", "content_filter"),
            ("BLOCKLIST", "content_filter"),
            ("PROHIBITED_CONTENT", "content_filter"),
            ("SPII", "content_filter"),
            ("MALFORMED_FUNCTION_CALL", "end_turn"),
            ("OTHER", "unknown"),
            ("FINISH_REASON_UNSPECIFIED", "unknown"),
        ] {
            let body = format!(
                "{prefix}{}",
                terminal(content.clone(), wire, Some(measured_usage()))
            );
            // @step When the builtin adapter consumes each supported terminal finish reason
            let observed = collect(body).await?;
            // @step Then each stream retains the text and normalized finish reason with exactly one measured terminal response
            assert!(observed.errors.is_empty(), "{wire}: {observed:?}");
            assert_eq!(observed.text, "retained response");
            assert_eq!(observed.finals.len(), 1, "{wire}");
            let response = &observed.finals[0];
            assert_eq!(response.stop_reason(), Some(normalized));
            let usage = response.token_usage().ok_or("missing measured usage")?;
            assert_eq!(usage.input_tokens, 6);
            assert_eq!(usage.output_tokens, 2);
            assert_eq!(usage.total_tokens, 15);
            assert_eq!(usage.cache_read_input_tokens, Some(4));
            assert_eq!(usage.reasoning_tokens, Some(3));
            assert_eq!(observed.usage, vec![usage]);
        }
    }
    Ok(())
}

#[tokio::test]
async fn contentless_terminal_candidates_accept_explicitly_measured_zero() -> TestResult {
    // @step Given Gemini terminal candidates have absent or null content and explicitly measured zero usage
    for content in [None, Some(Value::Null)] {
        let body = terminal(
            content,
            "STOP",
            Some(json!({
                "promptTokenCount": 0, "candidatesTokenCount": 0, "totalTokenCount": 0
            })),
        );
        // @step When the builtin adapter consumes the streams
        let observed = collect(body).await?;
        // @step Then each stream completes once with measured zero and the normalized finish reason
        assert!(observed.errors.is_empty(), "{observed:?}");
        assert_eq!(observed.finals.len(), 1);
        assert_eq!(observed.finals[0].stop_reason(), Some("end_turn"));
        let usage = observed.finals[0]
            .token_usage()
            .ok_or("missing measured zero")?;
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, None);
        assert_eq!(usage.reasoning_tokens, None);
        assert_eq!(observed.usage, vec![usage]);
    }
    Ok(())
}

#[tokio::test]
async fn contentless_terminal_candidates_reject_missing_or_inconsistent_usage() -> TestResult {
    // @step Given Gemini streams have valid earlier usage but terminal candidates with absent null or empty content and invalid usage
    let prefix = sse(json!({"candidates": [{"content": {
        "role": "model", "parts": [{"text": "retained response"}]
    }}], "usageMetadata": measured_usage()}));
    for content in content_shapes() {
        for (usage, expected) in [
            (None, "missing Gemini usage"),
            (Some(Value::Null), "missing Gemini usage"),
            (
                Some(json!({"promptTokenCount":10,"candidatesTokenCount":2,"totalTokenCount":11})),
                "inconsistent Gemini usage",
            ),
            (
                Some(
                    json!({"promptTokenCount":10,"candidatesTokenCount":2,"totalTokenCount":12,"cachedContentTokenCount":11}),
                ),
                "inconsistent Gemini usage",
            ),
            (
                Some(json!({"promptTokenCount":10,"candidatesTokenCount":-1,"totalTokenCount":9})),
                "inconsistent Gemini usage",
            ),
        ] {
            let body = format!("{prefix}{}", terminal(content.clone(), "STOP", usage));
            // @step When the builtin adapter consumes missing and inconsistent terminal measurements
            let observed = collect(body).await?;
            // @step Then each stream reports the appropriate usage error without a successful terminal response
            assert_eq!(observed.errors.len(), 1, "{observed:?}");
            assert!(observed.errors[0].contains(expected), "{observed:?}");
            assert!(observed.finals.is_empty(), "{observed:?}");
            assert_eq!(
                observed.usage.len(),
                1,
                "only earlier valid usage may be emitted"
            );
            assert_eq!(observed.usage[0].total_tokens, 15);
        }
    }
    Ok(())
}
