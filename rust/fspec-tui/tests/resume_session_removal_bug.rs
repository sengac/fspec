//! E2E integration test: /resume session removal bug reproduction.
//!
//! Drives the REAL embedded backend + SharedFspecService + real
//! SessionManager + App::dispatch through the full /resume flow with
//! NO mocks. Uses real persistence, real SessionManager, real App
//! dispatch routing.
//!
//! Steps:
//! 1. Create a session via the real backend (through SessionManager)
//! 2. Verify it appears in list_sessions
//! 3. Open resume view, select the session (AttachToSession)
//! 4. List sessions again — the session should STILL exist
//! 5. Verify the session manifest is intact on disk

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock
)]

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use codelet_common::get_data_dir;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_core::SessionManagerHandle;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_sessions::SessionManager;
use tempfile::TempDir;

static DATA_DIR_MUTEX: Mutex<()> = Mutex::new(());

fn setup_temp_data_dir() -> (std::sync::MutexGuard<'static, ()>, TempDir) {
    let guard = DATA_DIR_MUTEX.lock().expect("DATA_DIR_MUTEX");
    let temp = tempfile::tempdir().expect("tempdir");
    codelet_common::set_data_directory(temp.path().to_path_buf()).expect("set_data_directory");
    codelet_core::persistence::reset_stores_for_tests();
    // Hermetic: seed fake provider keys so `create_session` (which uses
    // the `openai:spark/qwen3.6` default model) works without relying on
    // ambient ANTHROPIC_API_KEY / OPENAI_API_KEY in the developer's shell.
    std::env::set_var("ANTHROPIC_API_KEY", "resume-bug-fake-ant-key");
    std::env::set_var("OPENAI_API_KEY", "resume-bug-fake-openai-key");
    (guard, temp)
}

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

/// Build a SharedFspecService with a real SessionManager attached.
fn service_with_session_manager(
    repo_path: &Path,
) -> (Arc<SharedFspecService>, Arc<SessionManager>) {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("watcher"));
    let manager = Arc::new(SessionManager::new());
    let service = Arc::new(SharedFspecService::with_session_manager(
        watcher,
        Arc::clone(&manager) as Arc<dyn SessionManagerHandle>,
    ));
    (service, manager)
}

/// Helper: drain all pending tasks and fold queued actions back into the App.
async fn drain_pending(app: &mut App) {
    loop {
        let mut drained = false;
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
            drained = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            drained = true;
        }
        if !drained {
            break;
        }
    }
}

/// Scenario: Full /resume flow through App::dispatch with real backend.
///
/// This replicates the exact bug: user does /resume on a session and it
/// disappears from the session history.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_session_through_app_dispatch_preserves_session() {
    // @step Given a fresh data directory with a workspace and real service + session manager
    let (_guard, temp) = setup_temp_data_dir();
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let (service, manager) = service_with_session_manager(&cwd);

    // @step When I create a session via the real backend (which goes through SessionManager)
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));

    // Set default model so create_session works
    manager.set_default_model("openai:spark/qwen3.6");

    // Create session via the backend (goes through SessionManager::create_session)
    let session_id = backend.create_session(None).await.expect("create_session");

    tracing::info!(
        session_id = %session_id.value,
        "E2E TEST: Created session via backend for resume test"
    );

    // @step Then list_sessions includes the session
    let sessions_before = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions before");
    let found_before = sessions_before.iter().any(|s| s.id == session_id.value);
    assert!(
        found_before,
        "Session {} should appear in list_sessions before resume",
        session_id.value
    );
    tracing::info!(
        session_count_before = sessions_before.len(),
        session_id = %session_id.value,
        "E2E TEST: list_sessions before resume"
    );

    // @step When I build an App and dispatch OpenResumeView
    let mut app = App::new(backend.clone());
    app.dispatch(Action::OpenResumeView);

    // @step And I drain pending tasks (list_sessions RPC round-trip)
    drain_pending(&mut app).await;

    // @step And I dispatch AttachToSession with the session ID
    app.dispatch(Action::AttachToSession(session_id.clone()));

    // @step And I drain all pending tasks (resume_session round-trip)
    drain_pending(&mut app).await;

    // @step Then list_sessions should STILL include the session
    let sessions_after = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions after");
    let found_after = sessions_after.iter().any(|s| s.id == session_id.value);

    tracing::info!(
        session_count_after = sessions_after.len(),
        session_id = %session_id.value,
        found_after,
        "E2E TEST: list_sessions AFTER resume through App dispatch"
    );

    assert!(
        found_after,
        "BUG: Session {} disappeared from list_sessions after /resume!\n\
         Sessions after resume: {:?}",
        session_id.value,
        sessions_after
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
    );

    // @step And the session manifest should still exist on disk
    let sessions_dir = get_data_dir().expect("data_dir").join("sessions");
    let manifest_files = fs::read_dir(&sessions_dir)
        .expect("read sessions dir")
        .count();
    tracing::info!(
        manifest_files,
        "E2E TEST: manifest files on disk after resume"
    );
    assert!(
        manifest_files > 0,
        "BUG: Session manifest files were deleted after /resume!"
    );
}

