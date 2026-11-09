//! Feature: spec/features/chunked-compactor-synthesis.feature

use codelet_agent_loop::generate_compaction_handler::execute_compaction_subagent;
use codelet_core::persistence::{
    append_message_with_metadata, create_session_with_provider, reset_stores_for_tests,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Both turns</dag-node>";
const INSTRUCTION: &str = "Build a hierarchical summary DAG";

fn completion(message: Value, finish_reason: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id":"cmpct087", "object":"chat.completion", "created":1, "model":"local-reasoner",
        "choices":[{"index":0,"message":message,"finish_reason":finish_reason}],
        "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
    }))
}

fn has_tools(body: &Value) -> bool {
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}

/// Tool-enabled requests always ask for SessionSearch; tool-free synthesis
/// requests first return an invalid candidate, then a valid DAG.
async fn mount(server: &MockServer) {
    let synthesis = Arc::new(AtomicUsize::new(0));
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("compactor JSON");
            if has_tools(&body) {
                return completion(
                    json!({"role":"assistant","content":null,"tool_calls":[{
                        "id":"evidence","type":"function","function":{"name":"SessionSearch",
                        "arguments":json!({"work_unit_id":"NONE","action_type":"recent","count":1}).to_string()}}]}),
                    "tool_calls",
                );
            }
            let content = if synthesis.fetch_add(1, Ordering::SeqCst) == 0 {
                "Summary: the session edited two files."
            } else {
                DAG
            };
            completion(json!({"role":"assistant","content":content}), "stop")
        })
        .mount(server)
        .await;
}

async fn compact(server: &MockServer) -> Result<String, Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("cmpct087", directory.path(), "openai")?;
    for turn in ["edit the files", "done"] {
        append_message_with_metadata(
            &mut manifest,
            "user",
            turn,
            [
                ("uuid".into(), json!(uuid::Uuid::new_v4())),
                ("message".into(), json!({"role": "user", "content": turn})),
            ]
            .into(),
        )?;
    }
    std::env::set_var("OPENAI_API_KEY", "offline-cmpct087");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    let result = execute_compaction_subagent(
        directory.path(),
        manifest.id,
        None,
        &Arc::new(AtomicBool::new(true)),
        "openai",
        Some("local-reasoner"),
        Some(200_000),
        Some(4096),
    )
    .await;
    std::env::remove_var("OPENAI_BASE_URL");
    Ok(result?)
}

async fn bodies(server: &MockServer) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(server
        .received_requests()
        .await
        .ok_or("requests")?
        .iter()
        .map(|request| request.body_json::<Value>())
        .collect::<Result<Vec<_>, _>>()?)
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn a_correction_attempt_re_synthesizes_without_repeating_exploration() -> TestResult {
    // @step Given a compactor whose first candidate fails DAG validation after exploring the session
    let server = MockServer::start().await;
    mount(&server).await;

    // @step When the correction attempt runs
    let candidate = compact(&server).await?;

    // @step Then the correction sends at most one exploration request before synthesis
    let requests = bodies(&server).await?;
    let first_synthesis = requests
        .iter()
        .position(|body| !has_tools(body))
        .ok_or("first synthesis")?;
    let first_attempt_exploration = requests[..first_synthesis].len();
    let correction_exploration = requests[first_synthesis + 1..]
        .iter()
        .take_while(|body| has_tools(body))
        .count();
    assert!(
        first_attempt_exploration > 1,
        "attempt 1 explores: {first_attempt_exploration}"
    );
    assert!(
        correction_exploration <= 1,
        "correction must not repeat exploration: {correction_exploration} tool requests"
    );

    // @step And the corrected DAG is returned
    assert!(candidate.contains(DAG), "{candidate}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn the_compaction_instruction_is_sent_once() -> TestResult {
    // @step Given a compactor run for a persisted session
    let server = MockServer::start().await;
    mount(&server).await;
    compact(&server).await?;

    // @step When the first compactor request is sent
    let first = bodies(&server)
        .await?
        .into_iter()
        .next()
        .ok_or("first request")?;
    let messages = first["messages"].as_array().cloned().unwrap_or_default();

    // @step Then the system preamble does not repeat the compaction instruction
    let system: String = messages
        .iter()
        .filter(|message| message["role"] == "system")
        .map(|message| message["content"].to_string())
        .collect();
    assert!(!system.contains(INSTRUCTION), "{system}");

    // @step And the user task carries the compaction instruction
    let user: String = messages
        .iter()
        .filter(|message| message["role"] == "user")
        .map(|message| message["content"].to_string())
        .collect();
    assert_eq!(user.matches(INSTRUCTION).count(), 1, "{user}");
    Ok(())
}
