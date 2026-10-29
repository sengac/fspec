use codelet_agent_loop::BackgroundOutput;
use codelet_cli::{interactive::run_agent_stream, session::Session};
use codelet_core::{persistence::*, RigAgent};
use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, session::CodexSession,
    CodexResponsesModel,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const MODEL: &str = "gpt-6.1-sol";
pub const EFFECT: &str = r#"{"proof":"canonical \\n outcome α"}"#;

pub(super) struct EffectTool(pub(super) PathBuf);

impl Tool for EffectTool {
    const NAME: &'static str = "write_effect";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write an isolated once-only effect".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.0)?;
        file.write_all(b"executed once")?;
        Ok(EFFECT.into())
    }
}

pub struct Harness {
    pub directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub receiver: Option<mpsc::Receiver<PromptInput>>,
    pub requests: Option<crate::transport::Server>,
    worker: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.abort();
        }
    }
}

pub fn history(session: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("missing genuine Codex extension")?
        .history
        .clone())
}

impl Harness {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().into())?;
        reset_stores_for_tests();
        let manifest = create_session_with_provider(
            "BUG-284 admission",
            directory.path(),
            &format!("codex/{MODEL}"),
        )?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::Codex, Some(200_000), Some(4096));
        manager.set_model_direct("codex", MODEL, Some(200_000), Some(4096), None)?;
        let mut inner = Session::from_provider_manager(manager);
        inner.bind_provider_persistence(manifest.id)?;
        let (sender, receiver) = mpsc::channel(32);
        let session = Arc::new(BackgroundSession::new(
            manifest.id,
            "BUG-284".into(),
            directory.path().to_string_lossy().into(),
            Some("codex".into()),
            Some(MODEL.into()),
            inner,
            sender,
            None,
            None,
            None,
            broadcast::channel(128).0,
            broadcast::channel(64).0,
            broadcast::channel(32).0,
        ));
        Ok(Self {
            directory,
            session,
            receiver: Some(receiver),
            requests: None,
            worker: None,
        })
    }

    pub async fn seed(&mut self) -> TestResult {
        let completed = |output: Vec<Value>| {
            crate::transport::event(json!({"type":"response.completed","response":{
                "id":"seed-response","status":"completed","end_turn":true,"output":output,
                "usage":{"input_tokens":20,"output_tokens":5,"total_tokens":25}}}))
        };
        let server = crate::transport::Server::start(
            vec![
                completed(vec![json!({"type":"function_call","id":"effect-item",
                    "call_id":"effect-call","name":"write_effect",
                    "arguments":json!({"work_unit_id":"NONE"}).to_string()})]),
                completed(vec![
                    json!({"type":"message","id":"seed-answer","role":"assistant",
                    "content":[{"type":"output_text","text":"effect completed","annotations":[]}]}),
                ]),
            ],
            false,
        )
        .await?;
        let _admission = codelet_tools::tool_admission::register_auxiliary(
            self.session.id,
            self.directory.path().into(),
        )?;
        let mut inner = self.session.inner.lock().await;
        let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
            .api_key("synthetic-offline-key")
            .base_url(&server.url)
            .http_client(RefreshingCodexClient::new_api_key().with_test_endpoint(&server.url)?)
            .build()?;
        let model = CodexResponsesModel::new(client, MODEL)
            .with_history(history(&inner)?)
            .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            });
        let agent = rig::agent::AgentBuilder::new(model)
            .tool(EffectTool(self.directory.path().join("effect")))
            .build();
        codelet_agent_loop::persist::persist_user_message(&self.session.id, "genuine task")?;
        tokio::time::timeout(
            Duration::from_secs(15),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "genuine task",
                &mut inner,
                self.session.is_interrupted.clone(),
                self.session.compaction_in_progress.clone(),
                self.session.interrupt_notify.clone(),
                &BackgroundOutput::with_provider(self.session.clone(), "codex".into(), None),
                self.session.id,
            ),
        )
        .await??;
        let checkpoint = serde_json::to_value(history(&inner)?.snapshot()?)?;
        assert_eq!(
            checkpoint["items"]
                .as_array()
                .ok_or("items")?
                .iter()
                .filter(|item| item["type"] == "function_call_output")
                .count(),
            1
        );
        assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 2);
        self.requests = Some(server);
        Ok(())
    }

    pub fn start(&mut self) -> TestResult {
        let receiver = self.receiver.take().ok_or("loop already started")?;
        self.worker = Some(tokio::spawn(codelet_agent_loop::agent_loop(
            self.session.clone(),
            receiver,
            mpsc::channel(1).1,
        )));
        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.abort();
            let _ = worker.await;
        }
    }

    pub async fn compact(&self) -> TestResult {
        let mut inner = self.session.inner.lock().await;
        let envelopes = get_session_message_envelopes(self.session.id)?;
        let replay = crate::replay_content::Replay::parse(&envelopes)?;
        let same_history =
            codelet_cli::session::replay::same_replay_messages(&replay.messages, &inner.messages);
        let source = inner.capture_durable_compaction(self.session.id)?;
        let dag = format!(
            "<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Work\">Genuine task; effect completed once</dag-node>",
            source.prefix.len().checked_sub(1).ok_or("empty source")?
        );
        let checkpoint = history(&inner)?.snapshot()?;
        let before = serde_json::to_value(load_session(self.session.id)?)?;
        let committed =
            inner.commit_selected_compaction(source, uuid::Uuid::new_v4(), &dag, |_| Ok(()));
        if let Err(error) = &committed {
            assert_eq!(
                before,
                serde_json::to_value(load_session(self.session.id)?)?
            );
            assert_eq!(checkpoint, history(&inner)?.snapshot()?);
            assert!(
                error.to_string().contains("lineage mismatch"),
                "unexpected failure: {error}"
            );
        }
        assert!(
            committed.is_ok(),
            "actual selected compaction rejected retained canceled input: {committed:?}"
        );
        assert!(
            same_history,
            "selected compaction accepted a cold conversation different from the admitted runtime"
        );
        Ok(())
    }
}
