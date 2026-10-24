use super::{review, timing};
use rig::client::CompletionClient;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use wiremock::{Mock, ResponseTemplate};

pub const ASSISTANT_TEXT: &str = "Read the authoritative originals before summarizing";
const REASONING: &str = "Keep exact evidence and call identities";
const SIGNATURE: &str = "authentic-thinking-signature";

fn stream(target: uuid::Uuid, tools: bool) -> ResponseTemplate {
    let mut events = vec![json!({"type":"message_start", "message":{
        "id":"scaffold-response", "type":"message", "role":"assistant", "model":"claude-test",
        "content":[], "stop_reason":null, "stop_sequence":null,
        "usage":{"input_tokens":100,"output_tokens":0}}})];
    if tools {
        events.extend([
            json!({"type":"content_block_start", "index":0, "content_block":{
                "type":"thinking", "thinking":"", "signature":""}}),
            json!({"type":"content_block_delta", "index":0, "delta":{
                "type":"thinking_delta", "thinking":REASONING}}),
            json!({"type":"content_block_delta", "index":0, "delta":{
                "type":"signature_delta", "signature":SIGNATURE}}),
            json!({"type":"content_block_stop", "index":0}),
        ]);
    }
    events.extend([
        json!({"type":"content_block_start", "index":1, "content_block":{"type":"text", "text":""}}),
        json!({"type":"content_block_delta", "index":1, "delta":{
            "type":"text_delta", "text":if tools {ASSISTANT_TEXT} else {timing::DAG}}}),
        json!({"type":"content_block_stop", "index":1}),
    ]);
    if tools {
        for (index, identity, name, arguments) in [
            (
                2,
                "originals-completed",
                "SessionSearch",
                json!({"work_unit_id":"NONE",
                "action_type":"originals", "session_id":target, "start_turn":0, "end_turn":1,
                "limit":2, "max_bytes":61440}),
            ),
            (
                3,
                "command-unfinished",
                "Bash",
                json!({"work_unit_id":"NONE", "command":"sleep 5"}),
            ),
        ] {
            events.extend([
                json!({"type":"content_block_start", "index":index, "content_block":{
                    "type":"tool_use", "id":identity, "name":name, "input":{}}}),
                json!({"type":"content_block_delta", "index":index, "delta":{
                    "type":"input_json_delta", "partial_json":arguments.to_string()}}),
                json!({"type":"content_block_stop", "index":index}),
            ]);
        }
    }
    events.extend([
        json!({"type":"message_delta", "delta":{"stop_reason":if tools {"tool_use"} else {"end_turn"},
            "stop_sequence":null}, "usage":{"output_tokens":20}}),
        json!({"type":"message_stop"}),
    ]);
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(
            events
                .iter()
                .map(|event| format!("data: {event}\n\n"))
                .collect::<String>(),
        )
}

pub async fn verify_signed_reasoning() -> timing::TestResult {
    let harness = timing::Harness::new().await?;
    let _budget = review::Budget::install(review::OPERATION_BUDGET);
    let target = harness.session.id;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("Anthropic request");
            stream(target, timing::has_tools(&body))
        })
        .mount(&harness.server)
        .await;
    let endpoint = harness.server.uri();
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        target,
        Some(Arc::new(move |target, source| {
            let endpoint = endpoint.clone();
            Box::pin(async move {
                let evidence =
                    codelet_agent_loop::compaction_evidence::CompactionEvidence::capture(source)?;
                let project = codelet_core::persistence::load_session(target)?.project;
                codelet_tools::set_session_search_handler(
                    target,
                    Some(evidence.search_handler(project)),
                );
                let _processes = codelet_tools::unified_exec::ProcessOwnerScope::new(target);
                let client: rig::providers::anthropic::Client =
                    rig::providers::anthropic::Client::builder()
                        .api_key("offline-test")
                        .base_url(endpoint)
                        .build()
                        .map_err(|error| error.to_string())?;
                codelet_agent_loop::compaction_execution::run(
                    client
                        .agent("claude-test")
                        .max_tokens(4096)
                        .tool(codelet_tools::SessionSearchTool::new(target))
                        .tool(codelet_tools::BashTool::new(target))
                        .build(),
                    target,
                    "Summarize originals",
                    "zai",
                    20,
                    codelet_providers::request_admission::AdmissionLimits {
                        context_capacity: Some(200_000),
                        output_reservation: Some(4096),
                    },
                )
                .await
            })
        })),
    );
    let result = tokio::time::timeout(Duration::from_secs(5), harness.call(false)).await?;
    codelet_tools::set_session_search_handler(target, None);
    result?;
    let bodies = harness.bodies().await;
    let synthesis = bodies
        .iter()
        .find(|body| !timing::has_tools(body))
        .ok_or("synthesis")?;
    let messages = synthesis["messages"].as_array().ok_or("messages")?;
    let assistant = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .ok_or("assistant scaffold")?;
    let content = assistant["content"].as_array().ok_or("assistant content")?;
    assert_eq!(content[0]["type"], "thinking");
    assert_eq!(content[0]["thinking"], REASONING);
    assert_eq!(content[0]["signature"], SIGNATURE);
    assert_eq!(content[1]["text"], ASSISTANT_TEXT);
    assert_eq!(content[2]["id"], "originals-completed");
    assert_eq!(content.len(), 3);
    assert!(!synthesis.to_string().contains("command-unfinished"));
    for original in ["committed evidence", "committed response"] {
        assert!(synthesis.to_string().contains(original));
    }
    assert!(codelet_core::persistence::load_session(target)?
        .compaction_commit
        .is_some());
    Ok(())
}
