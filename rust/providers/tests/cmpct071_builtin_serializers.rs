use codelet_providers::claude_refreshing_client::RefreshingClaudeClient;
use codelet_providers::copilot::CopilotHttpClient;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::{anthropic, gemini, openai};
use serde_json::{json, Value};

#[path = "cmpct071_builtin_serializers/support.rs"]
mod support;
use support::{assert_request_physical, request, server, TestResult};

async fn assert_physical<M: CompletionModel + 'static>(
    model: M,
    server: &wiremock::MockServer,
    params: Value,
    expected_path: &str,
) -> TestResult<Value> {
    let request = request(model.clone(), params)?;
    assert_request_physical(model, server, request, expected_path).await
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn openai_chat_preparation_matches_streaming_and_nonstreaming_sends() -> TestResult {
    // @step Given selected OpenAI chat models with both transport modes and strict tools
    for streaming in [true, false] {
        let server = server().await;
        let model = openai::CompletionsClient::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("test-model")
            .with_stream(streaming)
            .with_strict_tools();
        // @step When requests containing mandatory context and parameters are prepared then sent
        let body = assert_physical(
            model,
            &server,
            json!({"seed":7,"parallel_tool_calls":false}),
            "/chat/completions",
        )
        .await?;
        // @step Then prepared bytes equal the physical bodies including the selected stream options
        // @step And preparation sends nothing and leaves the input request unchanged
        assert_eq!(body["stream"], streaming);
        assert_eq!(body["seed"], 7);
        assert_eq!(body["tools"][0]["function"]["strict"], true);
        if streaming {
            assert_eq!(body["stream_options"]["include_usage"], true);
        } else {
            assert!(body.get("stream_options").is_none());
        }
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn gemini_retains_documents_and_explicit_limits_without_optional_configuration() -> TestResult
{
    // @step Given a selected Gemini model with mandatory documents and explicit output limits
    let server = server().await;
    let model = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    let mut request = request(model.clone(), json!({}))?;
    request.additional_params = None;
    request.max_tokens = Some(1234);
    request.documents = vec![rig::completion::Document {
        id: "required-file".into(),
        text: "authoritative-file-evidence".into(),
        additional_props: Default::default(),
    }];
    // @step When its request has no optional generation configuration and is prepared then sent
    let body = assert_request_physical(
        model,
        &server,
        request,
        "/v1beta/models/test-model:streamGenerateContent",
    )
    .await?;
    // @step Then prepared bytes equal the physical body including documents and the explicit output limits
    // @step And preparation sends nothing and leaves the input request unchanged
    assert_eq!(
        (
            &body["generationConfig"]["maxOutputTokens"],
            &body["generationConfig"]["temperature"],
        ),
        (&json!(1234), &json!(0.4))
    );
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn openai_responses_preparation_matches_the_streaming_send() -> TestResult {
    // @step Given a selected OpenAI Responses model with mandatory context and tool schemas
    let server = server().await;
    let model = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    // @step When its request is prepared then sent
    let body = assert_physical(
        model,
        &server,
        json!({"store":false,"metadata":{"evidence":"selected"}}),
        "/responses",
    )
    .await?;
    // @step Then prepared bytes equal the physical body including streaming and tool schemas
    // @step And preparation sends nothing and leaves the input request unchanged
    assert_eq!(body["stream"], true);
    assert_eq!(body["store"], false);
    assert_eq!(body["metadata"]["evidence"], "selected");
    assert_eq!(body["tools"][0]["name"], "selected_tool");
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn anthropic_preparation_matches_api_key_and_oauth_streaming_sends() -> TestResult {
    // @step Given selected Anthropic API key and OAuth models with caching and tool schemas
    for oauth in [false, true] {
        let server = server().await;
        let api_key = if oauth { "sk-ant-oat-test" } else { "test-key" };
        let client = if oauth {
            RefreshingClaudeClient::new_oauth(
                api_key.into(),
                "unused-refresh".into(),
                Some(3600),
                server.uri(),
            )
        } else {
            RefreshingClaudeClient::new_api_key()
        };
        let model = anthropic::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(client)
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        // @step When requests containing mandatory context and parameters are prepared then sent
        let body = assert_physical(model, &server, json!({"top_p":0.9}), "/v1/messages").await?;
        // @step Then prepared bytes equal the physical bodies including stable OAuth metadata
        // @step And preparation sends nothing and leaves the input request unchanged
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 4096);
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(body["top_p"], 0.9);
        if oauth {
            assert!(body["metadata"]["user_id"]
                .as_str()
                .is_some_and(|user| user.starts_with("user_")));
            assert!(body.get("temperature").is_none());
            assert!(body.get("tool_choice").is_none());
        } else {
            assert_eq!(body["temperature"], 0.4);
        }
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn gemini_preparation_matches_the_streaming_send() -> TestResult {
    // @step Given a selected Gemini model with mandatory context and generation parameters
    let server = server().await;
    let model = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    // @step When its request is prepared then sent
    let body = assert_physical(
        model,
        &server,
        json!({"generationConfig":{"topP":0.9}}),
        "/v1beta/models/test-model:streamGenerateContent",
    )
    .await?;
    // @step Then prepared bytes equal the physical body including generation parameters
    // @step And preparation sends nothing and leaves the input request unchanged
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 4096);
    assert_eq!(body["generationConfig"]["temperature"], 0.4);
    assert_eq!(body["generationConfig"]["topP"], 0.9);
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity.feature
#[tokio::test]
async fn copilot_preparation_includes_middleware_transformations() -> TestResult {
    // @step Given selected Copilot models using the actual HTTP middleware
    for name in ["claude-sonnet-4.5", "gpt-4o", "gemini-2.5-pro"] {
        let server = server().await;
        let model = openai::CompletionsClient::<reqwest::Client>::builder()
            .api_key("test-key")
            .http_client(CopilotHttpClient::new("test-token".into()))
            .base_url(server.uri())
            .build()?
            .completion_model(name)
            .with_stream(true);
        // @step When requests containing mandatory context and parameters are prepared then sent
        let body =
            assert_physical(model, &server, json!({"store":false}), "/chat/completions").await?;
        // @step Then prepared bytes equal the physical bodies including eligible cache markers
        // @step And preparation sends nothing and leaves the input request unchanged
        if name.starts_with("claude-") {
            assert_eq!(
                body["messages"][0]["copilot_cache_control"]["type"],
                "ephemeral"
            );
            assert_eq!(
                body["tools"][0]["copilot_cache_control"]["type"],
                "ephemeral"
            );
        } else {
            assert!(body["messages"][0].get("copilot_cache_control").is_none());
        }
    }
    let server = server().await;
    let model = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .http_client(CopilotHttpClient::new("test-token".into()))
        .base_url(server.uri())
        .build()?
        .completion_model("gpt-5");
    assert_physical(model, &server, json!({"store":false}), "/responses").await?;
    Ok(())
}
