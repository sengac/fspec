use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};

pub struct Budget;

impl Budget {
    pub fn install() -> Self {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(
            Duration::from_secs(2),
        ));
        Self
    }
}

impl Drop for Budget {
    fn drop(&mut self) {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
    }
}
use codelet_providers::request_admission::AdmissionLimits;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::Notify;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Evidence\">Complete original evidence</dag-node>";
pub const LIMITS: AdmissionLimits = AdmissionLimits {
    context_capacity: Some(200_000),
    output_reservation: Some(4096),
};

#[derive(Clone, Default)]
pub struct Gate {
    pub started: Arc<Notify>,
    pub release: Arc<Notify>,
    pub finished: Arc<Notify>,
    pub executions: Arc<AtomicUsize>,
}

impl Tool for Gate {
    const NAME: &'static str = "SettlementGate";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Wait for a synchronized test boundary".into(),
            parameters: json!({"type":"object", "properties":{}}),
        }
    }

    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        self.release.notified().await;
        self.finished.notify_one();
        Ok("genuine gate result".into())
    }
}

pub fn terminal(output: Vec<Value>) -> ResponseTemplate {
    let event = json!({"type":"response.completed", "response":{
        "id":"settlement-response", "status":"completed", "object":"response", "created_at":1,
        "model":"selected-codex-model", "output":output,
        "usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}});
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {event}\n\n"))
}

pub fn text() -> Value {
    json!({"type":"message", "id":"summary", "role":"assistant", "status":"completed",
        "content":[{"type":"output_text", "text":DAG, "annotations":[]}]})
}

pub fn batch(target: uuid::Uuid, partial: bool, blocked: bool) -> Vec<Value> {
    let mut output = vec![
        json!({"type":"reasoning", "id":"native-reasoning",
        "summary":[], "encrypted_content":"preserve-native-ciphertext"}),
        json!({"type":"message", "id":"native-commentary", "role":"assistant", "status":"completed",
            "content":[{"type":"output_text", "text":"Inspecting the captured originals.", "annotations":[]}]}),
    ];
    if partial || !blocked {
        output.push(
            json!({"type":"function_call", "id":"original-item", "call_id":"original-call",
            "name":"SessionSearch", "arguments":json!({"work_unit_id":"NONE",
                "action_type":"originals", "session_id":target,
                "start_turn":0,"end_turn":1,"limit":2,"max_bytes":61440}).to_string()}),
        );
    }
    if blocked {
        for identity in ["blocked", "unstarted"] {
            output.push(
                json!({"type":"function_call", "id":format!("{identity}-item"),
                "call_id":format!("{identity}-call"), "name":Gate::NAME,
                "arguments":json!({"work_unit_id":"NONE"}).to_string()}),
            );
        }
    }
    output
}

pub async fn mount(
    server: &MockServer,
    target: uuid::Uuid,
    partial: bool,
    blocked: bool,
) -> Arc<Notify> {
    server.reset().await;
    let continued = Arc::new(Notify::new());
    let observed = continued.clone();
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().expect("actual Codex wire request");
            let tools = has_tools(&body);
            let completed = body["input"]
                .as_array()
                .expect("Codex input")
                .iter()
                .any(|item| item["type"] == "function_call_output");
            if !tools {
                terminal(vec![text()])
            } else if completed {
                observed.notify_one();
                terminal(vec![text()]).set_delay(Duration::from_secs(30))
            } else {
                terminal(batch(target, partial, blocked))
            }
        })
        .mount(server)
        .await;
    continued
}

pub fn has_tools(body: &Value) -> bool {
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}

pub fn model(
    server: &MockServer,
    history: CodexHistory,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("offline-settlement")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "selected-codex-model")
        .with_history(history)
        .with_request_admission(LIMITS)
        .with_streaming_completions())
}

pub async fn run(
    agent: rig::agent::Agent<CodexResponsesModel>,
    target: uuid::Uuid,
) -> Result<String, String> {
    let budget = Duration::from_secs(2);
    let deadline = tokio::time::Instant::now() + budget;
    codelet_cli::compaction_timing::scope(uuid::Uuid::new_v4(), deadline, budget, async {
        let inherited = codelet_cli::compaction_timing::deadline();
        let result = codelet_agent_loop::compaction_execution::run(
            agent,
            target,
            "Summarize originals",
            "codex",
            20,
            LIMITS,
        )
        .await;
        if codelet_cli::compaction_timing::deadline() != inherited {
            return Err("handoff reset the absolute deadline".into());
        }
        result
    })
    .await
}

pub fn originals(target: uuid::Uuid) -> Result<(), String> {
    let manifest = codelet_core::persistence::load_session(target)?;
    let source = codelet_core::persistence::compaction_boundary::capture_compaction_source(
        &manifest,
        "selected-codex-model",
        "full-current-dag",
    )
    .map_err(|error| error.to_string())?;
    let evidence = codelet_agent_loop::compaction_evidence::CompactionEvidence::capture(source)?;
    codelet_tools::set_session_search_handler(
        target,
        Some(evidence.search_handler(manifest.project)),
    );
    Ok(())
}

pub fn agent(
    model: CodexResponsesModel,
    target: uuid::Uuid,
    gate: Gate,
) -> rig::agent::Agent<CodexResponsesModel> {
    rig::agent::AgentBuilder::new(model)
        .preamble("Build a complete DAG")
        .max_tokens(4096)
        .tool(codelet_tools::SessionSearchTool::new(target))
        .tool(gate)
        .build()
}
