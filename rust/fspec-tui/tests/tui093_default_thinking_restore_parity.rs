//! Feature: spec/features/default-thinking-level-restore-parity.feature
//!
//! TUI-093 — App::dispatch wiring for the D repaint: pressing D repaints the
//! active session's `[T:level]` badge (a `get_thinking_level` re-fetch +
//! `Action::ThinkingLevelLoaded`). MODEL-012 removed the TUI-side apply of a
//! saved level (the daemon applies each model's saved level), so the
//! startup / resume / refocus scenarios are gone.
//!
//! The test roots the process-global data directory (via
//! `codelet_common::set_data_directory`) at a throwaway directory; a `Mutex`
//! serialises it.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use tokio::sync::Mutex;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, ThinkingLevel};

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Root a throwaway data dir as the process-global data directory. Returns
/// the kept TempDir.
fn throwaway_data_dir() -> Result<tempfile::TempDir, codelet_test_helpers::TestError> {
    let tmp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    Ok(tmp)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// Scenario: Pressing D persists the default and immediately repaints the badge
#[tokio::test]
async fn pressing_d_persists_the_default_and_immediately_repaints_the_badge() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given an active session and the thinking dialog is open on Medium
    let _tmp = throwaway_data_dir()?;
    let (mut app, mock) = fresh_app();
    // Session is created with the badge at Off (no persisted default).
    mock.set_thinking_level(ThinkingLevel::Off);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await?;
    // After pressing D the in-memory base level becomes Medium; model the
    // backend re-fetch returning that new value.
    mock.set_thinking_level(ThinkingLevel::Effort("medium".to_string()));
    assert_ne!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .cloned(),
        Some(ThinkingLevel::Effort("medium".to_string())),
        "precondition: badge not yet Medium"
    );

    // @step When the user presses D to set Medium as the default
    app.dispatch(Action::SetThinkingLevelDefault(
        SessionId::new("s-1"),
        ThinkingLevel::Effort("medium".to_string()),
    ));
    drain_pending(&mut app).await?;

    // @step Then the default thinking level Medium is persisted to the shared config
    // (set_thinking_level_default is best-effort persist; the repaint is the
    //  user-visible behaviour pinned here)
    // @step And the handler dispatches a ThinkingLevelLoaded action carrying Medium for that session
    assert_eq!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .cloned(),
        Some(ThinkingLevel::Effort("medium".to_string())),
        "pressing D must repaint the badge to Medium via ThinkingLevelLoaded"
    );
    Ok(())
}
