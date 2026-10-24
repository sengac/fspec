use codelet_agent_loop::generate_compaction_handler::execute_compaction_subagent;
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn exploration(server: &MockServer, target: uuid::Uuid, summary: &str, streaming: bool) {
    server.reset().await;
    let summary = summary.to_owned();
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: serde_json::Value = request.body_json().expect("builder request");
            let tools = body["tools"]
                .as_array()
                .is_some_and(|tools| !tools.is_empty());
            let previous = body["messages"]
                .as_array()
                .expect("builder messages")
                .iter()
                .filter(|message| message["role"] == "tool")
                .count();
            let arguments = json!({
                "work_unit_id":"NONE",
                "action_type":"originals", "session_id":target,
                "start_turn":previous, "end_turn":previous, "limit":1,
                "max_bytes":61440
            })
            .to_string();
            let function = json!({"name":"SessionSearch", "arguments":arguments});
            if streaming {
                let delta = if tools {
                    json!({"role":"assistant", "tool_calls":[{
                        "index":0, "id":format!("evidence-{previous}"),
                        "type":"function", "function":function
                    }]})
                } else {
                    json!({"role":"assistant", "content":summary})
                };
                let chunk = json!({
                    "id":"stream-budget", "object":"chat.completion.chunk", "created":1,
                    "model":"o3", "choices":[{"index":0, "delta":delta,
                        "finish_reason":if tools {"tool_calls"} else {"stop"}}],
                    "usage":{"prompt_tokens":100, "completion_tokens":10, "total_tokens":110}
                });
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"));
            }
            let message = if tools {
                json!({"role":"assistant", "content":null, "tool_calls":[{
                    "id":format!("evidence-{previous}"), "type":"function", "function":function
                }]})
            } else {
                json!({"role":"assistant", "content":summary})
            };
            ResponseTemplate::new(200).set_body_json(json!({
                "id":"budget", "object":"chat.completion", "created":1, "model":"o3",
                "choices":[{"index":0, "message":message,
                    "finish_reason":if tools {"tool_calls"} else {"stop"}}],
                "usage":{"prompt_tokens":100, "completion_tokens":10, "total_tokens":110}
            }))
        })
        .mount(server)
        .await;
}

pub struct StreamingBuilder {
    pub agent: rig::agent::Agent<codelet_providers::codex::CodexResponsesModel>,
    registration: uuid::Uuid,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for StreamingBuilder {
    fn drop(&mut self) {
        codelet_tools::set_session_search_handler(self.registration, None);
    }
}

pub async fn codex_exploration(server: &MockServer, target: uuid::Uuid) -> StreamingBuilder {
    use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
    server.reset().await;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: serde_json::Value = request.body_json().expect("streaming request");
            let tools = body["tools"].as_array().is_some_and(|tools| !tools.is_empty());
            let position = body["input"].as_array().expect("input").iter()
                .filter(|item| item["type"] == "function_call_output").count();
            let item = if tools {
                json!({"type":"function_call", "id":format!("item-{position}"),
                    "call_id":format!("evidence-{position}"), "name":"SessionSearch",
                    "arguments":json!({"work_unit_id":"NONE",
                        "action_type":"originals", "session_id":target,
                        "start_turn":position, "end_turn":position, "limit":1}).to_string()})
            } else {
                json!({"type":"message", "id":"summary", "role":"assistant", "status":"completed",
                    "content":[{"type":"output_text", "text":super::metrics::DAG, "annotations":[]}]})
            };
            let output = if tools {
                format!("data: {}\n\n", json!({"type":"response.output_item.done", "output_index":0, "item":item}))
            } else {
                format!("data: {}\n\n", json!({"type":"response.output_text.delta",
                    "item_id":"summary", "output_index":0, "content_index":0,
                    "delta":super::metrics::DAG}))
            };
            let completed = json!({"type":"response.completed", "response":{
                "id":format!("response-{position}"), "object":"response", "status":"completed",
                "created_at":1, "model":"selected-codex-model", "output":[item],
                "usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}});
            ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
                .set_body_string(format!("{output}data: {completed}\n\n"))
        }).mount(server).await;
    let manifest = codelet_core::persistence::load_session(target).expect("manifest");
    let source = codelet_core::persistence::compaction_boundary::capture_compaction_source(
        &manifest,
        "selected-codex-model",
        "full-current-dag",
    )
    .expect("source");
    let evidence = codelet_agent_loop::compaction_evidence::CompactionEvidence::capture(source)
        .expect("captured originals");
    let registration = manifest.id;
    let admission =
        codelet_tools::tool_admission::register_auxiliary(registration, manifest.project.clone())
            .expect("trusted standalone compactor context");
    codelet_tools::set_session_search_handler(
        registration,
        Some(evidence.search_handler(manifest.project)),
    );
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("offline-test")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()
        .expect("Codex client");
    let model = CodexResponsesModel::new(client, "selected-codex-model")
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        })
        .with_streaming_completions();
    StreamingBuilder {
        agent: rig::agent::AgentBuilder::new(model)
            .preamble("Build a complete DAG")
            .tool(codelet_tools::SessionSearchTool::new(registration))
            .build(),
        registration,
        _admission: admission,
    }
}

pub async fn build(target: uuid::Uuid) -> Result<String, String> {
    execute_compaction_subagent(
        &std::env::temp_dir(),
        target,
        None,
        &Arc::new(AtomicBool::new(true)),
        "openai",
        Some("o3"),
        Some(200_000),
        Some(4096),
    )
    .await
}

pub fn has_tools(request: &wiremock::Request) -> bool {
    let body: serde_json::Value = request.body_json().expect("request JSON");
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}