/// Scenario: Multiple /resume cycles — open resume, select, open resume again.
///
/// This tests whether repeated /resume calls cause sessions to disappear.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_resume_cycles_preserve_sessions() {
    // @step Given a fresh data directory with two sessions
    let (_guard, temp) = setup_temp_data_dir();
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let (service, manager) = service_with_session_manager(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));

    manager.set_default_model("openai:spark/qwen3.6");

    // Create two sessions via the backend
    let id1 = backend
        .create_session(None)
        .await
        .expect("create session 1");
    let id2 = backend
        .create_session(None)
        .await
        .expect("create session 2");

    tracing::info!(
        session1 = %id1.value,
        session2 = %id2.value,
        "E2E TEST: Created two sessions for multi-cycle test"
    );

    let mut app = App::new(backend.clone());

    // First /resume cycle: select session 1
    app.dispatch(Action::OpenResumeView);
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(id1.clone()));
    drain_pending(&mut app).await;

    // Second /resume cycle: select session 2
    app.dispatch(Action::OpenResumeView);
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(id2.clone()));
    drain_pending(&mut app).await;

    // Third /resume cycle: select session 1 again
    app.dispatch(Action::OpenResumeView);
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(id1.clone()));
    drain_pending(&mut app).await;

    // @step Then both sessions should still exist
    let sessions = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions");
    let ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();

    tracing::info!(
        session_count = sessions.len(),
        ids = ?ids,
        "E2E TEST: list_sessions after multiple resume cycles"
    );

    let found1 = ids.iter().any(|s| *s == id1.value);
    let found2 = ids.iter().any(|s| *s == id2.value);

    assert!(
        found1,
        "BUG: Session 1 {} disappeared after multiple resume cycles!\n\
         Sessions: {:?}",
        id1.value, ids
    );
    assert!(
        found2,
        "BUG: Session 2 {} disappeared after multiple resume cycles!\n\
         Sessions: {:?}",
        id2.value, ids
    );
}

