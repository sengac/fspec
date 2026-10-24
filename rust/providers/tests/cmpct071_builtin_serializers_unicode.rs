use codelet_providers::copilot::CopilotHttpClient;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestCaseError, TestRunner};
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::{anthropic, gemini, openai};
use serde_json::{json, Value};

#[path = "cmpct071_builtin_serializers/support.rs"]
mod support;
use support::{assert_request_physical, request, server, TestResult};

fn contains_description(value: &Value, description: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.get("description").and_then(Value::as_str) == Some(description)
                || object
                    .values()
                    .any(|nested| contains_description(nested, description))
        }
        Value::Array(array) => array
            .iter()
            .any(|nested| contains_description(nested, description)),
        _ => false,
    }
}

async fn probe<M: CompletionModel + 'static>(
    model: M,
    server: &wiremock::MockServer,
    path: &str,
    description: &str,
) -> TestResult {
    let mut request = request(model.clone(), json!({}))?;
    request.tools[0].description = description.into();
    let body = assert_request_physical(model, server, request, path).await?;
    assert!(contains_description(&body, description));
    Ok(())
}

async fn builtin_descriptions(description: &str) -> TestResult {
    let server = server().await;
    let chat = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    probe(chat, &server, "/chat/completions", description).await?;
    let server = support::server().await;
    let responses = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    probe(responses, &server, "/responses", description).await?;
    server.reset().await;
    let claude = anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    probe(claude, &server, "/v1/messages", description).await?;
    server.reset().await;
    let gemini = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    probe(
        gemini,
        &server,
        "/v1beta/models/test-model:streamGenerateContent",
        description,
    )
    .await?;
    server.reset().await;
    let copilot = openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .http_client(CopilotHttpClient::new("test-token".into()))
        .base_url(server.uri())
        .build()?
        .completion_model("claude-sonnet-4.5");
    probe(copilot, &server, "/chat/completions", description).await
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity-unicode.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_physical_serializers_preserve_arbitrary_unicode_tool_descriptions() -> TestResult {
    // @step Given selected builtin adapters with arbitrary Unicode tool descriptions
    let mut runner = TestRunner::new(Config {
        cases: 8,
        failure_persistence: None,
        ..Config::default()
    });
    let descriptions = proptest::collection::vec(any::<char>(), 1..80)
        .prop_map(|characters| characters.into_iter().collect::<String>());
    // @step When each request is prepared for admission then physically sent
    let result = runner.run(&descriptions, |description| {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(builtin_descriptions(&description))
                .map_err(|error| TestCaseError::fail(error.to_string()))
        })
    });
    // @step Then the physical bodies equal the admission bytes and retain the exact descriptions
    result?;
    Ok(())
}
