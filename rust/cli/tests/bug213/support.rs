use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::{ProviderManager, ProviderType};
use std::sync::{Arc, Mutex, PoisonError};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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

pub struct Fixture {
    pub session: Session,
    pub id: uuid::Uuid,
    pub output: Output,
    pub started: Arc<tokio::sync::Notify>,
    _directory: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let mut manifest =
            create_session_with_provider("truthful activity", directory.path(), "openai")?;
        append_message(&mut manifest, "user", "original question")?;
        append_message(&mut manifest, "assistant", "original answer")?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
        manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
        let mut session = Session::from_provider_manager(manager);
        session.messages = vec![
            rig::message::Message::user("original question"),
            rig::message::Message::assistant("original answer"),
        ];
        session.token_tracker.input_tokens = 195_000;
        let started = Arc::new(tokio::sync::Notify::new());
        let entered = started.clone();
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
            manifest.id,
            Some(Arc::new(move |_, _| {
                entered.notify_one();
                Box::pin(std::future::pending())
            })),
        );
        Ok(Self {
            session,
            id: manifest.id,
            output: Output::default(),
            started,
            _directory: directory,
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.id, None);
    }
}
