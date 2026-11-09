//! BUG-647: /mux errors are never dropped. A failed save from the mux
//! config dialog is an app-level error (the Session error dialog, through
//! Action::SessionOperationFailed) whether or not a session is open; a
//! /mux slash-command error stays a line in the submitting session's
//! scrollback.
//!
//! Feature: spec/features/mux-errors-and-notices-show-with-no-session-open.feature
//!
//! Harness: App + MockBackend with the process data directory rooted in a
//! temp dir (the bug167 pattern). A user fspec-config.json that is not JSON
//! makes every save fail (BUG-461). Deterministic: no clock, no network.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::sync::Arc;

use serial_test::serial;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

/// Serialises tests that mutate the process-global data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const MUX_CONFIG_DIALOG_ID: &str = "mux-config-dialog";
const SAVE_ERROR_PREFIX: &str = "/mux: could not save config:";

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Root the process data directory at a temp dir whose user
/// fspec-config.json is not JSON, so a save fails (BUG-461).
async fn root_data_dir_with_broken_config() -> Result<
    (tokio::sync::MutexGuard<'static, ()>, tempfile::TempDir),
    codelet_test_helpers::TestError,
> {
    let guard = DATA_DIR_GUARD.lock().await;
    let tmp = tempfile::tempdir().or_fail("tempdir")?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(tmp.path().to_path_buf()).or_fail("set data dir")?;
    fs::write(tmp.path().join("fspec-config.json"), "{ not json").or_fail("seed config")?;
    Ok((guard, tmp))
}

/// Open the mux config dialog (the board's M key action) and commit its
/// draft with Save (Shift+Tab from the layout field wraps to Save; Enter).
async fn commit_dialog_with_save(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(Action::OpenMuxConfigDialog);
    drain_pending(app).await?;
    assert!(app.compositor().contains(MUX_CONFIG_DIALOG_ID));
    app.handle_event(&key(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(app).await?;
    Ok(())
}

fn save_error(app: &App) -> Option<&String> {
    app.session_operation_errors()
        .iter()
        .find(|error| error.starts_with(SAVE_ERROR_PREFIX))
}

fn scrollback_text(
    app: &App,
    session: &SessionId,
) -> Result<String, codelet_test_helpers::TestError> {
    let context = app
        .agent_view_store()
        .session_context_for(session)
        .or_fail("the session context")?;
    Ok(context
        .scrollback
        .chunks()
        .iter()
        .filter_map(|chunk| chunk.source.as_ref().map(|source| source.text.clone()))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[tokio::test]
#[serial]
async fn with_no_session_open_a_failed_save_from_the_mux_config_dialog_shows_as_an_app_level_error(
) -> TestResult {
    // @step Given no agent session is open and the user config file is not valid JSON
    let (_guard, _tmp) = root_data_dir_with_broken_config().await?;
    let mut app = fresh_app();
    assert!(app.current_session().is_none());

    // @step When the mux config dialog commits its draft with Save
    commit_dialog_with_save(&mut app).await?;

    // @step Then the Session error dialog lists an error starting "/mux: could not save config:"
    assert!(
        save_error(&app).is_some(),
        "errors: {:?}",
        app.session_operation_errors()
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn with_a_session_open_a_mux_error_stays_in_its_scrollback() -> TestResult {
    // @step Given an agent session is open and current
    let mut app = fresh_app();
    let session = SessionId::new("s-1");
    app.dispatch(Action::SessionCreated(session.clone()));
    drain_pending(&mut app).await?;
    assert_eq!(app.current_session(), Some(session.clone()));

    // @step When the user submits "/mux banana"
    app.dispatch(Action::InputSubmitted(
        session.clone(),
        "/mux banana".to_string(),
    ));
    drain_pending(&mut app).await?;

    // @step Then the session's last scrollback line is the /mux parse error
    let text = scrollback_text(&app, &session)?;
    let last = text.lines().last().or_fail("a scrollback line")?;
    assert!(
        last.contains("banana"),
        "the last scrollback line must be the /mux parse error: {last}"
    );

    // @step And no dialog opens
    assert!(
        app.compositor().is_empty(),
        "layers={:?}",
        app.compositor().layer_ids()
    );
    assert!(app.session_operation_errors().is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn with_a_session_open_a_failed_save_from_the_mux_config_dialog_still_shows_as_an_app_level_error(
) -> TestResult {
    // @step Given an agent session is open and current and the user config file is not valid JSON
    let (_guard, _tmp) = root_data_dir_with_broken_config().await?;
    let mut app = fresh_app();
    let session = SessionId::new("s-1");
    app.dispatch(Action::SessionCreated(session.clone()));
    drain_pending(&mut app).await?;

    // @step When the mux config dialog commits its draft with Save
    commit_dialog_with_save(&mut app).await?;

    // @step Then the Session error dialog lists an error starting "/mux: could not save config:"
    assert!(
        save_error(&app).is_some(),
        "errors: {:?}",
        app.session_operation_errors()
    );

    // @step And the session's scrollback has no line starting "/mux: could not save config:"
    let text = scrollback_text(&app, &session)?;
    assert!(
        !text.lines().any(|line| line.starts_with(SAVE_ERROR_PREFIX)),
        "the save error must not be hidden in the scrollback: {text}"
    );
    Ok(())
}