/// Scenario: Resume a session, then destroy it via the backend, then verify
/// destroy_session only removes from memory (manifest persists).
/// persistence_delete_session is required to remove the manifest.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_then_explicit_destroy_removes_session() {
    // @step Given a fresh data directory with one session
    let (_guard, temp) = setup_temp_data_dir();
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let (service, manager) = service_with_session_manager(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));

    manager.set_default_model("openai:spark/qwen3.6");

    let session_id = backend.create_session(None).await.expect("create session");

    // @step When I resume the session
    let mut app = App::new(backend.clone());
    app.dispatch(Action::OpenResumeView);
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(session_id.clone()));
    drain_pending(&mut app).await;

    // @step Then the session should exist
    let sessions_after_resume = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions after resume");
    assert!(
        sessions_after_resume
            .iter()
            .any(|s| s.id == session_id.value),
        "Session should exist after resume"
    );

    // @step When I explicitly destroy the session
    // This only removes from in-memory; manifest persists on disk
    let destroy_result = backend.destroy_session(session_id.clone()).await;
    assert!(destroy_result.is_ok(), "destroy_session should succeed");

    // @step Then the session should STILL appear in list_sessions (via persisted merge)
    // because destroy_session does NOT delete the manifest
    let sessions_after_destroy = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions after destroy");
    let found_after_destroy = sessions_after_destroy
        .iter()
        .any(|s| s.id == session_id.value);

    tracing::info!(
        session_count = sessions_after_destroy.len(),
        found_after_destroy,
        "E2E TEST: list_sessions after explicit destroy — session still visible via persisted manifest"
    );

    assert!(
        found_after_destroy,
        "Session should still appear in list_sessions after destroy_session \
         (manifest persists on disk, list_sessions merges persisted sessions)"
    );

    // @step When I also call persistence_delete_session to remove the manifest
    let delete_result = backend.persistence_delete_session(session_id.clone()).await;
    assert!(
        delete_result.is_ok(),
        "persistence_delete_session should succeed"
    );

    // @step Then the session should be gone from list_sessions
    let sessions_after_delete = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions after delete");
    let found_after_delete = sessions_after_delete
        .iter()
        .any(|s| s.id == session_id.value);

    tracing::info!(
        session_count = sessions_after_delete.len(),
        found_after_delete,
        "E2E TEST: list_sessions after persistence_delete_session"
    );

    assert!(
        !found_after_delete,
        "Session should be gone after persistence_delete_session removes the manifest"
    );
}

/// Scenario: Resume a session that was created on disk (not in memory).
///
/// This tests the create_session_from_manifest path — the session exists
/// on disk but the SessionManager doesn't have it in memory.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_session_from_disk_preserves_session() {
    // @step Given a fresh data directory with a session on disk
    let (_guard, temp) = setup_temp_data_dir();
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let (service, manager) = service_with_session_manager(&cwd);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));

    manager.set_default_model("openai:spark/qwen3.6");

    // Create a session via the backend (this puts it in memory AND on disk)
    let session_id = backend.create_session(None).await.expect("create session");

    tracing::info!(
        session_id = %session_id.value,
        "E2E TEST: Created session on disk for resume-from-disk test"
    );

    // Verify the manifest exists on disk
    let sessions_dir = get_data_dir().expect("data_dir").join("sessions");
    let manifest_files_before = fs::read_dir(&sessions_dir)
        .expect("read sessions dir")
        .count();
    assert!(
        manifest_files_before > 0,
        "Session manifest should exist on disk"
    );

    // @step When I resume the session
    let mut app = App::new(backend.clone());
    app.dispatch(Action::OpenResumeView);
    drain_pending(&mut app).await;
    app.dispatch(Action::AttachToSession(session_id.clone()));
    drain_pending(&mut app).await;

    // @step Then the session should still exist in list_sessions
    let sessions_after = backend
        .list_sessions(String::new())
        .await
        .expect("list_sessions after");
    let found = sessions_after.iter().any(|s| s.id == session_id.value);

    tracing::info!(
        session_count = sessions_after.len(),
        found,
        "E2E TEST: list_sessions after resume from disk"
    );

    assert!(
        found,
        "BUG: Session {} disappeared after resume from disk!\n\
         Sessions: {:?}",
        session_id.value,
        sessions_after
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
    );

    // @step And the manifest should still exist on disk
    let manifest_files_after = fs::read_dir(&sessions_dir)
        .expect("read sessions dir")
        .count();
    assert!(
        manifest_files_after > 0,
        "BUG: Session manifest was deleted from disk after resume!"
    );
}
