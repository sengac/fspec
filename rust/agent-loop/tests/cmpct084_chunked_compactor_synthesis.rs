//! Feature: spec/features/chunked-compactor-synthesis.feature

use rig::client::CompletionClient;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FIRST: &str =
    "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Early evidence\">Turns zero and one</dag-node>";
const SECOND: &str =
    "<dag-node depth=\"D1\" turns=\"2-3\" label=\"Later evidence\">Turns two and three</dag-node>";
const FRAGMENT: &str = "<dag-node depth=\"D1\" turns=\"2-3\" label=\"Later evidence\">half-writ";

fn completion(message: Value, finish_reason: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id":"cmpct084", "object":"chat.completion", "created":1, "model":"local-reasoner",
        "choices":[{"index":0,"message":message,"finish_reason":finish_reason}],
        "usage":{"prompt_tokens":100,"completion_tokens":4096,"total_tokens":4196}
    }))
}

fn has_tools(body: &Value) -> bool {
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}

/// Exploration requests receive a SessionSearch call until max depth ends
/// exploration; tool-free synthesis requests receive `synthesis` in order
/// (the last entry repeats).
async fn mount(server: &MockServer, synthesis: Vec<(String, &'static str)>) {
    let served = Arc::new(AtomicUsize::new(0));
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("compactor JSON");
            if has_tools(&body) {
                return completion(
                    json!({"role":"assistant", "content":null, "tool_calls":[{
                        "id":"evidence", "type":"function", "function":{
                            "name":"SessionSearch",
                            "arguments":json!({"work_unit_id":"NONE",
                                "action_type":"recent", "count":1}).to_string()}}]}),
                    "tool_calls",
                );
            }
            let index = served
                .fetch_add(1, Ordering::SeqCst)
                .min(synthesis.len() - 1);
            let (content, finish) = &synthesis[index];
            completion(
                json!({"role":"assistant", "content":content,
                    "reasoning_content":"Weigh the evidence before writing nodes."}),
                finish,
            )
        })
        .mount(server)
        .await;
}

async fn run_compactor(server: &MockServer) -> Result<String, String> {
    let directory = codelet_test_helpers::setup_temp_directory().map_err(|e| e.to_string())?;
    let session_id = uuid::Uuid::new_v4();
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        directory.path().to_path_buf(),
    )
    .map_err(|error| error.to_string())?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline-cmpct084")
            .base_url(server.uri())
            .build()
            .map_err(|error| error.to_string())?;
    let agent = client
        .agent("local-reasoner")
        .preamble("Build a complete DAG")
        .max_tokens(4096)
        .tool(codelet_tools::SessionSearchTool::new(session_id))
        .build();
    codelet_agent_loop::compaction_execution::run(
        agent,
        session_id,
        "Build the summary",
        "openai",
        2,
        codelet_cli::compaction_operation::BuildRequest::First,
        &std::sync::Mutex::new(None),
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    )
    .await
}

async fn synthesis_bodies(server: &MockServer) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(server
        .received_requests()
        .await
        .ok_or("requests")?
        .iter()
        .map(|request| request.body_json::<Value>())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|body| !has_tools(body))
        .collect())
}

fn last_user_text(body: &Value) -> String {
    body["messages"]
        .as_array()
        .and_then(|messages| messages.iter().rev().find(|m| m["role"] == "user"))
        .map(|message| message["content"].to_string())
        .unwrap_or_default()
}

fn assistant_dag_turns(body: &Value) -> Vec<String> {
    body["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|message| message["role"] == "assistant")
        .map(|message| message["content"].to_string())
        .filter(|content| content.contains("dag-node"))
        .collect()
}

#[tokio::test]
#[serial_test::serial]
async fn a_synthesis_truncated_mid_node_continues_from_the_next_node_and_commits_the_complete_dag(
) -> TestResult {
    // @step Given a compactor whose synthesis returns one complete node and half of the next before the output limit
    // @step And the continuation response returns the remaining node and stops normally
    let server = MockServer::start().await;
    mount(
        &server,
        vec![
            (format!("{FIRST}\n{FRAGMENT}"), "length"),
            (SECOND.to_string(), "stop"),
        ],
    )
    .await;
    // @step When the compactor runs
    let candidate = run_compactor(&server).await?;
    // @step Then two synthesis requests are sent
    let synthesis = synthesis_bodies(&server).await?;
    assert_eq!(synthesis.len(), 2);
    // @step And the continuation request carries only the complete node and asks for the remaining nodes
    let carried = assistant_dag_turns(&synthesis[1]);
    assert_eq!(carried.len(), 1, "{carried:?}");
    assert!(carried[0].contains("Turns zero and one"), "{carried:?}");
    assert!(!carried[0].contains("half-writ"), "{carried:?}");
    let instruction = last_user_text(&synthesis[1]);
    assert!(instruction.contains("output token limit"), "{instruction}");
    assert!(instruction.contains("remaining"), "{instruction}");
    // @step And the compactor candidate contains both nodes and not the partial fragment
    assert!(candidate.contains(FIRST), "{candidate}");
    assert!(candidate.contains(SECOND), "{candidate}");
    assert!(!candidate.contains("half-writ"), "{candidate}");
    codelet_core::compaction::validate_dag_structure(&candidate)?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_synthesis_that_spends_the_output_limit_on_reasoning_is_asked_to_write_the_next_node_immediately(
) -> TestResult {
    // @step Given a compactor whose first synthesis response contains no text and stops at the output limit
    // @step And the next response returns the complete DAG and stops normally
    let server = MockServer::start().await;
    let complete = format!("{FIRST}\n{SECOND}");
    mount(
        &server,
        vec![(String::new(), "length"), (complete.clone(), "stop")],
    )
    .await;
    // @step When the compactor runs
    let candidate = run_compactor(&server).await?;
    // @step Then the continuation request has no assistant turn and asks for the next node immediately
    let synthesis = synthesis_bodies(&server).await?;
    assert_eq!(synthesis.len(), 2);
    assert!(assistant_dag_turns(&synthesis[1]).is_empty());
    let instruction = last_user_text(&synthesis[1]);
    assert!(instruction.contains("immediately"), "{instruction}");
    // @step And the compactor candidate is the complete DAG
    assert_eq!(candidate.trim(), complete);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn repeated_output_limit_stops_without_progress_fail_with_an_output_limit_error() -> TestResult
{
    // @step Given a compactor whose every synthesis response contains no text and stops at the output limit
    let server = MockServer::start().await;
    mount(&server, vec![(String::new(), "length")]).await;
    // @step When the compactor runs
    let result = run_compactor(&server).await;
    // @step Then the compactor fails with an error stating the output limit was reached
    let error = result.err().ok_or("compactor must fail")?;
    assert!(error.contains("output limit"), "{error}");
    // @step And no further synthesis request is sent after the second chunk
    assert_eq!(synthesis_bodies(&server).await?.len(), 2);
    Ok(())
}
