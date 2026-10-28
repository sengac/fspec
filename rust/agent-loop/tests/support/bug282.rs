use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, session::CodexSession,
    CodexResponsesModel,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::BackgroundSession;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

#[path = "../../../providers/tests/fixtures/prov148.rs"]
pub mod transport;
pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const MODEL: &str = "gpt-6.1-sol";

pub fn payload() -> Value {
    json!({"literal":r"\n\t\r", "quoted":"\"authentic α\"", "path":r"C:\new\tools\result", "lines":"first\nsecond\tthird\rfinal"})
}

pub fn string_payload() -> Value {
    json!(r#""canonical quoted α""#)
}

struct EffectTool {
    path: std::path::PathBuf,
    returned: Value,
}

impl Tool for EffectTool {
    const NAME: &'static str = "write_effect";
    type Args = Value;
    type Output = Value;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write a genuine isolated effect".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, _: Value) -> Result<Value, Self::Error> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.path)?;
        file.write_all(b"effect once")?;
        Ok(self.returned.clone())
    }
}

pub struct Output {
    native: BackgroundOutput,
    pub results: Mutex<Vec<String>>,
}

impl StreamOutput for Output {
    fn prepare_tool_dispatch(&self, id: &str) -> Result<(), String> {
        self.native.prepare_tool_dispatch(id)
    }

    fn emit(&self, event: StreamEvent) {
        if let StreamEvent::ToolResult(result) = &event {
            self.results
                .lock()
                .expect("fixture result capture lock")
                .push(result.content.clone());
        }
        self.native.emit(event);
    }
}

pub fn runtime() -> Result<Session, Box<dyn std::error::Error>> {
    let mut manager = ProviderManager::for_testing(ProviderType::Codex, Some(200_000), Some(4096));
    manager.set_model_direct("codex", MODEL, Some(200_000), Some(4096), None)?;
    Ok(Session::from_provider_manager(manager))
}

pub fn history(session: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension")?
        .history
        .clone())
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub background: Arc<BackgroundSession>,
    pub output: Output,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().into())?;
        reset_stores_for_tests();
        let mut manifest = create_session_with_provider(
            "BUG-282 canonical outcomes",
            directory.path(),
            &format!("codex/{MODEL}"),
        )?;
        append_message(&mut manifest, "user", "original task")?;
        let mut session = runtime()?;
        session
            .messages
            .push(rig::message::Message::user("original task"));
        session.bind_provider_persistence(manifest.id)?;
        let (input, _) = mpsc::channel(4);
        let background = Arc::new(BackgroundSession::new(
            manifest.id,
            "BUG-282".into(),
            directory.path().to_string_lossy().into(),
            Some("codex".into()),
            Some(MODEL.into()),
            session,
            input,
            None,
            None,
            None,
            broadcast::channel(64).0,
            broadcast::channel(64).0,
            broadcast::channel(32).0,
        ));
        let output = Output {
            native: BackgroundOutput::with_provider(background.clone(), "codex".into(), None),
            results: Mutex::new(Vec::new()),
        };
        Ok(Self {
            directory,
            background,
            output,
        })
    }

    pub async fn run(
        &self,
        cycle: usize,
        tool: bool,
    ) -> Result<transport::Server, Box<dyn std::error::Error>> {
        self.run_output(cycle, tool, payload()).await
    }

    pub async fn run_output(
        &self,
        cycle: usize,
        tool: bool,
        returned: Value,
    ) -> Result<transport::Server, Box<dyn std::error::Error>> {
        let call = json!({"type":"function_call","id":format!("item-{cycle}"),
            "call_id":format!("call-{cycle}"),"name":"write_effect",
            "arguments":json!({"work_unit_id":"NONE"}).to_string()});
        let completed = |output: Vec<Value>| {
            transport::event(json!({"type":"response.completed","response":{
            "id":format!("response-{cycle}"),"status":"completed","end_turn":true,
            "output":output,"usage":{"input_tokens":20,"output_tokens":5,"total_tokens":25}}}))
        };
        let answer = json!({"type":"message","id":format!("answer-{cycle}"),"role":"assistant",
            "content":[{"type":"output_text","text":format!("completed cycle {cycle}"),"annotations":[]}]});
        let bodies = if tool {
            vec![completed(vec![call]), completed(vec![answer])]
        } else {
            vec![completed(vec![answer])]
        };
        let server = transport::Server::start(bodies, false).await?;
        let _admission = codelet_tools::tool_admission::register_auxiliary(
            self.background.id,
            self.directory.path().into(),
        )?;
        let mut session = self.background.inner.lock().await;
        let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
            .api_key("synthetic-offline-key")
            .base_url(&server.url)
            .http_client(RefreshingCodexClient::new_api_key())
            .build()?;
        let model = CodexResponsesModel::new(client, MODEL)
            .with_history(history(&session)?)
            .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            });
        let agent = rig::agent::AgentBuilder::new(model)
            .tool(EffectTool {
                path: self.directory.path().join(format!("effect-{cycle}")),
                returned,
            })
            .build();
        let prompt = format!("cycle {cycle}");
        codelet_agent_loop::persist::persist_user_message(&self.background.id, &prompt)?;
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                &prompt,
                &mut session,
                self.background.is_interrupted.clone(),
                Arc::new(AtomicBool::new(false)),
                self.background.interrupt_notify.clone(),
                &self.output,
                self.background.id,
            ),
        )
        .await
        .map_err(|error| {
            format!(
                "{error}; requests={:?}; chunks={:?}",
                server.requests.lock(),
                self.background.get_buffered_output(30)
            )
        })??;
        Ok(server)
    }

    pub async fn cold(&self) -> TestResult {
        reset_stores_for_tests();
        let manifest = load_session(self.background.id)?;
        let envelopes = get_session_message_envelopes(manifest.id)?;
        let mut cold = runtime()?;
        cold.preflight_compaction_restore(&manifest, &envelopes)?;
        cold.restore_compaction_boundary(&manifest)?;
        cold.restore_provider_state(&envelopes)?;
        cold.messages = crate::replay_content::Replay::parse(&envelopes)?.messages;
        cold.bind_provider_persistence(manifest.id)?;
        cold.ensure_compaction_inference_ready()?;
        *self.background.inner.lock().await = cold;
        Ok(())
    }

    pub fn originals(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let manifest = load_session(self.background.id)?;
        let originals = manifest
            .messages
            .iter()
            .map(|reference| {
                get_message(reference.message_id)?.ok_or_else(|| "missing original".to_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(serde_json::to_value(originals)?)
    }
}
