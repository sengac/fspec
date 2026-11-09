// Feature: spec/features/codex-models-never-block-the-provider-list-or-replace-the-saved-model.feature
//! BUG-577: the daemon builds the provider list on its blocking pool, so a
//! slow Codex /models fetch or profile probe never holds an async worker.
//!
//! The real `FspecServiceImpl` runs on a runtime with ONE worker thread. Its
//! session manager's `list_providers` blocks until the test releases it.
//! While it is blocked, a second request must still be answered: that is
//! only possible when the list is not built on the single worker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::{
    LogRecord, ProviderInfo, SessionId, SessionInfo, SessionStatus, StreamChunk,
};
use codelet_test_helpers::{deadlock_guard, wait_until, TestError, TestResult};
use tokio::sync::broadcast;

/// A session manager whose provider list blocks until released; the rest
/// is the stub's.
struct BlockedList {
    stub: StubSessionManagerHandle,
    entered: AtomicBool,
    release: Mutex<Receiver<()>>,
}

impl SessionManagerHandle for BlockedList {
    fn list_providers(&self) -> Result<Vec<ProviderInfo>, String> {
        self.entered.store(true, Ordering::SeqCst);
        self.release
            .lock()
            .map_err(|_| "the release lock is poisoned".to_string())?
            .recv()
            .map_err(|error| error.to_string())?;
        Ok(vec![ProviderInfo {
            key: "anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            ..ProviderInfo::default()
        }])
    }

    fn get_default_model(&self) -> Option<String> {
        Some("anthropic/claude-opus-5-5".to_string())
    }

    fn list_sessions(&self, project_path: &str) -> Result<Vec<SessionInfo>, String> {
        self.stub.list_sessions(project_path)
    }

    fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String> {
        self.stub.create_session_checked(role)
    }

    fn send_input(&self, session_id: &SessionId, text: String) -> Result<(), String> {
        self.stub.send_input(session_id, text)
    }

    fn interrupt(&self, session_id: &SessionId) -> Result<(), String> {
        self.stub.interrupt(session_id)
    }

    fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus, String> {
        self.stub.get_session_status(session_id)
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.stub.chunks_rx()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.stub.logs_rx()
    }

    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.stub.chunks_tx()
    }

    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.stub.logs_tx()
    }
}

fn daemon(
    project: &std::path::Path,
) -> Result<(FspecServiceImpl, Arc<BlockedList>, Sender<()>), TestError> {
    std::fs::create_dir_all(project.join("spec"))?;
    std::fs::write(project.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    let (release, released) = channel();
    let blocked = Arc::new(BlockedList {
        stub: StubSessionManagerHandle::new(),
        entered: AtomicBool::new(false),
        release: Mutex::new(released),
    });
    let manager: Arc<dyn SessionManagerHandle> = blocked.clone();
    let shared = SharedFspecService::with_session_manager(
        Arc::new(WorkUnitsWatcher::new(project)?),
        manager,
    )
    .with_cwd(project.to_path_buf());
    Ok((FspecServiceImpl::new(Arc::new(shared)), blocked, release))
}

// Scenario: A slow provider list does not hold up other daemon requests
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn a_slow_provider_list_does_not_hold_up_other_daemon_requests() -> TestResult {
    // @step Given the daemon has one worker thread and building the provider list is blocked
    let root = tempfile::tempdir()?;
    let (service, blocked, release) = daemon(root.path())?;
    let list = tokio::spawn(service.clone().list_providers(tarpc::context::current()));
    let watched = Arc::clone(&blocked);
    wait_until("the provider list to start", move || {
        let entered = watched.entered.load(Ordering::SeqCst);
        async move { entered }
    })
    .await?;

    // @step When another request is sent while the provider list is being built
    let other = tokio::spawn(service.get_default_model(tarpc::context::current()));
    let answered = deadlock_guard("the other request", other).await;
    let list_was_pending = !list.is_finished();
    // Unblock the list whatever happened, so the runtime can shut down.
    release.send(())?;

    // @step Then the other request is answered before the provider list
    assert_eq!(answered??.as_deref(), Some("anthropic/claude-opus-5-5"));
    assert!(
        list_was_pending,
        "the provider list finished before it was released"
    );

    // @step And the provider list is returned once it is unblocked
    let providers = deadlock_guard("the provider list", list).await???;
    assert_eq!(providers.len(), 1);
    Ok(())
}
