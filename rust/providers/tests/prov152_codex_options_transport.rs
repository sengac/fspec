//! Feature: spec/features/codex-capability-aware-request-options.feature

use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov147.rs"]
pub mod fixture;

#[tokio::test]
// @step Given Codex and ordinary OpenAI models have the same model name
// @step When each provider sends its configured request
// @step Then only Codex applies capability resolution
// @step And ordinary OpenAI request serialization remains unchanged
async fn scenario_non_codex_requests_retain_their_existing_configuration(
) -> Result<(), Box<dyn std::error::Error>> {
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
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let codex = CodexResponsesModel::new(client.clone(), "same-name");
    let ordinary = ResponsesCompletionModel::new(client, "same-name");
    let config = json!({"store":true,"reasoning":{"effort":"low"}});
    let request = CompletionRequestBuilder::new(codex.clone(), "hello")
        .preamble("Test instructions".into())
        .additional_params(config.clone())
        .build();
    let mut stream = codex.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let request = CompletionRequestBuilder::new(ordinary.clone(), "hello")
        .preamble("Test instructions".into())
        .additional_params(config)
        .build();
    let mut stream = ordinary.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let requests = server
        .received_requests()
        .await
        .ok_or("request recording disabled")?;
    assert_eq!(requests.len(), 2);
    let codex_body: Value = requests[0].body_json()?;
    let ordinary_body: Value = requests[1].body_json()?;
    assert_eq!(codex_body["store"], false);
    assert_eq!(
        codex_body["include"],
        json!(["reasoning.encrypted_content"])
    );
    assert_eq!(ordinary_body["store"], true);
    assert!(ordinary_body.get("include").is_none());
    assert_eq!(codex_body["reasoning"], ordinary_body["reasoning"]);
    assert_eq!(codex_body["model"], ordinary_body["model"]);
    Ok(())
}
