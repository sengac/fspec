use codelet_agent_loop::agent_manager_handler::{create_async_handler, create_handler};
use codelet_rpc_types::SessionStatus;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::agent_manager::{
    AgentManagerAction, AgentManagerAsyncHandler, AgentManagerHandler, AgentManagerResult,
};
use std::sync::Arc;
use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    pub handler: AgentManagerHandler,
    pub async_handler: AgentManagerAsyncHandler,
    pub target: Arc<BackgroundSession>,
    pub caller: Uuid,
    pub target_id: String,
    _directory: tempfile::TempDir,
    _manager: Arc<SessionManager>,
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        std::env::set_var("ANTHROPIC_API_KEY", "busy-message-test-key");
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let cache = directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("../fixtures/prov101_models.json"),
        )?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        let manager = Arc::new(SessionManager::new());
        let target_id = Uuid::new_v4().to_string();
        manager
            .create_session_with_id(
                &target_id,
                "anthropic/claude-opus-4-5",
                directory.path().to_str().ok_or("non-UTF-8 project path")?,
                "recipient",
            )
            .await?;
        let target = manager.get_session(&target_id)?;
        assert_eq!(target.get_status(), SessionStatus::Idle);
        let handler = create_handler(Some(manager.clone()), String::new(), None, None, None);
        let async_handler = create_async_handler(Some(manager.clone()));
        Ok(Self {
            handler,
            async_handler,
            target,
            caller: Uuid::new_v4(),
            target_id,
            _directory: directory,
            _manager: manager,
        })
    }

    pub fn send(&self, message: &str) -> AgentManagerResult {
        (self.handler)(
            AgentManagerAction::Message {
                session_id: self.target_id.clone(),
                message: message.to_string(),
                context: None,
            },
            self.caller,
        )
    }

    pub fn status_changes(
        &self,
    ) -> tokio::sync::broadcast::Receiver<(codelet_rpc_types::SessionId, SessionStatus)> {
        self._manager.status_changes_tx().subscribe()
    }

    pub async fn finish(&self) -> TestResult {
        self.target.incoming_message_rx.lock().await.try_recv()?;
        self.target
            .incoming_message_pending
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        self.target.set_status(SessionStatus::Idle);
        Ok(())
    }
}

pub fn assert_busy(result: AgentManagerResult, target_id: &str) -> TestResult {
    let AgentManagerResult::Error {
        error,
        code,
        message,
    } = result
    else {
        return Err(format!("expected session_busy, got {result:?}").into());
    };
    assert!(error);
    assert_eq!(code, "session_busy");
    assert!(message.contains(target_id));
    assert!(message.contains("await_idle"));
    assert!(message.contains("send"));
    assert!(message.contains("waiting for you"));
    assert!(message.contains("return results"));
    Ok(())
}
