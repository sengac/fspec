//! Isolated durable compaction setup, using the real selected-request and commit APIs.
//! The synthetic model is used for request preparation only; no inference is sent.
use codelet_cli::{
    compaction_operation::CompactionFailure,
    compactor_sub_agent::*,
    interactive::{StreamEvent, StreamOutput},
    session::Session,
};
use codelet_core::{persistence::*, TokenState};
use codelet_providers::{
    request_admission::AdmissionLimits, selected_request::SelectedRequestPreparer, ProviderManager,
    ProviderType,
};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder, message::Message};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub struct Fixture {
    pub session: Session,
    pub id: uuid::Uuid,
    pub tokens: Arc<Mutex<TokenState>>,
    pub busy: Arc<AtomicBool>,
    pub directory: tempfile::TempDir,
    turns: usize,
}
impl Fixture {
    pub fn new(turns: usize) -> TestResult<Self> {
        if turns == 0 {
            return Err("fixture needs durable originals".into());
        }
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let mut manifest =
            create_session_with_provider("compactor contract", directory.path(), "openai")?;
        let mut messages = Vec::new();
        for turn in 0..turns {
            let user = format!("user turn {turn} do some work");
            let assistant = format!("assistant turn {turn} did the work");
            append_message(&mut manifest, "user", &user)?;
            append_message(&mut manifest, "assistant", &assistant)?;
            messages.extend([Message::user(user), Message::assistant(assistant)]);
        }
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
        manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
        let mut session = Session::from_provider_manager(manager);
        session.messages = messages;
        session.token_tracker.input_tokens = 190_000;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-synthetic")
                .build()?;
        let model = client.completion_model("o3");
        let template = CompletionRequestBuilder::new(model.clone(), "unused")
            .max_tokens(4096)
            .build();
        session.install_selected_preparer(SelectedRequestPreparer::new(
            model,
            template,
            AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            },
        ));
        Ok(Self {
            session,
            id: manifest.id,
            directory,
            turns,
            tokens: Arc::new(Mutex::new(TokenState {
                input_tokens: 190_000,
                compaction_needed: true,
                ..Default::default()
            })),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn dag(&self) -> String {
        format!("<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Overflow arc\">the session survived the overflow</dag-node>",self.turns*2-1)
    }
    pub fn handler(&self, handler: CompactorSubAgentHandler) {
        set_compactor_sub_agent_handler(self.id, Some(handler));
    }
    pub fn valid_builder(&self) {
        let dag = self.dag();
        self.handler(Arc::new(move |_, _| {
            let dag = dag.clone();
            Box::pin(async move { Ok(dag) })
        }));
    }
    pub async fn run(&mut self, output: &impl StreamOutput) -> Result<(), CompactionFailure> {
        run_compactor_sub_agent_round(&mut self.session, self.id, &self.tokens, &self.busy, output)
            .await
    }
    pub fn snapshot(&self) -> TestResult<serde_json::Value> {
        Ok(serde_json::to_value((
            &self.session.messages,
            load_session(self.id)?,
        ))?)
    }
    pub fn originals(&self) -> TestResult<Vec<uuid::Uuid>> {
        Ok(load_session(self.id)?
            .messages
            .iter()
            .map(|message| message.message_id)
            .collect())
    }
    pub fn assert_typed_summary(&self) -> TestResult {
        let envelopes = codelet_core::persistence::get_session_message_envelopes(self.id)?;
        let parsed = envelopes
            .iter()
            .map(|encoded| serde_json::from_str::<serde_json::Value>(encoded))
            .collect::<Result<Vec<_>, _>>()?;
        let summaries = parsed
            .iter()
            .filter(|envelope| envelope["_compactionSummary"] == true)
            .collect::<Vec<_>>();
        assert_eq!(
            summaries.len(),
            1,
            "exactly one structurally identified durable summary"
        );
        let summary = summaries[0];
        assert_eq!(summary["provider"], "compaction");
        let manifest = codelet_core::persistence::load_session(self.id)?;
        let boundary = manifest
            .compaction_commit
            .ok_or("committed boundary missing")?;
        assert_eq!(
            summary["_committedCompaction"]["generation"],
            boundary.identity.generation
        );
        assert_eq!(
            summary["_committedCompaction"]["operationId"],
            boundary.identity.operation_id.to_string()
        );
        let content = &summary["message"]["content"];
        let text = match content {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Array(blocks) => blocks
                .iter()
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            _ => return Err("durable summary content has invalid type".into()),
        };
        assert_eq!(
            text.matches(&self.dag()).count(),
            1,
            "exact durable content must be retained once"
        );
        Ok(())
    }
    pub fn assert_memory_preserved(&self, before: &serde_json::Value) -> TestResult {
        assert_eq!(&self.snapshot()?, before);
        assert_eq!(self.session.current_provider_name(), "openai");
        assert_eq!(self.session.current_model_id().as_deref(), Some("o3"));
        assert_eq!(self.session.token_tracker.input_tokens, 190_000);
        assert!(
            self.tokens
                .lock()
                .map_err(|error| error.to_string())?
                .compaction_needed
        );
        assert!(self.session.ensure_compaction_inference_ready().is_err());
        assert!(!self.busy.load(Ordering::Acquire));
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        set_compactor_sub_agent_handler(self.id, None);
    }
}

#[derive(Default)]
pub struct Output {
    pub events: Mutex<Vec<StreamEvent>>,
}
impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}
impl Output {
    pub fn count(&self, predicate: impl Fn(&StreamEvent) -> bool) -> usize {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|event| predicate(event))
            .count()
    }
}
