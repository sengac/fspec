#[path = "terminal_recovery.rs"]
mod terminal;

use codelet_agent_loop::generate_compaction_handler::{
    register_compactor_sub_agent_handler, register_generate_compaction_handler,
};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder, tool::Tool};
use std::sync::{Arc, Mutex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Evidence\">Complete original evidence</dag-node>";

pub struct Harness {
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    _directory: tempfile::TempDir,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Harness {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (directory, session, _) = terminal::fixture();
        let mut manifest = codelet_core::persistence::load_session(session.id)?;
        manifest.provider = "openai/o3".into();
        manifest.messages.clear();
        codelet_core::persistence::save_session(&manifest)?;
        for (role, content) in [
            ("user", "committed evidence"),
            ("assistant", "committed response"),
        ] {
            codelet_core::persistence::append_message_with_metadata(
                &mut manifest,
                role,
                content,
                [
                    ("uuid".into(), serde_json::json!(uuid::Uuid::new_v4())),
                    (
                        "message".into(),
                        serde_json::json!({"role":role,"content":content}),
                    ),
                ]
                .into(),
            )?;
        }
        let server = MockServer::start().await;
        std::env::set_var("OPENAI_API_KEY", "offline-test");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )?;
        {
            let mut inner = session.inner.lock().await;
            inner.provider_manager_mut().set_facade_override(None);
            inner.provider_manager_mut().set_model_direct(
                "openai",
                "o3",
                Some(200_000),
                Some(4096),
                None,
            )?;
            let client: rig::providers::openai::CompletionsClient =
                rig::providers::openai::CompletionsClient::new("offline-test")?;
            let model = client.completion_model("o3");
            inner.install_selected_preparer(
                codelet_providers::selected_request::SelectedRequestPreparer::new(
                    model.clone(),
                    CompletionRequestBuilder::new(model, "unused")
                        .max_tokens(4096)
                        .build(),
                    codelet_providers::request_admission::AdmissionLimits {
                        context_capacity: Some(200_000),
                        output_reservation: Some(4096),
                    },
                ),
            );
            register_compactor_sub_agent_handler(
                session.id,
                &inner,
                directory.path().into(),
                session.compaction_in_progress.clone(),
            );
        }
        Ok(Self {
            session,
            server,
            _directory: directory,
            _admission: admission,
        })
    }

    pub async fn call(&self, generated: bool) -> Result<(), String> {
        if generated {
            let queue = {
                let inner = self.session.inner.lock().await;
                register_generate_compaction_handler(
                    self.session.id,
                    &inner,
                    self._directory.path().into(),
                    self.session.clone(),
                    None,
                )
            };
            GenerateCompactionTool::new(self.session.id)
                .call(GenerateCompactionArgs { session_id: None })
                .await
                .map_err(|error| error.to_string())?;
            queue.apply(&mut *self.session.inner.lock().await, &self.session)?;
            Ok(())
        } else {
            let output = codelet_agent_loop::BackgroundOutput::with_provider(
                self.session.clone(),
                "openai".into(),
                None,
            );
            codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
                &mut *self.session.inner.lock().await,
                self.session.id,
                &Arc::new(Mutex::new(codelet_core::TokenState::default())),
                &self.session.compaction_in_progress,
                &output,
                &codelet_cli::compaction_cancellation::CompactionCancellation::new(
                    self.session.is_interrupted.clone(),
                    Some(self.session.interrupt_notify.clone()),
                ),
            )
            .await
            .map_err(|error| error.to_string())
        }
    }

    pub async fn prior_generation(&self) -> TestResult {
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id":"prior", "object":"chat.completion", "created":1, "model":"o3",
                "choices":[{"index":0,"message":{"role":"assistant","content":DAG},"finish_reason":"stop"}],
                "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
            }))).mount(&self.server).await;
        self.call(false).await?;
        Ok(())
    }

    pub async fn bodies(&self) -> Vec<serde_json::Value> {
        self.server
            .received_requests()
            .await
            .expect("requests")
            .iter()
            .map(|request| request.body_json().expect("JSON"))
            .collect()
    }

    pub fn chunks(&self) -> Vec<StreamChunk> {
        self.session.get_buffered_output(100)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.session.id, None);
        codelet_tools::set_generate_compaction_handler(self.session.id, None);
        codelet_tools::set_inject_summary_handler(self.session.id, None);
    }
}

pub fn has_tools(body: &serde_json::Value) -> bool {
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}
