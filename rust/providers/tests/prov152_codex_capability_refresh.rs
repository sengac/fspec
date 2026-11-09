//! Feature: spec/features/codex-capability-aware-request-options.feature

use codelet_providers::codex::{
    history::CodexHistory,
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
        .expect(4)
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    // BUG-342: the Codex boundary admits a request only against known limits.
    let model = CodexResponsesModel::new(client, "not-a-catalog-model").with_request_admission(
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1_000),
        },
    );
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
    // @step Given a Codex model whose capability snapshot does not advertise xhigh
    // BUG-441: missing capabilities no longer reject anything; capabilities
    // that do not advertise xhigh do.
    let without_xhigh: CodexCapabilities = serde_json::from_value(json!({"efforts":["low"]}))?;
    model.set_capabilities(Some(without_xhigh))?;
    let error = cloned
        .completion(request())
        .await
        .err()
        .ok_or("capabilities without xhigh must reject it")?;
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
    // @step When the capabilities are refreshed and independent direct, streamed and numeric-effort requests are sent
    model.set_capabilities(Some(capabilities.clone()))?;
    // BUG-454: each request is its own conversation (a fresh history), so the
    // Codex lineage check does not compare it with the previous reply; the
    // clones share the refreshed capability snapshot.
    let independent = || cloned.clone().with_history(CodexHistory::default());
    independent().completion(request()).await?;
    let mut stream = independent().stream(request()).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    independent()
        .completion(
            CompletionRequestBuilder::new(cloned.clone(), "Hello")
                .preamble("Test instructions".into())
                .additional_params(json!({"reasoning":{"effort":"2048"}}))
                .build(),
        )
        .await?;
    // @step Then the server receives all three requests with the refreshed options
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
    // BUG-441: stale capabilities validate nothing — the explicit options
    // reach the server unchanged for it to accept or reject.
    independent().completion(request()).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("recording disabled")?;
    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests[3].body_json::<Value>()?["reasoning"]["effort"],
        "xhigh"
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
