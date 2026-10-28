//! Feature: spec/features/codex-compactor-occupancy-runway.feature

#[path = "support/bug211_budget.rs"]
pub mod budget;
#[path = "support/bug210_completion_metrics.rs"]
pub mod metrics;

use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn message(text: &str) -> Value {
    json!({"type":"message", "id":"runway-text", "role":"assistant", "status":"completed",
        "content":[{"type":"output_text", "text":text, "annotations":[]}]})
}

fn terminal(item: Value, end_turn: bool) -> ResponseTemplate {
    let done = json!({"type":"response.output_item.done", "output_index":0, "item":item});
    let completed = json!({"type":"response.completed", "response":{
        "id":"runway-response", "status":"completed", "object":"response", "created_at":1,
        "model":"selected-codex-model", "output":[item], "end_turn":end_turn,
        "usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}});
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {done}\n\ndata: {completed}\n\n"))
}

fn model(
    server: &MockServer,
    limits: AdmissionLimits,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("offline-runway")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "selected-codex-model")
        .with_request_admission(limits)
        .with_streaming_completions())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_codex_continuation_capacity_recovery_retains_final_tool_evidence(
) -> metrics::TestResult {
    // @step Given a streaming compactor has gathered a persisted original evidence page
    let fixture = metrics::Fixture::new().await?;
    let builder = budget::codex_exploration(&fixture.server, fixture.session.id).await;
    fixture.server.reset().await;
    let target = fixture.session.id;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("Codex request");
            let tools = body["tools"].as_array().is_some_and(|tools| !tools.is_empty());
            let evidence = body["input"].as_array().expect("input").iter()
                .any(|item| item["type"] == "function_call_output");
            if tools && !evidence {
                terminal(json!({"type":"function_call", "id":"page-item", "call_id":"final-page",
                    "name":"SessionSearch", "arguments":json!({"work_unit_id":"NONE",
                        "action_type":"originals",
                        "session_id":target, "start_turn":0, "end_turn":0, "limit":1}).to_string()}), true)
            } else if tools {
                terminal(message(&"replayed physical output ".repeat(10_000)), false)
            } else { terminal(message(metrics::DAG), true) }
        }).mount(&fixture.server).await;
    // @step When replayed Codex output makes a physical exploration continuation exceed capacity
    let limits = AdmissionLimits {
        context_capacity: Some(200_000),
        output_reservation: Some(4096),
    };
    let result = codelet_agent_loop::compaction_execution::run(
        builder.agent.clone(),
        fixture.session.id,
        "Build the complete summary",
        "codex",
        20,
        limits,
    )
    .await;
    // @step Then the rejected continuation never reaches transport and tool-free synthesis retains the complete page
    assert_eq!(result?, metrics::DAG);
    let requests = fixture.server.received_requests().await.ok_or("requests")?;
    assert_eq!(requests.len(), 3);
    for request in &requests {
        codelet_providers::request_admission::admit_complete_request(&request.body, limits)?;
    }
    let exploration: Value = requests[1].body_json()?;
    let synthesis: Value = requests[2].body_json()?;
    let final_page = |body: &Value| {
        body["input"]
            .as_array()
            .expect("input")
            .iter()
            .find(|item| item["type"] == "function_call_output" && item["call_id"] == "final-page")
            .cloned()
    };
    assert_eq!(final_page(&exploration), final_page(&synthesis));
    assert!(final_page(&synthesis).ok_or("final evidence")?["output"]
        .to_string()
        .contains("original 0"));
    assert!(synthesis["tools"].as_array().is_none_or(Vec::is_empty));
    // @step And the selected model and output reservation remain unchanged
    assert_eq!(synthesis["model"], "selected-codex-model");
    assert!(requests
        .iter()
        .all(|request| request.body.len() as u64 + 4096 <= 200_000));
    Ok(())
}

#[tokio::test]
async fn scenario_codex_physical_requests_respect_the_exact_incident_boundary(
) -> metrics::TestResult {
    // @step Given a Codex request has capacity 1050000 and output reservation 128000
    let limits = AdmissionLimits {
        context_capacity: Some(1_050_000),
        output_reservation: Some(128_000),
    };
    // @step When complete initial and continuation requests contain 922000 or 922001 estimated input units
    for continuation in [false, true] {
        for occupancy in [922_000, 922_001] {
            let server = MockServer::start().await;
            let model = model(&server, limits)?;
            let mut request = CompletionRequestBuilder::new(model.clone(), "boundary")
                .max_tokens(128_000)
                .preamble("x".into())
                .build();
            let mut wire: Value = serde_json::from_slice(&model.prepare_request(request.clone())?)?;
            let replay = if continuation {
                wire["input"]
                    .as_array_mut()
                    .ok_or("input")?
                    .push(message("x"));
                let padding = occupancy - serde_json::to_vec(&wire)?.len() + 1;
                message(&"x".repeat(padding))
            } else {
                let padding = occupancy - serde_json::to_vec(&wire)?.len() + 1;
                request.preamble = Some("x".repeat(padding));
                message("final")
            };
            let count = Arc::new(AtomicUsize::new(0));
            let observed = count.clone();
            Mock::given(wiremock::matchers::method("POST"))
                .respond_with(move |_: &wiremock::Request| {
                    if observed.fetch_add(1, Ordering::SeqCst) == 0 && continuation {
                        terminal(replay.clone(), false)
                    } else {
                        terminal(message("final"), true)
                    }
                })
                .mount(&server)
                .await;
            let mut stream = model.stream(request).await?;
            let mut capacity_failure = false;
            while let Some(item) = stream.next().await {
                match item {
                    Err(CompletionError::RequestError(error)) => {
                        let error = error
                            .downcast_ref::<RequestAdmissionError>()
                            .ok_or("typed admission")?;
                        assert!(matches!(
                            error,
                            RequestAdmissionError::BudgetExceeded {
                                input_tokens: 922_001,
                                ..
                            }
                        ));
                        capacity_failure = true;
                    }
                    Err(error) => return Err(error.into()),
                    _ => {}
                }
            }
            // @step Then only requests at or below 922000 reach transport
            assert_eq!(capacity_failure, occupancy == 922_001);
            let requests = server.received_requests().await.ok_or("requests")?;
            assert_eq!(
                requests.len(),
                usize::from(continuation) + usize::from(occupancy == 922_000)
            );
            for request in &requests {
                codelet_providers::request_admission::admit_complete_request(
                    &request.body,
                    limits,
                )?;
                let body: Value = request.body_json()?;
                assert_eq!(body["max_output_tokens"], 128_000);
            }
            if occupancy == 922_000 {
                assert_eq!(
                    requests.last().ok_or("fitting request")?.body.len(),
                    occupancy
                );
            }
        }
    }
    Ok(())
}
