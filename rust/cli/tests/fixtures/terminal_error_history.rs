use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::{
    create_session_with_provider, get_session_message_envelopes, reset_stores_for_tests,
};
use codelet_core::RigAgent;
use codelet_providers::codex::history::CodexHistory;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use rig::completion::ToolDefinition;
use rig::message::{AssistantContent, Message, UserContent};
use rig::tool::Tool;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Notify;

#[path = "../../../providers/tests/fixtures/prov148.rs"]
pub mod transport;
pub use transport::TestResult;

#[derive(Default)]
pub struct Output(pub Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

impl Output {
    pub fn results(&self) -> Vec<(String, String, String)> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolResult(result) => Some((
                    result.id.clone(),
                    result.call_id.clone().unwrap_or_default(),
                    result.content.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    pub fn assert_failure(&self, code: &str) {
        let events = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(events
            .iter()
            .any(|event| matches!(event, StreamEvent::Error(error)
            if error.contains(code) && !error.contains("Removed the failed tool call"))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, StreamEvent::Done(_))));
    }
}

struct WriteTool(PathBuf);

impl Tool for WriteTool {
    const NAME: &'static str = "write_effect";
    type Args = Value;
    type Output = Value;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write a real side effect".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, args: Value) -> Result<Value, Self::Error> {
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.0)?,
            "effect"
        )?;
        if args["fail"] == true {
            return Err(std::io::Error::other("genuine tool failure after write"));
        }
        Ok(json!({"saved":true}))
    }
}

pub fn call(number: usize, fail: bool) -> Value {
    json!({"type":"function_call", "id":format!("item-{number}"),
        "call_id":format!("call-{number}"), "name":"write_effect",
        "arguments":json!({"fail":fail}).to_string()})
}

pub fn completed(calls: Vec<Value>) -> String {
    transport::event(json!({"type":"response.completed","response":{
        "id":"tool-response", "status":"completed", "end_turn":true,
        "output":calls, "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

pub fn rejection(code: &str) -> String {
    transport::event(json!({"type":"error","code":code,
        "message":"Provider rejected inference", "type_detail":"service_unavailable_error"}))
}

pub fn new_session() -> Session {
    Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Codex,
        Some(272_000),
        Some(4096),
    ))
}

pub fn history(session: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension missing")?
        .history
        .clone())
}

pub struct Harness {
    pub directory: tempfile::TempDir,
    pub session: Session,
    pub server: transport::Server,
    pub id: uuid::Uuid,
}

impl Harness {
    pub async fn new(bodies: Vec<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        reset_stores_for_tests();
        let manifest = create_session_with_provider("terminal error", directory.path(), "codex")?;
        let mut session = new_session();
        session.bind_provider_persistence(manifest.id)?;
        Ok(Self {
            directory,
            session,
            server: transport::Server::start(bodies, false).await?,
            id: manifest.id,
        })
    }

    pub async fn run(&mut self, prompt: &str, output: &Output) -> anyhow::Result<()> {
        let model = self
            .server
            .models()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
            .0
            .with_history(
                history(&self.session).map_err(|error| anyhow::anyhow!(error.to_string()))?,
            );
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble("Test instructions")
            .tool(WriteTool(self.directory.path().join("effects")))
            .build();
        tokio::time::timeout(
            Duration::from_secs(10),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                prompt,
                &mut self.session,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
                Arc::new(Notify::new()),
                output,
                self.id,
            ),
        )
        .await?
    }

    pub fn cold_reload(&mut self) -> TestResult {
        let messages_path = self.directory.path().join("conversation.json");
        std::fs::write(&messages_path, serde_json::to_vec(&self.session.messages)?)?;
        let expected = history(&self.session)?.snapshot()?;
        self.session = new_session();
        reset_stores_for_tests();
        self.session
            .restore_provider_state(&get_session_message_envelopes(self.id)?)?;
        self.session.messages = serde_json::from_slice(&std::fs::read(messages_path)?)?;
        self.session.bind_provider_persistence(self.id)?;
        assert_eq!(history(&self.session)?.snapshot()?, expected);
        Ok(())
    }

    pub fn assert_retained(&self, expected: &[(String, String, String)]) -> TestResult {
        let mut calls = Vec::new();
        let mut results = Vec::new();
        for message in &self.session.messages {
            match message {
                Message::Assistant { content, .. } => {
                    for item in content.iter() {
                        if let AssistantContent::ToolCall(call) = item {
                            calls.push((call.id.clone(), call.call_id.clone().unwrap_or_default()));
                        }
                    }
                }
                Message::User { content } => {
                    for item in content.iter() {
                        if let UserContent::ToolResult(result) = item {
                            results.push((
                                result.id.clone(),
                                result.call_id.clone().unwrap_or_default(),
                            ));
                        }
                    }
                }
            }
        }
        let identities: Vec<_> = expected
            .iter()
            .map(|(item, call, _)| (item.clone(), call.clone()))
            .collect();
        assert_eq!(
            calls, identities,
            "terminal errors must retain committed calls"
        );
        assert_eq!(
            results, identities,
            "terminal errors must retain genuine outcomes"
        );
        assert_eq!(
            std::fs::read_to_string(self.directory.path().join("effects"))?,
            "effect\n".repeat(expected.len())
        );
        Ok(())
    }

    pub async fn assert_continuation(
        &mut self,
        expected: &[(String, String, String)],
    ) -> TestResult {
        let output = Output::default();
        self.run("continue", &output).await?;
        self.assert_retained(expected)?;
        let requests = self
            .server
            .requests
            .lock()
            .map_err(|_| "requests poisoned")?;
        let input = requests.last().ok_or("no continuation")?["input"]
            .as_array()
            .ok_or("input missing")?;
        for (item_id, call_id, text) in expected {
            let calls: Vec<_> = input
                .iter()
                .filter(|item| item["type"] == "function_call" && item["call_id"] == *call_id)
                .collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["id"], *item_id);
            let results: Vec<_> = input
                .iter()
                .filter(|item| {
                    item["type"] == "function_call_output" && item["call_id"] == *call_id
                })
                .collect();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["output"], *text);
        }
        assert!(output
            .0
            .lock()
            .map_err(|_| "output poisoned")?
            .iter()
            .any(|event| matches!(event, StreamEvent::Done(_))));
        Ok(())
    }
}
