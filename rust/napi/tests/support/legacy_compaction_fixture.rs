use std::ffi::OsString;
use std::sync::{Arc, Mutex, PoisonError};

use codelet_rpc_types::SessionId;
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::{SessionManager, SessionManagerHooks};
use tokio::sync::mpsc;

#[derive(Default)]
struct DeferredInput(Mutex<Vec<mpsc::Receiver<PromptInput>>>);

impl SessionManagerHooks for DeferredInput {
    fn spawn_agent_loop(
        &self,
        _: Arc<BackgroundSession>,
        input: mpsc::Receiver<PromptInput>,
        _: mpsc::Receiver<codelet_tools::McpInjection>,
    ) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(input);
    }

    fn spawn_scheduler(&self, _: String, _: tokio::runtime::Handle) {}
    fn ensure_scheduler_running_for_loop(&self, _: String, _: tokio::runtime::Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}

pub struct Fixture {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let environment = ["FSPEC_USER_DIR", "OPENAI_API_KEY", "OPENAI_BASE_URL"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        let fixture = Self {
            directory,
            environment,
        };
        std::env::set_var("FSPEC_USER_DIR", fixture.directory.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        std::env::set_var("OPENAI_BASE_URL", "http://127.0.0.1:1/v1");
        codelet_common::set_data_directory(fixture.directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let cache = fixture.directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("../../../agent-loop/tests/fixtures/prov101_models.json"),
        )?;
        Ok(fixture)
    }

    pub async fn create_session(
        &self,
        manager: &SessionManager,
    ) -> Result<SessionId, Box<dyn std::error::Error>> {
        manager.set_hooks(Arc::new(DeferredInput::default()));
        let id = uuid::Uuid::new_v4().to_string();
        manager
            .create_session_with_id(
                &id,
                "openai/o3",
                &self.directory.path().to_string_lossy(),
                "legacy compaction",
            )
            .await?;
        Ok(SessionId::from(id))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (key, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
