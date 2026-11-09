//! COV-057 in-view DAG fixture: a persisted four-turn history (third turn a
//! 500-line Read result) behind a real `BackgroundSession`, plus a manager
//! whose agent loop is retained but never run.

use codelet_cli::session::system_reminders::add_system_reminder;
use codelet_cli::session::{Session, SystemReminderType};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::session_manager::SessionManagerHooks;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::message::Message;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const READ_ID: &str = "read-entry-point";

fn metadata(message: serde_json::Value) -> HashMap<String, serde_json::Value> {
    [
        ("uuid".to_owned(), json!(uuid::Uuid::new_v4())),
        ("message".to_owned(), message),
    ]
    .into()
}

fn read_output() -> String {
    (1..=500)
        .map(|line| format!("     {line}: line {line} content here with some text"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn text(message: &Message) -> String {
    serde_json::to_string(message).expect("serializable message")
}

pub struct Fixture {
    pub session: Arc<BackgroundSession>,
    pub directory: tempfile::TempDir,
}

impl Fixture {
    /// Persist a four-turn history whose third turn is a 500-line Read result,
    /// with an environment system reminder in the live context.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let mut manifest = codelet_core::persistence::create_session_with_provider(
            "in-view dag",
            directory.path(),
            "openai",
        )?;
        let request = "Review the entry point in src/main.rs";
        let finding = "The entry point loads configuration and starts the server.";
        let turns = [
            (
                "user",
                request.to_owned(),
                json!({"role": "user", "content": request}),
            ),
            (
                "assistant",
                "[tool_use: Read]".to_owned(),
                json!({"role": "assistant",
                "content": [{"type": "tool_use", "id": READ_ID, "name": "Read",
                    "input": {"file_path": "src/main.rs"}}]}),
            ),
            (
                "user",
                read_output(),
                json!({"role": "user",
                "content": [{"type": "tool_result", "tool_use_id": READ_ID,
                    "content": "[content in main field]", "is_error": false}]}),
            ),
            (
                "assistant",
                finding.to_owned(),
                json!({"role": "assistant", "content": finding}),
            ),
        ];
        for (role, content, message) in turns {
            codelet_core::persistence::append_message_with_metadata(
                &mut manifest,
                role,
                &content,
                metadata(message),
            )?;
        }
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
        manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
        let mut inner = Session::from_provider_manager(manager);
        inner.messages = add_system_reminder(&[], SystemReminderType::Environment, "cwd: /repo");
        inner.messages.push(Message::user(request));
        inner.messages.push(Message::assistant(finding));
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-test")
                .build()?;
        let model = client.completion_model("o3");
        inner.install_selected_preparer(SelectedRequestPreparer::new(
            model.clone(),
            CompletionRequestBuilder::new(model, "unused")
                .max_tokens(4096)
                .build(),
            AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            },
        ));
        let (input_tx, _) = mpsc::channel(4);
        let session = Arc::new(BackgroundSession::new(
            manifest.id,
            "in-view dag".into(),
            directory.path().to_string_lossy().into(),
            Some("openai".into()),
            Some("o3".into()),
            inner,
            input_tx,
            None,
            None,
            None,
            broadcast::channel(64).0,
            broadcast::channel(16).0,
            broadcast::channel(4).0,
        ));
        Ok(Self { session, directory })
    }
}

/// The DAG an agent writes from the trimmed SessionSearch turns.
pub fn dag_from(
    messages: &[codelet_tools::session_search::SessionMessage],
    reference: &str,
) -> String {
    let last = messages.len() - 1;
    format!(
        "<dag-node depth=\"D2\" turns=\"0-{last}\" label=\"Entry point review\">Reviewed the startup flow of src/main.rs.</dag-node>\n\
         <dag-node depth=\"D1\" turns=\"0-1\" label=\"Request\">The user asked for an entry point review.</dag-node>\n\
         <dag-node depth=\"D1\" turns=\"2-{last}\" label=\"Findings\">Configuration loads before the server starts.</dag-node>\n\
         <dag-node depth=\"D0\" turns=\"2-2\" label=\"Read evidence\">{reference}</dag-node>"
    )
}

/// Keeps each session's input queue open without running an agent loop, so
/// the queued manual-compaction request (BUG-516) has a live receiver.
#[derive(Default)]
pub struct RetainedLoop(Mutex<Vec<mpsc::Receiver<PromptInput>>>);

impl RetainedLoop {
    /// Take every input queued so far on the retained receivers.
    pub fn take_queued(&self) -> Vec<PromptInput> {
        let mut queued = Vec::new();
        if let Ok(mut retained) = self.0.lock() {
            for receiver in retained.iter_mut() {
                while let Ok(input) = receiver.try_recv() {
                    queued.push(input);
                }
            }
        }
        queued
    }
}

impl SessionManagerHooks for RetainedLoop {
    fn spawn_agent_loop(
        &self,
        _: Arc<BackgroundSession>,
        input: mpsc::Receiver<PromptInput>,
        _: mpsc::Receiver<codelet_tools::McpInjection>,
    ) {
        if let Ok(mut retained) = self.0.lock() {
            retained.push(input);
        }
    }
    fn spawn_scheduler(&self, _: String, _: tokio::runtime::Handle) {}
    fn ensure_scheduler_running_for_loop(&self, _: String, _: tokio::runtime::Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}
