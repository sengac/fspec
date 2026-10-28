// Feature: spec/features/whole-request-admission-before-dag-commit.feature
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::Context;
use codelet_cli::compactor_sub_agent::{
    run_compactor_sub_agent_round, set_compactor_sub_agent_handler,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::system_reminders::add_system_reminder;
use codelet_cli::session::{Session, SystemReminderType};
use codelet_core::TokenState;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use rig::client::CompletionClient;
use rig::message::Message;
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">\nVerified work.\n</dag-node>\n<dag-files>\n</dag-files>";

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

impl RecordingOutput {
    fn completions(&self) -> usize {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|event| matches!(event, StreamEvent::CompactionComplete { .. }))
            .count()
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    session: Session,
    session_id: Uuid,
    token_state: Arc<Mutex<TokenState>>,
    in_progress: Arc<AtomicBool>,
    output: RecordingOutput,
}

impl Fixture {
    async fn new(capacity: usize, reservation: usize, instructions: &str) -> anyhow::Result<Self> {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())
            .map_err(anyhow::Error::msg)?;
        codelet_core::persistence::reset_stores_for_tests();
        let mut manifest = codelet_core::persistence::create_session_with_provider(
            "request admission",
            directory.path(),
            "openai",
        )
        .map_err(anyhow::Error::msg)?;
        codelet_core::persistence::append_message(
            &mut manifest,
            "user",
            "original immutable evidence",
        )
        .map_err(anyhow::Error::msg)?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(capacity), Some(reservation));
        manager.set_model_direct(
            "openai",
            "selected-model",
            Some(capacity),
            Some(reservation),
            None,
        )?;
        let mut session = Session::from_provider_manager(manager);
        let server = MockServer::start().await;
        let chunk = serde_json::json!({"id":"selected","object":"chat.completion.chunk","created":1,"model":"selected-model","choices":[{"index":0,"delta":{"role":"assistant","content":"ready"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
            )
            .mount(&server)
            .await;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("test-selected-key")
                .base_url(server.uri())
                .build()?;
        let model = client.completion_model("selected-model");
        let agent = codelet_core::RigAgent::with_default_depth(
            rig::agent::AgentBuilder::new(model.clone())
                .preamble("actual selected provider instructions")
                .max_tokens(reservation as u64)
                .build(),
        );
        if capacity == 0 || reservation == 0 {
            let template = rig::completion::CompletionRequestBuilder::new(
                model.clone(),
                "current selected prompt",
            )
            .preamble("actual selected provider instructions".into())
            .max_tokens(reservation as u64)
            .build();
            let preparer = codelet_providers::selected_request::SelectedRequestPreparer::new(
                model,
                template,
                codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: (capacity > 0).then_some(capacity as u64),
                    output_reservation: (reservation > 0).then_some(reservation as u64),
                },
            );
            session.install_selected_preparer(preparer);
        } else {
            codelet_cli::interactive::run_agent_stream(
                agent,
                "current selected prompt",
                &mut session,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
                Arc::new(tokio::sync::Notify::new()),
                &RecordingOutput::default(),
                manifest.id,
            )
            .await?;
        }
        session.messages = add_system_reminder(&[], SystemReminderType::SystemPrompt, instructions);
        session
            .messages
            .push(Message::user("original immutable evidence"));
        session.token_tracker.input_tokens = 99_999;
        Ok(Self {
            _directory: directory,
            session,
            session_id: manifest.id,
            token_state: Arc::new(Mutex::new(TokenState {
                input_tokens: 99_999,
                compaction_needed: true,
                ..TokenState::default()
            })),
            in_progress: Arc::new(AtomicBool::new(false)),
            output: RecordingOutput::default(),
        })
    }

    fn select_builder(&self) {
        set_compactor_sub_agent_handler(
            self.session_id,
            Some(Arc::new(|_, _| Box::pin(async { Ok(DAG.to_owned()) }))),
        );
    }

    async fn run(
        &mut self,
    ) -> anyhow::Result<Result<(), codelet_cli::compaction_operation::CompactionFailure>> {
        Ok(tokio::time::timeout(
            Duration::from_secs(2),
            run_compactor_sub_agent_round(
                &mut self.session,
                self.session_id,
                &self.token_state,
                &self.in_progress,
                &self.output,
            ),
        )
        .await?)
    }

    fn assert_preserved(&self, original: &[u8]) -> anyhow::Result<()> {
        assert_eq!(serde_json::to_vec(&self.session.messages)?, original);
        assert_eq!(self.session.token_tracker.input_tokens, 99_999);
        let state = self
            .token_state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(state.input_tokens, 99_999);
        assert!(state.compaction_needed, "ordinary inference stays blocked");
        assert!(!self.in_progress.load(Ordering::SeqCst));
        assert_eq!(self.output.completions(), 0);
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_compactor_sub_agent_handler(self.session_id, None);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn mandatory_fixed_context_cannot_be_shrunk_into_a_successful_commit() -> anyhow::Result<()> {
    // @step Given mandatory instructions larger than an explicit provider capacity
    let mut fixture = Fixture::new(1_024, 256, &"mandatory instruction ".repeat(2_000)).await?;
    let temporary = setup_temp_directory()?;
    let evidence_path = temporary.path().join("original.json");
    let original = serde_json::to_vec(&fixture.session.messages)?;
    std::fs::write(&evidence_path, &original)?;
    // @step And a valid small DAG returned by the selected builder
    fixture.select_builder();
    // @step When the compaction round attempts to commit the candidate
    let result = fixture.run().await?;
    // @step Then the round terminates boundedly with a blocked admission failure
    let failure = result
        .err()
        .context("fixed context cannot be shrunk by replacing the DAG")?;
    assert!(failure.to_string().contains("admission"), "{failure}");
    // @step And the previous memory and token state remain unchanged without completion
    fixture.assert_preserved(&original)?;
    assert_eq!(std::fs::read(evidence_path)?, original);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn output_reservation_participates_in_candidate_admission() -> anyhow::Result<()> {
    // @step Given mandatory instructions below capacity but above the usable input budget
    let instructions = "mandatory instruction ".repeat(80);
    let mut fixture = Fixture::new(1_024, 1_000, &instructions).await?;
    let original = serde_json::to_vec(&fixture.session.messages)?;
    // @step And a valid small DAG returned by the selected builder
    fixture.select_builder();
    // @step When the compaction round attempts to commit the candidate
    let result = fixture.run().await?;
    // @step Then the round terminates boundedly with a blocked admission failure
    let failure = result
        .err()
        .context("output reservation is mandatory request capacity")?;
    assert!(failure.to_string().contains("admission"), "{failure}");
    // @step And the previous memory and token state remain unchanged without completion
    fixture.assert_preserved(&original)?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn missing_capacity_cannot_authorize_a_dag_replacement() -> anyhow::Result<()> {
    // @step Given an explicitly unavailable provider capacity
    let mut fixture = Fixture::new(0, 0, "mandatory instructions").await?;
    let original = serde_json::to_vec(&fixture.session.messages)?;
    // @step And a valid small DAG returned by the selected builder
    fixture.select_builder();
    // @step When the compaction round attempts to commit the candidate
    let result = fixture.run().await?;
    // @step Then the round terminates boundedly with a blocked admission failure
    let failure = result
        .err()
        .context("missing limits are not a fit certificate")?;
    assert!(failure.to_string().contains("admission"), "{failure}");
    // @step And the previous memory and token state remain unchanged without completion
    fixture.assert_preserved(&original)?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_fitting_complete_request_remains_eligible_for_commit() -> anyhow::Result<()> {
    // @step Given explicit usable limits large enough for all retained context
    let mut fixture = Fixture::new(200_000, 4_096, "mandatory instructions").await?;
    // @step And a valid small DAG returned by the selected builder
    fixture.select_builder();
    // @step When the compaction round attempts to commit the candidate
    fixture.run().await??;
    // @step Then the selected unchanged DAG replaces the old view with one completion
    let memory = serde_json::to_string(&fixture.session.messages)?;
    let expected = serde_json::to_string(DAG)?;
    assert!(memory.contains(&expected[1..expected.len() - 1]));
    assert_eq!(fixture.output.completions(), 1);
    let events = fixture
        .output
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let publication = events.iter().position(|event| matches!(event,
        StreamEvent::ContextSnapshot(snapshot) if snapshot.generation == 1 && snapshot.input_tokens.is_some()
    )).context("authoritative committed context must be published")?;
    let completion = events
        .iter()
        .position(|event| matches!(event, StreamEvent::CompactionComplete(_)))
        .context("legacy completion follows publication")?;
    assert!(publication < completion);
    Ok(())
}
