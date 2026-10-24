use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::openai;
use serde_json::{json, Value};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "cmpct071_builtin_serializers/support.rs"]
mod support;
use support::{assert_request_physical, request, server, TestResult};

fn assert_limit(body: &Value, limit: Option<u64>) {
    assert_eq!(body["seed"], 7);
    assert_eq!(body["parallel_tool_calls"], false);
    assert!(body.get("max_tokens").is_none());
    match limit {
        Some(limit) => assert_eq!(body["max_completion_tokens"], limit),
        None => assert!(body.get("max_completion_tokens").is_none()),
    }
}

async fn assert_selected_limits(streaming: bool) -> TestResult {
    for limit in [Some(8192), Some(4096), Some(u64::MAX), None] {
        let server = server().await;
        let model = openai::CompletionsClient::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("glm-4.7")
            .with_stream(streaming);
        let mut request = request(model.clone(), json!({"seed":7,"parallel_tool_calls":false}))?;
        request.max_tokens = limit;
        let body = assert_request_physical(model, &server, request, "/chat/completions").await?;
        assert_limit(&body, limit);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-output-limit-serialization.feature
/// @step Given a selected streaming Chat model with configured or absent core output limits
/// @step When each request is prepared for admission and physically streamed
/// @step Then exact prepared and physical bytes retain the selected output limit and optional parameters
/// @step And an absent limit stays omitted without introducing the legacy output limit field
#[tokio::test]
async fn scenario_streaming_chat_preserves_output_limits() -> TestResult {
    assert_selected_limits(true).await
}

/// Feature: spec/features/builtin-provider-output-limit-serialization.feature
/// @step Given a selected nonstreaming Chat model with configured or absent core output limits
/// @step When each request is prepared for admission and physically submitted
/// @step Then exact prepared and physical bytes retain the selected output limit and optional parameters
/// @step And an absent limit stays omitted without introducing the legacy output limit field
#[tokio::test]
async fn scenario_nonstreaming_chat_preserves_output_limits() -> TestResult {
    assert_selected_limits(false).await
}

/// Feature: spec/features/builtin-provider-output-limit-serialization.feature
#[tokio::test]
async fn scenario_direct_chat_preserves_output_limits() -> TestResult {
    // @step Given a selected direct Chat model with configured or absent core output limits
    for limit in [Some(8192), Some(4096), Some(u64::MAX), None] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"output-limit-response","object":"chat.completion","created":0,
                "model":"glm-4.7","choices":[{"index":0,
                    "message":{"role":"assistant","content":"verified"},"finish_reason":"stop"}],
                "usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}
            })))
            .mount(&server)
            .await;
        let model = openai::CompletionsClient::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("glm-4.7");
        let mut request = request(model.clone(), json!({"seed":7,"parallel_tool_calls":false}))?;
        request.max_tokens = limit;
        let original = format!("{request:?}");
        // @step When each request completes through the physical HTTP adapter
        model.completion(request.clone()).await?;
        assert_eq!(format!("{request:?}"), original);
        let requests = server
            .received_requests()
            .await
            .ok_or("missing direct completion capture")?;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/chat/completions");
        // @step Then physical requests retain the selected output limit and optional parameters
        // @step And an absent limit stays omitted without introducing the legacy output limit field
        assert_limit(&serde_json::from_slice(&requests[0].body)?, limit);
    }
    Ok(())
}
