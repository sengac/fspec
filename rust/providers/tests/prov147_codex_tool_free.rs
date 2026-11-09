use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use codelet_providers::CodexProvider;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov147.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn tool_free_codex_callers_use_the_compatibility_boundary() -> TestResult {
    // @step Given a Codex provider used for background learnings extraction
    let provider = CodexProvider::from_api_key("test-key", "shared-model")?.with_request_admission(
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1_000),
        },
    );
    // @step When a tool-free completion model is requested
    let model = provider.completion_model();
    // @step Then it uses the same Codex-owned boundary as the production agent
    assert!(std::any::type_name_of_val(&model).contains("CodexResponsesModel"));
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture::response()))
        .expect(1)
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model").with_request_admission(
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1_000),
        },
    );
    model
        .completion(
            CompletionRequestBuilder::new(model.clone(), "Extract learnings")
                .preamble("Extract learnings".into())
                .build(),
        )
        .await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("recording disabled")?;
    // @step And its requests disable storage and include encrypted reasoning
    let body: Value = requests[0].body_json()?;
    assert_eq!(body["store"], false);
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert!(body["tools"].as_array().is_none_or(Vec::is_empty));
    Ok(())
}

#[tokio::test]
async fn subscription_direct_completions_buffer_streaming_responses() -> TestResult {
    // @step Given a Codex subscription endpoint that only accepts streaming requests
    let server = MockServer::start().await;
    let response = fixture::response();
    let body = format!(
        "data: {}\n\n",
        serde_json::json!({"type":"response.completed","response":response})
    );
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model")
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1_000),
        })
        .with_streaming_completions();
    // @step When a direct completion is requested through the Codex boundary
    let result = model
        .completion(
            CompletionRequestBuilder::new(model.clone(), "Hello")
                .preamble("Test instructions".into())
                .build(),
        )
        .await?;
    // @step And the completed response retains its text and token usage
    assert_eq!(
        result.choice.first(),
        rig::message::AssistantContent::text("Hello")
    );
    assert_eq!(result.usage.total_tokens, 20);
    let requests = server
        .received_requests()
        .await
        .ok_or("recording disabled")?;
    // @step Then the request uses streaming with storage disabled
    let body: Value = requests[0].body_json()?;
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    Ok(())
}
