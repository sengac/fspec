//! Feature: spec/features/compaction-synthesis-admission.feature

#[path = "support/bug211_budget.rs"]
mod budget;
#[path = "support/bug210_completion_metrics.rs"]
mod metrics;

use codelet_agent_loop::generate_compaction_handler::execute_compaction_subagent;
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial_test::serial]
async fn scenario_synthesis_respects_provider_requirements_for_historical_tool_evidence() {
    // @step Given a provider requires tool declarations alongside historical tool evidence
    use rig::client::CompletionClient;
    let directory = codelet_test_helpers::setup_temp_directory().expect("standalone project");
    let session_id = uuid::Uuid::new_v4();
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        directory.path().to_path_buf(),
    )
    .expect("trusted standalone compactor context");
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let body: serde_json::Value = request.body_json().expect("Claude request");
            let synthesis = body["tool_choice"]["type"] == "none";
            if body["tools"].as_array().is_none_or(Vec::is_empty) {
                return ResponseTemplate::new(400).set_body_json(json!({"type":"error",
                    "error":{"type":"invalid_request_error", "message":"tool history requires tool definitions"}}));
            }
            let content = if synthesis {
                json!([{"type":"text", "text":metrics::DAG}])
            } else {
                json!([{"type":"tool_use", "id":"evidence", "name":"SessionSearch",
                    "input":{"work_unit_id":"NONE",
                        "action_type":"recent", "count":1}}])
            };
            ResponseTemplate::new(200).set_body_json(json!({"id":"claude-budget",
                "type":"message", "role":"assistant", "model":"selected-claude-model",
                "content":content, "stop_reason":if synthesis {"end_turn"} else {"tool_use"},
                "stop_sequence":null, "usage":{"input_tokens":100, "output_tokens":10}}))
        }).mount(&server).await;
    let client: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("offline-test")
        .base_url(server.uri())
        .build()
        .expect("Claude client");
    let agent = client
        .agent("selected-claude-model")
        .preamble("Build a complete DAG")
        .max_tokens(4096)
        .tool(codelet_tools::SessionSearchTool::new(session_id))
        .build();
    // @step When an exhausted compactor makes its final synthesis request
    let result = codelet_agent_loop::compaction_execution::run(
        agent,
        session_id,
        "Build the summary",
        "claude",
        2,
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    )
    .await;
    assert!(result.is_ok(), "provider-compatible synthesis: {result:?}");
    // @step Then historical evidence remains valid and further tool use is disabled
    let requests = server.received_requests().await.expect("requests");
    let bodies = requests
        .iter()
        .map(|request| request.body_json::<serde_json::Value>().expect("JSON"))
        .collect::<Vec<_>>();
    assert!(bodies[0]["tools"][0]["input_schema"]["required"]
        .as_array()
        .is_some_and(|required| required.contains(&serde_json::json!("work_unit_id"))));
    assert_eq!(
        bodies
            .iter()
            .filter(|body| body["tool_choice"]["type"] == "none")
            .count(),
        1
    );
    let final_body = bodies.last().expect("synthesis request");
    assert_eq!(final_body["model"], "selected-claude-model");
    assert!(final_body["messages"].to_string().contains("tool_result"));
    // @step And the summary is returned without another exploration round
    assert_eq!(result.expect("summary"), metrics::DAG);
    assert_eq!(requests.len(), 5);
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_oversized_synthesis_is_rejected_before_provider_dispatch() {
    // @step Given gathered evidence exceeds the selected synthesis request budget
    let fixture = metrics::Fixture::new().await.expect("durable fixture");
    budget::exploration(&fixture.server, fixture.session.id, metrics::DAG, false).await;
    // @step When the compactor finishes its exploration budget
    let result = execute_compaction_subagent(
        &std::env::temp_dir(),
        fixture.session.id,
        None,
        &Arc::new(AtomicBool::new(true)),
        "openai",
        Some("o3"),
        Some(8500),
        Some(8192),
    )
    .await;
    // @step Then synthesis fails admission without sending an oversized request
    assert!(result.is_err(), "oversized synthesis must fail locally");
    assert!(result
        .expect_err("admission rejection")
        .contains("exceeds capacity"));
    let requests = fixture.server.received_requests().await.expect("requests");
    assert!(requests.iter().all(budget::has_tools));
}
