//! Feature: spec/features/codex-cache-affinity-routing.feature

use codelet_providers::codex::{
    refreshing_client::RefreshingCodexClient, routing::CodexRoutingHandle, CodexResponsesModel,
};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::streaming::StreamedAssistantContent;
use serde_json::Value;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov147.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn setup() -> Result<
    (
        MockServer,
        CodexResponsesModel,
        ResponsesCompletionModel<RefreshingCodexClient>,
    ),
    Box<dyn std::error::Error>,
> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .insert_header("x-codex-turn-state", "opaque-server-state")
                .set_body_string(fixture::stream_body()),
        )
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("private-token")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok((
        server,
        CodexResponsesModel::new(client.clone(), "model"),
        ResponsesCompletionModel::new(client, "model"),
    ))
}

async fn run<Model: CompletionModel<StreamingResponse = rig::providers::openai::responses_api::streaming::StreamingCompletionResponse>>(model: Model) -> Result<u64, Box<dyn std::error::Error>>{
    let request = CompletionRequestBuilder::new(model.clone(), "private prompt")
        .preamble("instructions".into())
        .build();
    let mut stream = model.stream(request).await?;
    let mut cached = 0;
    while let Some(event) = stream.next().await {
        if let StreamedAssistantContent::Final(response) = event? {
            cached = response
                .usage
                .input_tokens_details
                .map(|details| details.cached_tokens)
                .unwrap_or(0);
        }
    }
    Ok(cached)
}

fn key(request: &wiremock::Request) -> Result<String, Box<dyn std::error::Error>> {
    let body: Value = request.body_json()?;
    let key = body["prompt_cache_key"]
        .as_str()
        .ok_or("missing affinity")?;
    uuid::Uuid::parse_str(key)?;
    assert_eq!(request.headers["session_id"].to_str()?, key);
    assert!(!key.contains("private"));
    Ok(key.to_owned())
}

#[tokio::test]
async fn requests_retain_opaque_affinity_only_within_their_session() -> TestResult {
    // @step Given two independent Codex sessions including a subordinate session
    // @step When each session makes repeated streamed requests
    // @step Then each session sends a stable distinct opaque cache key and session header
    let (server, model, _) = setup().await?;
    let parent = model
        .clone()
        .with_routing_handle(CodexRoutingHandle::default());
    let child = model.with_routing_handle(CodexRoutingHandle::default());
    run(parent.clone()).await?;
    run(parent).await?;
    run(child.clone()).await?;
    run(child).await?;
    let requests = server.received_requests().await.ok_or("requests")?;
    assert_eq!(key(&requests[0])?, key(&requests[1])?);
    assert_eq!(key(&requests[2])?, key(&requests[3])?);
    assert_ne!(key(&requests[0])?, key(&requests[2])?);
    assert!(!requests[2].headers.contains_key("x-codex-turn-state"));
    Ok(())
}

#[tokio::test]
async fn turn_state_survives_follow_ups_but_not_a_new_user_turn() -> TestResult {
    // @step Given a Codex session receiving server turn state
    // @step When follow-ups and a new user turn send requests
    // @step Then only follow-ups return the first valid turn state
    let (server, model, _) = setup().await?;
    let handle = CodexRoutingHandle::default();
    let model = model.with_routing_handle(handle.clone());
    run(model.clone()).await?;
    run(model.clone()).await?;
    handle.begin_turn();
    run(model).await?;
    let requests = server.received_requests().await.ok_or("requests")?;
    assert!(!requests[0].headers.contains_key("x-codex-turn-state"));
    assert_eq!(
        requests[1].headers["x-codex-turn-state"],
        "opaque-server-state"
    );
    assert!(!requests[2].headers.contains_key("x-codex-turn-state"));
    assert_eq!(key(&requests[0])?, key(&requests[2])?);
    assert!(!format!("{handle:?}").contains("opaque-server-state"));
    Ok(())
}

#[tokio::test]
async fn reset_and_account_changes_invalidate_routing() -> TestResult {
    // @step Given a session with cached affinity and server turn state
    // @step When reset stale recovery or account change occurs
    // @step Then requests use fresh affinity without old turn state and late responses are ignored
    let (server, model, _) = setup().await?;
    let handle = CodexRoutingHandle::default();
    let model = model.with_routing_handle(handle.clone());
    run(model.clone()).await?;
    handle.reset();
    run(model.clone()).await?;
    let previous = handle.bind_account("private-account-a");
    handle.bind_account("private-account-b");
    previous.observe_turn_state("stale-state");
    run(model).await?;
    let requests = server.received_requests().await.ok_or("requests")?;
    assert_ne!(key(&requests[0])?, key(&requests[1])?);
    assert_ne!(key(&requests[1])?, key(&requests[2])?);
    assert!(!requests[1].headers.contains_key("x-codex-turn-state"));
    assert!(!requests[2].headers.contains_key("x-codex-turn-state"));
    Ok(())
}

#[tokio::test]
async fn missing_affinity_support_preserves_ordinary_streaming_and_usage() -> TestResult {
    // @step Given Codex without routing and a non-Codex Responses client
    // @step When both stream responses with cached token usage
    // @step Then neither receives routing additions and cached usage remains accurate
    let (server, model, ordinary) = setup().await?;
    let codex_cached = run(model).await?;
    let ordinary_cached = run(ordinary).await?;
    assert_eq!(codex_cached, ordinary_cached);
    let requests = server.received_requests().await.ok_or("requests")?;
    for request in requests {
        assert!(request
            .body_json::<Value>()?
            .get("prompt_cache_key")
            .is_none());
        assert!(!request.headers.contains_key("session_id"));
        assert!(!request.headers.contains_key("x-codex-turn-state"));
    }
    Ok(())
}
