use super::timing;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use wiremock::{Mock, ResponseTemplate};

pub const OPERATION_BUDGET: Duration = Duration::from_secs(3);

pub struct Budget;

impl Budget {
    pub fn install(duration: Duration) -> Self {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(duration));
        Self
    }
}

fn responses_stream(calls: Vec<Value>, tools: bool) -> ResponseTemplate {
    let output = if tools {
        calls
            .into_iter()
            .map(|call| {
                json!({"type":"function_call", "id":call["id"],
            "call_id":call["id"], "name":call["function"]["name"],
            "arguments":call["function"]["arguments"], "status":"completed"})
            })
            .collect::<Vec<_>>()
    } else {
        vec![
            json!({"type":"message", "id":"summary", "role":"assistant", "status":"completed",
        "content":[{"type":"output_text", "text":timing::DAG, "annotations":[]}]}),
        ]
    };
    let mut events = if tools {
        format!(
            "data: {}\n\n",
            json!({"type":"response.output_text.delta", "output_index":0,
            "sequence_number":0, "content_index":0, "delta":super::scaffold::ASSISTANT_TEXT})
        )
    } else {
        String::new()
    };
    events.push_str(
        &output
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let event = json!({"type":"response.output_item.done", "output_index":index,
            "sequence_number":index, "item":item});
                format!("data: {event}\n\n")
            })
            .collect::<String>(),
    );
    if !tools {
        let event = json!({"type":"response.output_text.delta", "output_index":0,
            "sequence_number":2, "content_index":0, "delta":timing::DAG});
        events.push_str(&format!("data: {event}\n\n"));
    }
    let event = json!({"type":"response.completed", "sequence_number":3, "response":{
        "id":"review", "object":"response", "created_at":1, "status":"completed", "model":"o3", "output":output,
        "usage":{"input_tokens":100,"output_tokens":20,"total_tokens":120}}});
    events.push_str(&format!("data: {event}\n\n"));
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(events)
}

pub async fn mount_stalled_model(harness: &timing::Harness, stall_synthesis: bool) {
    harness.server.reset().await;
    let target = harness.session.id;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("request JSON");
            let tools = timing::has_tools(&body);
            let gathered = body["messages"].as_array().or_else(|| body["input"].as_array())
                .expect("request history").iter().any(|message|
                    message["role"] == "tool" || message["type"] == "function_call_output");
            let message = if tools {
                json!({"role":"assistant", "tool_calls":[{"id":"all-originals","type":"function", "function":{
                    "name":"SessionSearch", "arguments":json!({"work_unit_id":"NONE", "action_type":"originals", "session_id":target,
                    "start_turn":0, "end_turn":1, "limit":2, "max_bytes":61440}).to_string()}}]})
            } else { json!({"role":"assistant", "content":timing::DAG}) };
            let template = ResponseTemplate::new(200).set_body_json(json!({
                "id":"elapsed", "object":"chat.completion", "created":1, "model":"o3",
                "choices":[{"index":0, "message":message,"finish_reason":if tools {"tool_calls"} else {"stop"}}],
                "usage":{"prompt_tokens":100, "completion_tokens":10, "total_tokens":110}
            }));
            if (tools && gathered) || (!tools && stall_synthesis) { template.set_delay(Duration::from_secs(5)) } else { template }
        }).mount(&harness.server).await;
}

impl Drop for Budget {
    fn drop(&mut self) {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
    }
}

pub async fn mount(harness: &timing::Harness, unfinished_batch: bool) {
    harness.server.reset().await;
    let target = harness.session.id;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("provider request");
            let tools = timing::has_tools(&body);
            let gathered = body["messages"].as_array().or_else(|| body["input"].as_array())
                .expect("request history").iter().any(|message|
                    message["role"] == "tool" || message["type"] == "function_call_output");
            let mut calls = vec![json!({"id":"originals-completed", "type":"function", "function":{
                "name":"SessionSearch", "arguments":json!({"work_unit_id":"NONE", "action_type":"originals",
                    "session_id":target,"start_turn":0,"end_turn":1,"limit":2,"max_bytes":61440}).to_string()}})];
            if unfinished_batch {
                calls.push(json!({"id":"command-unfinished","type":"function","function":{
                    "name":"Bash","arguments":json!({"work_unit_id":"NONE","command":"sleep 5"}).to_string()}}));
            }
            let template = if body["stream"] == true {
                responses_stream(calls, tools)
            } else {
                let message = if tools { json!({"role":"assistant","content":super::scaffold::ASSISTANT_TEXT,"tool_calls":calls}) }
                    else { json!({"role":"assistant","content":timing::DAG}) };
                ResponseTemplate::new(200).set_body_json(json!({"id":"review","object":"chat.completion","created":1,"model":"o3",
                    "choices":[{"index":0,"message":message,"finish_reason":if tools {"tool_calls"} else {"stop"}}],
                    "usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120}}))
            };
            if tools && gathered { template.set_delay(Duration::from_secs(5)) }
            else if !tools && !unfinished_batch { template.set_delay(Duration::from_millis(1200)) }
            else { template }
        }).mount(&harness.server).await;
}

struct SearchRegistration(uuid::Uuid);
impl Drop for SearchRegistration {
    fn drop(&mut self) {
        codelet_tools::set_session_search_handler(self.0, None);
    }
}

pub fn install_streaming(harness: &timing::Harness) {
    let endpoint = harness.server.uri();
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        harness.session.id,
        Some(Arc::new(move |target, source| {
            let endpoint = endpoint.clone();
            Box::pin(async move {
                use rig::client::CompletionClient;
                let evidence =
                    codelet_agent_loop::compaction_evidence::CompactionEvidence::capture(source)?;
                let project = codelet_core::persistence::load_session(target)?.project;
                codelet_tools::set_session_search_handler(
                    target,
                    Some(evidence.search_handler(project)),
                );
                let _search = SearchRegistration(target);
                let _processes = codelet_tools::unified_exec::ProcessOwnerScope::new(target);
                let client: rig::providers::openai::Client =
                    rig::providers::openai::Client::builder()
                        .api_key("offline-test")
                        .base_url(endpoint)
                        .build()
                        .map_err(|error| error.to_string())?;
                codelet_agent_loop::compaction_execution::run(
                    client
                        .agent("o3")
                        .max_tokens(4096)
                        .tool(codelet_tools::SessionSearchTool::new(target))
                        .tool(codelet_tools::BashTool::new(target))
                        .build(),
                    target,
                    "Summarize the captured original evidence",
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
}
