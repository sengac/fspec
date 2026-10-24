use codelet_agent_loop::generate_compaction_handler::register_compactor_sub_agent_handler;
use codelet_cli::compactor_sub_agent::{
    run_compactor_sub_agent_round, set_compactor_sub_agent_handler,
};
use codelet_cli::interactive::StreamOutput;
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::TokenState;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{CompactionResult, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const ORIGINAL: u64 = 166_750;
pub const COMPACTED: u64 = 90_336;
pub const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-195\" label=\"Work\">Preserved progress</dag-node>";

pub struct Fixture {
    pub session: Arc<BackgroundSession>,
    pub server: wiremock::MockServer,
    _directory: tempfile::TempDir,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let server = wiremock::MockServer::start().await;
        std::env::set_var("OPENAI_API_KEY", "offline-test");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id":"chatcmpl-metrics", "object":"chat.completion", "created":1,
                    "model":"o3", "usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120},
                    "choices":[{"index":0,"message":{
                        "role":"assistant", "content":DAG
                    },"finish_reason":"stop"}]
                })),
            )
            .mount(&server)
            .await;
        let mut manifest =
            create_session_with_provider("completion metrics", directory.path(), "openai")?;
        for position in 0..196 {
            let content = format!("original {position}");
            append_message_with_metadata(
                &mut manifest,
                "user",
                &content,
                [
                    ("uuid".into(), serde_json::json!(uuid::Uuid::new_v4())),
                    (
                        "message".into(),
                        serde_json::json!({"role":"user", "content":content}),
                    ),
                ]
                .into(),
            )?;
        }
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
        manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
        let mut inner = Session::from_provider_manager(manager);
        inner.token_tracker.input_tokens = ORIGINAL;
        let mut projection = manifest.clone();
        projection.compaction = Some(CompactionState {
            summary: DAG.into(),
            compacted_before_index: manifest.messages.len(),
            compacted_at: chrono::Utc::now(),
        });
        let mut messages = Vec::new();
        for envelope in envelopes_for_manifest(&projection)? {
            let envelope: serde_json::Value = serde_json::from_str(&envelope)?;
            let message: rig::providers::anthropic::completion::Message =
                serde_json::from_value(envelope["message"].clone())?;
            messages.push(rig::message::Message::try_from(message)?);
        }
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-test")
                .build()?;
        let model = client.completion_model("o3");
        let prepare = |padding| {
            SelectedRequestPreparer::new(
                model.clone(),
                CompletionRequestBuilder::new(model.clone(), "unused")
                    .preamble("x".repeat(padding))
                    .max_tokens(4096)
                    .build(),
                AdmissionLimits {
                    context_capacity: Some(200_000),
                    output_reservation: Some(4096),
                },
            )
        };
        let baseline = prepare(1).prepare(&messages, 1)?.input_tokens();
        let padding =
            usize::try_from(COMPACTED.checked_sub(baseline).ok_or("fixture budget")? + 1)?;
        let preparer = prepare(padding);
        assert_eq!(preparer.prepare(&messages, 1)?.input_tokens(), COMPACTED);
        inner.install_selected_preparer(preparer);
        let (input_tx, _) = mpsc::channel(4);
        let (chunks_tx, _) = broadcast::channel(64);
        let (status_tx, _) = broadcast::channel(16);
        let (cleared_tx, _) = broadcast::channel(4);
        let session = Arc::new(BackgroundSession::new(
            manifest.id,
            "completion metrics".into(),
            directory.path().to_string_lossy().into(),
            Some("openai".into()),
            Some("o3".into()),
            inner,
            input_tx,
            None,
            None,
            None,
            chunks_tx,
            status_tx,
            cleared_tx,
        ));
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )?;
        session.update_tokens(ORIGINAL as u32, 0);
        session.set_model_limits(200_000, 4096, 160_000);
        register_compactor_sub_agent_handler(
            session.id,
            &*session.inner.lock().await,
            directory.path().to_owned(),
            session.compaction_in_progress.clone(),
        );
        Ok(Self {
            session,
            server,
            _directory: directory,
            _admission: admission,
        })
    }

    pub async fn run(&self, output: &impl StreamOutput) -> TestResult {
        let mut inner = self.session.inner.lock().await;
        run_compactor_sub_agent_round(
            &mut inner,
            self.session.id,
            &Arc::new(Mutex::new(TokenState::default())),
            &self.session.compaction_in_progress,
            output,
        )
        .await?;
        Ok(())
    }

    pub fn chunks(&self) -> Vec<StreamChunk> {
        self.session.get_buffered_output(100)
    }

    pub fn completion(&self) -> Result<CompactionResult, Box<dyn std::error::Error>> {
        self.chunks()
            .into_iter()
            .find_map(|chunk| match chunk {
                StreamChunk::CompactionComplete {
                    compaction_result, ..
                } => Some(compaction_result),
                _ => None,
            })
            .ok_or_else(|| "successful builder must publish completion".into())
    }

    pub fn boundary(&self) -> Result<compaction_boundary::CommittedGeneration, String> {
        load_session(self.session.id)?
            .compaction_commit
            .ok_or_else(|| "automatic builder must commit a durable generation".into())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_compactor_sub_agent_handler(self.session.id, None);
        std::env::remove_var("OPENAI_BASE_URL");
        std::env::remove_var("OPENAI_API_KEY");
    }
}
