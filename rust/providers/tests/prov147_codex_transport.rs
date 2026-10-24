//! Feature: spec/features/codex-only-adapter-boundary.feature

use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use codelet_providers::{CodexProvider, OpenAIProvider};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov147.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn models(
    server: &MockServer,
) -> Result<
    (
        CodexResponsesModel,
        ResponsesCompletionModel<RefreshingCodexClient>,
    ),
    Box<dyn std::error::Error>,
> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok((
        CodexResponsesModel::new(client.clone(), "shared-model"),
        ResponsesCompletionModel::new(client, "shared-model"),
    ))
}

fn request<Model: CompletionModel>(model: Model) -> rig::completion::CompletionRequest {
    CompletionRequestBuilder::new(model, "Hello")
        .preamble("Test instructions".to_string())
        .additional_params(json!({"store": false,
            "reasoning": {"effort": "high", "summary": "auto"},
            "include": ["reasoning.encrypted_content"]}))
        .build()
}

async fn assert_requests(server: &MockServer, streaming: bool) -> TestResult {
    let requests = server
        .received_requests()
        .await
        .ok_or("request recording disabled")?;
    assert_eq!(requests.len(), 2);
    let codex: Value = requests[0].body_json()?;
    let ordinary: Value = requests[1].body_json()?;
    assert_eq!(codex, ordinary);
    assert_eq!(codex["model"], "shared-model");
    assert_eq!(codex["instructions"], "Test instructions");
    assert_eq!(codex["store"], false);
    assert_eq!(
        codex["reasoning"],
        json!({"effort": "high", "summary": "auto"})
    );
    assert_eq!(codex["include"], json!(["reasoning.encrypted_content"]));
    assert_eq!(codex["stream"].as_bool().unwrap_or(false), streaming);
    assert_eq!(codex["input"][0]["content"][0]["text"], "Hello");
    Ok(())
}

async fn collect_stream<Model>(model: Model) -> Result<Vec<String>, rig::completion::CompletionError>
where
    Model: CompletionModel<StreamingResponse = rig::providers::openai::responses_api::streaming::StreamingCompletionResponse>,
{
    let mut stream = model.stream(request(model.clone())).await?;
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Text(text) => events.push(format!("text:{}", text.text)),
            StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                events.push(format!("reasoning:{reasoning}"))
            }
            StreamedAssistantContent::ToolCall(call) => {
                assert_eq!(call.id, "fc_test");
                assert_eq!(call.call_id.as_deref(), Some("call_test"));
                assert_eq!(call.function.arguments, json!({"path": "."}));
                events.push(format!("tool:{}", call.function.name));
            }
            StreamedAssistantContent::Usage(usage) => {
                assert_eq!(usage.input_tokens, 12);
                assert_eq!(usage.output_tokens, 8);
                assert_eq!(usage.cache_read_input_tokens, Some(3));
                assert_eq!(usage.reasoning_tokens, Some(4));
                events.push(format!("usage:{}", usage.total_tokens));
            }
            StreamedAssistantContent::Final(response) => {
                events.push(format!("final:{}", response.usage.total_tokens))
            }
            other => events.push(format!("{other:?}")),
        }
    }
    Ok(events)
}

#[tokio::test]
async fn scenario_existing_streamed_responses_remain_unchanged() -> TestResult {
    // @step Given a local response containing text reasoning function calls and usage
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(fixture::stream_body()),
        )
        .expect(2)
        .mount(&server)
        .await;
    let (codex, ordinary) = models(&server)?;
    // @step When the response is streamed through the Codex boundary
    let events = match collect_stream(codex).await {
        Ok(events) => events,
        Err(err) => {
            eprintln!("DEBUG codex stream error: {err:?}");
            return Err(Box::from(format!("codex stream: {err}")));
        }
    };
    let ordinary_events = match collect_stream(ordinary).await {
        Ok(events) => events,
        Err(err) => {
            eprintln!("DEBUG ordinary stream error: {err:?}");
            return Err(Box::from(format!("ordinary stream: {err}")));
        }
    };
    // @step Then its request and events match the existing Responses behavior
    assert_eq!(events, ordinary_events);
    assert_eq!(
        events,
        [
            "text:Hello",
            "reasoning:Thinking",
            "usage:20",
            "tool:inspect",
            "final:20"
        ]
    );
    // @step And ordinary OpenAI Responses requests remain unchanged
    assert_requests(&server, true).await
}

#[tokio::test]
async fn scenario_existing_direct_completions_remain_unchanged() -> TestResult {
    // @step Given a local completed response with text and usage
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture::response()))
        .expect(2)
        .mount(&server)
        .await;
    let (codex, ordinary) = models(&server)?;
    // @step When a completion is requested through the Codex boundary
    let result = match codex.completion(request(codex.clone())).await {
        Ok(r) => r,
        Err(err) => {
            eprintln!("DEBUG codex completion error: {err:?}");
            return Err(Box::from(format!("codex completion: {err}")));
        }
    };
    let baseline = match ordinary.completion(request(ordinary.clone())).await {
        Ok(r) => r,
        Err(err) => {
            eprintln!("DEBUG ordinary completion error: {err:?}");
            return Err(Box::from(format!("ordinary completion: {err}")));
        }
    };
    // @step Then its request and result match the existing Responses behavior
    assert_eq!(
        serde_json::to_value(&result.choice)?,
        serde_json::to_value(&baseline.choice)?
    );
    assert_eq!(
        serde_json::to_value(&result.raw_response)?,
        serde_json::to_value(&baseline.raw_response)?
    );
    assert_eq!(result.usage.total_tokens, 20);
    assert_eq!(
        result.choice.first(),
        rig::message::AssistantContent::text("Hello")
    );
    // @step And ordinary OpenAI Responses requests remain unchanged
    assert_requests(&server, false).await
}

#[tokio::test]
async fn scenario_provider_identity_selects_the_codex_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step And model names without Codex still use the boundary when Codex is selected
    for model in ["gpt-codex-shared", "ordinary-model"] {
        // @step Given Codex and OpenAI providers configured with identical model names
        let codex = CodexProvider::from_api_key("test-key", model)?;
        let openai = OpenAIProvider::from_api_key("test-key", model)?;
        // @step When their production agents are created
        let codex_agent = codex.create_rig_agent(uuid::Uuid::new_v4(), None, None);
        let openai_agent = openai.create_rig_agent(uuid::Uuid::new_v4(), None, None);
        // @step Then only Codex uses the Codex-owned compatibility boundary
        assert!(
            std::any::type_name_of_val(&codex_agent).contains("codex::model::CodexResponsesModel")
        );
        assert!(!std::any::type_name_of_val(&openai_agent)
            .contains("codex::model::CodexResponsesModel"));
    }
    Ok(())
}
