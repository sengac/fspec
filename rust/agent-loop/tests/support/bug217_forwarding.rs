use codelet_agent_loop::agent_manager_handler::create_handler;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::agent_manager::{AgentManagerAction, AgentManagerResult};
use std::sync::Arc;
use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    _directory: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub root: Uuid,
    pub child: Uuid,
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        let cache = directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("../fixtures/prov101_models.json"),
        )?;
        let manager = SessionManager::new_arc();
        let root = Uuid::new_v4();
        manager
            .create_session_with_id(
                &root.to_string(),
                "codex/gpt-5",
                &directory.path().to_string_lossy(),
                "root",
            )
            .await?;
        let middle = Uuid::new_v4();
        manager
            .create_session_with_id(
                &middle.to_string(),
                "codex/gpt-5",
                &directory.path().to_string_lossy(),
                "middle",
            )
            .await?;
        manager.add_supervisor(middle, root)?;
        let handler = create_handler(
            Some(manager.clone()),
            directory.path().to_string_lossy().into_owned(),
            Some("codex/gpt-5".into()),
            None,
            None,
        );
        let spawned = handler(AgentManagerAction::Spawn { role: None }, middle);
        let AgentManagerResult::Spawned { session_id } = spawned else {
            return Err(format!("expected subordinate spawn, got {spawned:?}").into());
        };
        Ok(Self {
            _directory: directory,
            manager,
            root,
            child: Uuid::parse_str(&session_id)?,
        })
    }

    pub fn session(&self) -> Result<Arc<BackgroundSession>, String> {
        self.manager.get_session(&self.child.to_string())
    }

    pub async fn burst(&self, prefix: &str) -> TestResult {
        let session = self.session()?;
        let prefix = prefix.to_owned();
        tokio::spawn(async move {
            let mut witness = session.subscribe_to_stream();
            for index in 0..600 {
                if index == 100 {
                    session.handle_output(StreamChunk::done());
                } else {
                    session.handle_output(StreamChunk::text(format!("{prefix}-{index}")));
                }
            }
            assert!(matches!(
                witness.try_recv(),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_))
            ));
        })
        .await?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.manager.destroy_session(&self.child.to_string());
    }
}
