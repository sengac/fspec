use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder, Document, ToolDefinition};
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn model(
    script: &std::path::Path,
    url: &str,
) -> Result<RhaiCustomProviderModel, Box<dyn std::error::Error>> {
    let config: ProviderConfig = serde_json::from_value(json!({
        "name":"parity", "display_name":"Parity", "base_url":url,
        "script":script, "auth":{"type":"bearer","env_var":"CMPCT_UNUSED"},
        "models":{"smart":{"id":"selected", "context_window":100000,
            "max_output_tokens":4096, "supports_streaming":true, "supports_tools":true}},
        "tool_style":"openai", "api_style":"openai_chat"
    }))?;
    Ok(RhaiCustomProviderModel::new(Arc::new(
        RhaiCustomProvider::new(
            Arc::new(config),
            Arc::new(ScriptLoader::with_default_engine()),
            "smart".into(),
        )?,
    )))
}

const SCRIPT: &str = r#"
fn build_request(request) { #{ ordinary: true } }
fn build_stream_request(request) {
    #{ stream: true, messages: request.messages, tools: request.tools,
       thinking: request.thinking_config, mandatory: "selected script overhead" }
}
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url }
fn parse_response(raw) { #{ content: "done", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { #{ kind: "text", text: "done" } }
fn map_error(status, body) { #{ type: "api", message: body } }
"#;

#[tokio::test]
async fn custom_preparation_matches_the_selected_streaming_request() -> TestResult {
    // @step Given a custom provider script with different ordinary and streaming request bodies
    let directory = setup_temp_directory()?;
    let script = directory.path().join("parity.rhai");
    std::fs::write(&script, SCRIPT)?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: [DONE]\n\n"),
        )
        .mount(&server)
        .await;
    let model = model(&script, &server.uri())?;
    let request = CompletionRequestBuilder::new(model.clone(), "verified suffix and DAG")
        .preamble("mandatory instructions and file reminders".into())
        .documents(vec![Document {
            id: "required-evidence".into(),
            text: "retrieval document must survive".into(),
            additional_props: Default::default(),
        }])
        .tools(vec![codelet_tools::tool_attribution::decorate_definition(
            ToolDefinition {
                name: "selected_tool".into(),
                description: "mandatory schema".into(),
                parameters: json!({"type":"object","properties":{"evidence":{"type":"string"}}}),
            },
        )?])
        .additional_params(json!({"thinking_config":{"budget_tokens":1024}}))
        .build();
    let unchanged = format!("{request:?}");
    // @step When mandatory context including retrieval documents tools and thinking configuration are prepared then streamed
    let bytes = model.prepare_request(request.clone())?;
    assert_eq!(
        bytes,
        model
            .clone()
            .prepare_request_for_generation(request.clone(), 3)?
    );
    assert_eq!(format!("{request:?}"), unchanged);
    assert!(server
        .received_requests()
        .await
        .ok_or("missing capture")?
        .is_empty());
    let mut stream = model.stream(request.clone()).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let captured = server.received_requests().await.ok_or("missing capture")?;
    assert_eq!(captured.len(), 1);
    // @step Then prepared bytes equal the physical streaming body without an extra request
    assert_eq!(captured[0].body, bytes);
    let body: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(body["stream"], true);
    assert!(body.get("ordinary").is_none());
    assert_eq!(body["thinking"]["thinking_config"]["budget_tokens"], 1024);
    let text = String::from_utf8(bytes)?;
    for mandatory in [
        "verified suffix and DAG",
        "mandatory instructions",
        "file reminders",
        "selected_tool",
        "mandatory schema",
        "selected script overhead",
        "required-evidence",
        "retrieval document must survive",
    ] {
        assert!(text.contains(mandatory), "missing {mandatory}");
    }
    // @step And the source request remains unchanged
    assert_eq!(format!("{request:?}"), unchanged);
    Ok(())
}

#[tokio::test]
async fn custom_preparation_rejects_a_script_builder_failure() -> TestResult {
    // @step Given a selected custom provider whose streaming request builder fails
    let directory = setup_temp_directory()?;
    let script = directory.path().join("broken.rhai");
    std::fs::write(&script, SCRIPT.replace("#{ stream: true, messages: request.messages, tools: request.tools,\n       thinking: request.thinking_config, mandatory: \"selected script overhead\" }", "throw \"selected builder failure\";"))?;
    let server = MockServer::start().await;
    let model = model(&script, &server.uri())?;
    let request = CompletionRequestBuilder::new(model.clone(), "unchanged DAG").build();
    let preparer = SelectedRequestPreparer::new(
        model,
        request.clone(),
        AdmissionLimits {
            context_capacity: Some(100000),
            output_reservation: Some(4096),
        },
    );
    let messages: Vec<_> = request.chat_history.into_iter().collect();
    // @step When admission prepares its next request
    let failure = preparer
        .prepare(&messages, 1)
        .err()
        .ok_or("script cannot authorize admission")?;
    // @step Then preparation returns an explicit error without physical inference
    assert!(
        matches!(&failure, RequestAdmissionError::Preparation(message) if message.contains("selected builder failure")),
        "{failure}"
    );
    assert!(server
        .received_requests()
        .await
        .ok_or("missing capture")?
        .is_empty());
    Ok(())
}
