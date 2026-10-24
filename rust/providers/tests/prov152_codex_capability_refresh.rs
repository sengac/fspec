//! Feature: spec/features/codex-capability-aware-request-options.feature

use codelet_providers::codex::{
    refreshing_client::RefreshingCodexClient,
    request_options::{CodexCapabilities, RequestOptionsError},
    CodexResponsesModel,
};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use serde_json::{json, Value};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "fixtures/prov147.rs"]
pub mod fixture;

#[tokio::test]
async fn scenario_capability_refresh_reaches_direct_and_streamed_requests(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            if request
                .body_json::<Value>()
                .map(|body| body["stream"] == true)
                .unwrap_or(false)
            {
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(fixture::stream_body())
            } else {
                ResponseTemplate::new(200).set_body_json(fixture::response())
            }
        })
        .expect(3)
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "not-a-catalog-model");
    let cloned = model.clone();
    let config = json!({"reasoning":{"effort":"xhigh","summary":"auto","context":"all_turns"},
        "text":{"verbosity":"low"}, "service_tier":"priority",
        "stream_options":{"reasoning_summary_delivery":"sequential_cutoff"}});
    let request = || {
        CompletionRequestBuilder::new(cloned.clone(), "Hello")
            .preamble("Test instructions".into())
            .additional_params(config.clone())
            .build()
    };
    let error = cloned
        .completion(request())
        .await
        .err()
        .ok_or("missing capabilities must reject xhigh")?;
    assert!(matches!(error, CompletionError::RequestError(ref source)
        if source.downcast_ref::<RequestOptionsError>().is_some()));
    assert!(server
        .received_requests()
        .await
        .ok_or("recording disabled")?
        .is_empty());
    let capabilities: CodexCapabilities = serde_json::from_value(json!({
        "efforts":["xhigh",2048], "summaries":["auto"], "contexts":["all_turns"],
        "verbosities":["low"], "service_tiers":["priority"], "sequential_cutoff":true
    }))?;
    model.set_capabilities(Some(capabilities.clone()))?;
    cloned.completion(request()).await?;
    let mut stream = cloned.stream(request()).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    cloned
        .completion(
            CompletionRequestBuilder::new(cloned.clone(), "Hello")
                .preamble("Test instructions".into())
                .additional_params(json!({"reasoning":{"effort":"2048"}}))
                .build(),
        )
        .await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("recording disabled")?;
    for request in &requests[..2] {
        let body: Value = request.body_json()?;
        for (key, value) in config.as_object().ok_or("expected object")? {
            assert_eq!(&body[key], value);
        }
        assert_eq!(body["store"], false);
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
    }
    assert_eq!(
        requests[2].body_json::<Value>()?["reasoning"]["effort"],
        2048
    );
    model.set_capabilities(Some(CodexCapabilities {
        stale: true,
        ..capabilities
    }))?;
    assert!(cloned.completion(request()).await.is_err());
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("recording disabled")?
            .len(),
        3
    );
    Ok(())
}

#[tokio::test]
async fn scenario_agent_preserves_explicit_options() -> Result<(), Box<dyn std::error::Error>> {
    let provider = codelet_providers::CodexProvider::from_api_key("test-key", "same-name")?;
    let config = json!({"text":{"verbosity":"low"},"service_tier":"priority"});
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, Some(config.clone()));
    assert_eq!(agent.additional_params, Some(config));
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);
    assert!(agent.additional_params.is_none());
    Ok(())
}
