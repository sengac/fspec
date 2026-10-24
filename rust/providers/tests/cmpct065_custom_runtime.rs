use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn custom_live_and_final_usage_share_the_declared_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a scripted custom provider reports disjoint cache and reasoning categories
    let directory = setup_temp_directory()?;
    let script = directory.path().join("usage.rhai");
    std::fs::write(
        &script,
        r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn usage() { #{ input_tokens: 600, output_tokens: 70,
    cache_read_input_tokens: 400, cache_creation_input_tokens: 50, reasoning_tokens: 30 } }
fn parse_response(raw) { #{ content: "done", stop_reason: "end_turn", usage: usage() } }
fn parse_stream_chunk(config, data) { let result = usage(); result.kind = "usage"; result }
"#,
    )?;
    let server = MockServer::start().await;
    let config: ProviderConfig = serde_json::from_value(json!({
        "name":"accounting", "display_name":"Accounting", "base_url":server.uri(),
        "script":script, "auth":{"type":"bearer","env_var":"CMPCT_UNUSED"},
        "models":{"smart":{"id":"accounting-model", "context_window":10000,
            "max_output_tokens":200, "supports_streaming":true, "supports_tools":true}},
        "tool_style":"openai", "api_style":"openai_chat"
    }))?;
    let provider = RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "smart".into(),
    )?;
    let model = RhaiCustomProviderModel::new(Arc::new(provider));
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: measured\n\ndata: [DONE]\n\n"),
        )
        .mount(&server)
        .await;
    // @step When its real streaming and nonstreaming adapters deliver usage
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?;
    let mut measurements = Vec::new();
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Usage(usage) => measurements.push(usage),
            StreamedAssistantContent::Final(raw) => measurements.extend(raw.token_usage()),
            _ => {}
        }
    }
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;
    let response = model
        .completion(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?;
    measurements.push(response.usage);
    measurements.extend(response.raw_response.token_usage());
    // @step Then live final and nonstreaming occupancy agree without dropping detail
    assert_eq!(measurements.len(), 4);
    for usage in measurements {
        assert_eq!(usage.total_tokens, 1150);
        assert_eq!(usage.cache_read_input_tokens, Some(400));
        assert_eq!(usage.cache_creation_input_tokens, Some(50));
        assert_eq!(usage.reasoning_tokens, Some(30));
    }
    Ok(())
}
