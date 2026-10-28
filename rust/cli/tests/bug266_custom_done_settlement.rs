//! Feature: spec/features/custom-done-settlement.feature

#[path = "support/prov161_runtime.rs"]
pub mod runtime;

use codelet_cli::{interactive::StreamEvent, session::Session};
use codelet_core::RigAgent;
use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_tools::DoneTool;
use runtime::{Output, RuntimeFixture, TestResult};
use serde_json::json;
use std::sync::Arc;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

const SCRIPT: &str = r#"
fn build_url(config) { config.base_url }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_request(request) { #{ messages: request.messages } }
fn build_stream_request(request) { #{ stream: true, tools: request.tools } }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    if data == "tool" { return #{ kind: "tool_call_delta", index: 0, id: "native-done",
        name: "Done", arguments: "{\"summary\":\"custom complete\",\"work_unit_id\":\"NONE\"}" }; }
    if data == "stop" { return #{ kind: "stop", reason: "tool_use" }; }
    if data == "usage" { return #{ kind: "usage", input_tokens: 12, output_tokens: 3 }; }
    throw "unexpected payload";
}
"#;

#[tokio::test]
#[serial_test::serial]
async fn native_done_cannot_discard_trailing_physical_usage() -> TestResult {
    // @step Given a local custom stream queues an attributed armed native Done before stop and trailing usage
    let environment = RuntimeFixture::new()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: tool\n\ndata: stop\n\ndata: usage\n\ndata: [DONE]\n\n"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let script = directory.path().join("custom.rhai");
    std::fs::write(&script, SCRIPT)?;
    let config: ProviderConfig = serde_json::from_value(json!({
        "name":"bug266", "display_name":"BUG-266", "base_url":server.uri(),
        "script":script, "models":{"smart":{"id":"offline"}}
    }))?;
    let provider = RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "smart".into(),
    )?;
    let model = RhaiCustomProviderModel::new(Arc::new(provider));
    let manager = ProviderManager::for_testing(
        ProviderType::Custom("bug266".into()),
        Some(200_000),
        Some(4096),
    );
    let session_id = environment.session_id("bug266")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    codelet_tools::set_continue_armed(session_id, true);
    let agent = rig::agent::AgentBuilder::new(model)
        .tool(DoneTool::new(session_id))
        .build();
    let mut session = Session::from_provider_manager(manager);
    session.continue_enabled = true;
    let output = Output::default();
    // @step When normal admission and the shared stream loop consume the response
    let outcome = runtime::run(RigAgent::new(agent, 2), &mut session, session_id, &output).await;
    codelet_tools::set_continue_armed(session_id, false);
    // @step Then exact physical usage settles once before accepted Done without a nudge
    outcome?;
    {
        let events = output.0.lock().map_err(|error| error.to_string())?;
        assert!(!events.iter().any(|event| matches!(event, StreamEvent::ToolResult(result) if result.content.contains("invalid work_unit_id"))));
        assert!(events.iter().any(
            |event| matches!(event, StreamEvent::Status(text) if text.contains("done: custom complete"))
        ));
    }
    let manifest = codelet_core::persistence::load_session(session_id)?;
    assert_eq!(manifest.token_usage.cumulative_billed_input, 12);
    assert_eq!(manifest.token_usage.cumulative_billed_output, 3);
    assert_eq!(session.continue_nudges_used, 0);
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("missing capture")?
            .len(),
        1
    );
    Ok(())
}
