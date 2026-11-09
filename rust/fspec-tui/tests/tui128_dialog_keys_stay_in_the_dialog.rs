//! TUI-128 — keys typed in a dialog never reach the view behind it.
//!
//! Feature: spec/features/keys-typed-in-a-dialog-never-reach-the-view-behind-it.feature
//!
//! Harness: the real `App` (`handle_event` / `handle_paste` / `dispatch`)
//! over `MockBackend` on a paused tokio clock, the TUI-124 / BUG-552
//! pattern. Every dialog is opened through the action or key the product
//! uses; every reply is awaited through the App's own pending tasks and
//! action bus (`drain`), so nothing depends on wall-clock time.

use crate::common::TaskOutcome;
use codelet_test_helpers::{TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionChangesSummary, SessionId, StreamChunk};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

const ERROR_DIALOG_ID: &str = "error-dialog";
const HELP_DIALOG_ID: &str = "help-dialog";
const CLOSE_DIALOG_ID: &str = "exit-confirmation-dialog";
const MERGE_DIALOG_ID: &str = "merge-confirm-dialog";
const NOTIFICATION_DIALOG_ID: &str = "notification-dialog";

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    (App::new(backend), mock)
}

/// Run every pending task and every queued action until nothing moves.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..100 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Err(TestError::from("the App never settled"))
}

async fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

async fn press(app: &mut App, code: KeyCode) -> TestResult {
    press_with(app, code, KeyModifiers::NONE).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch)).await?;
    }
    Ok(())
}

fn composer(app: &App) -> String {
    app.navigator().agent.input.value()
}

/// Open `sessions` and focus `focused` on the Agent view.
async fn agent_view(app: &mut App, sessions: &[&str], focused: &str) -> TestResult {
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    drain(app).await?;
    app.dispatch(Action::OpenAgentView(Some(sid(focused))));
    drain(app).await?;
    app.navigator_mut().active_view = ViewMode::Agent;
    Ok(())
}

/// s-1 focused, its worktree holding changes, `/merge-worktree` run.
async fn merge_dialog_open() -> Result<(App, Arc<MockBackend>), TestError> {
    let (mut app, mock) = fresh_app();
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 1,
        insertions: 3,
        deletions: 0,
        commits: vec!["abc1234".to_string()],
        files_ignored: 0,
    });
    agent_view(&mut app, &["s-1"], "s-1").await?;
    type_text(&mut app, "/merge-worktree").await?;
    press(&mut app, KeyCode::Enter).await?;
    if !app.compositor().contains(MERGE_DIALOG_ID) {
        return Err(TestError::from("/merge-worktree did not open its dialog"));
    }
    Ok((app, mock))
}

/// Scenario: Typing under an API Error dialog sends nothing to the session behind it
#[tokio::test(start_paused = true)]
async fn typing_under_an_api_error_dialog_sends_nothing_to_the_session_behind_it() -> TestResult {
    // @step Given session s-1 is focused on the agent view
    let (mut app, mock) = fresh_app();
    agent_view(&mut app, &["s-1"], "s-1").await?;

    // @step And an API Error dialog is open over it
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::error("429 Too Many Requests".to_string()),
    ));
    drain(&mut app).await?;
    assert!(app.compositor().contains(ERROR_DIALOG_ID));

    // @step When the user types "ok" and presses Enter
    type_text(&mut app, "ok").await?;
    press(&mut app, KeyCode::Enter).await?;

    // @step Then nothing is sent to s-1
    assert_eq!(mock.send_input_calls(), 0);
    assert!(mock.submitted_inputs().is_empty());

    // @step And the composer of s-1 is empty
    assert_eq!(composer(&app), "");

    // @step And the API Error dialog is still open
    assert!(app.compositor().contains(ERROR_DIALOG_ID));

    // @step When the user presses Esc
    press(&mut app, KeyCode::Esc).await?;

    // @step And types "ok"
    type_text(&mut app, "ok").await?;

    // @step Then the API Error dialog is closed
    assert!(!app.compositor().contains(ERROR_DIALOG_ID));

    // @step And the composer of s-1 holds "ok"
    assert_eq!(composer(&app), "ok");
    Ok(())
}

/// Scenario: Board keys pressed while the board help is open open nothing behind it
#[tokio::test(start_paused = true)]
async fn board_keys_pressed_while_the_board_help_is_open_open_nothing_behind_it() -> TestResult {
    // @step Given the board is shown
    let (mut app, _mock) = fresh_app();
    drain(&mut app).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Board);

    // @step And the user has opened the board help with "?"
    press(&mut app, KeyCode::Char('?')).await?;
    assert!(app.compositor().contains(HELP_DIALOG_ID));

    // @step When the user presses "p", "f" and "c"
    for ch in ['p', 'f', 'c'] {
        press(&mut app, KeyCode::Char(ch)).await?;
    }

    // @step Then the board help is still open
    assert!(app.compositor().contains(HELP_DIALOG_ID));

    // @step And the board is still shown
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    Ok(())
}

/// Scenario: Tab and Shift+Right under the close dialog change nothing behind it
#[tokio::test(start_paused = true)]
async fn tab_and_shift_right_under_the_close_dialog_change_nothing_behind_it() -> TestResult {
    // @step Given sessions s-1 and s-2 are open and s-1 is focused on the agent view
    let (mut app, _mock) = fresh_app();
    agent_view(&mut app, &["s-1", "s-2"], "s-1").await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));

    // @step And the "Close Agent?" dialog is open over s-1
    press(&mut app, KeyCode::Esc).await?;
    assert!(app.compositor().contains(CLOSE_DIALOG_ID));

    // @step When the user presses Tab and Shift+Right
    press(&mut app, KeyCode::Tab).await?;
    press_with(&mut app, KeyCode::Right, KeyModifiers::SHIFT).await?;

    // @step Then s-1 is still focused
    assert_eq!(app.current_session(), Some(sid("s-1")));

    // @step And turn-select mode is off
    assert!(!app.navigator().agent.turn_select_mode);

    // @step And the "Close Agent?" dialog is still open
    assert!(app.compositor().contains(CLOSE_DIALOG_ID));
    Ok(())
}

/// Scenario: Keys, ? and a paste under the merge dialog reach nothing behind it
#[tokio::test(start_paused = true)]
async fn keys_help_and_a_paste_under_the_merge_dialog_reach_nothing_behind_it() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    // @step And the "Merge Worktree" dialog is open over s-1
    let (mut app, _mock) = merge_dialog_open().await?;
    assert_eq!(composer(&app), "");

    // @step When the user types "x", presses "?" and pastes "hello"
    press(&mut app, KeyCode::Char('x')).await?;
    press(&mut app, KeyCode::Char('?')).await?;
    let _ = app.handle_paste("hello");
    drain(&mut app).await?;

    // @step Then the composer of s-1 is empty
    assert_eq!(composer(&app), "");

    // @step And no help dialog is open
    assert!(!app.compositor().contains(HELP_DIALOG_ID));

    // @step And the "Merge Worktree" dialog is still open
    assert!(app.compositor().contains(MERGE_DIALOG_ID));
    Ok(())
}

/// Scenario: Typing while a notification shows reaches nothing behind it
#[tokio::test(start_paused = true)]
async fn typing_while_a_notification_shows_reaches_nothing_behind_it() -> TestResult {
    // @step Given no session is open and the agent view is shown
    let (mut app, _mock) = fresh_app();
    app.navigator_mut().active_view = ViewMode::Agent;
    assert!(app.current_session().is_none());

    // @step And the notification from /debug is showing
    app.dispatch(Action::SlashCommandSelected(
        None,
        SlashCommandAction::Debug,
    ));
    drain(&mut app).await?;
    assert!(app.compositor().contains(NOTIFICATION_DIALOG_ID));

    // @step When the user types "hi"
    type_text(&mut app, "hi").await?;

    // @step Then the composer is empty
    assert_eq!(composer(&app), "");

    // @step When the user presses Esc
    press(&mut app, KeyCode::Esc).await?;
    assert!(!app.compositor().contains(NOTIFICATION_DIALOG_ID));

    // @step And types "hi"
    type_text(&mut app, "hi").await?;

    // @step Then the composer holds "hi"
    assert_eq!(composer(&app), "hi");
    Ok(())
}

/// Scenario: Ctrl+D quits while a dialog is open
#[tokio::test(start_paused = true)]
async fn ctrl_d_quits_while_a_dialog_is_open() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    // @step And the "Merge Worktree" dialog is open over s-1
    let (mut app, _mock) = merge_dialog_open().await?;

    // @step When the user presses Ctrl+D
    press_with(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL).await?;

    // @step Then fspec quits
    assert!(app.should_quit());
    Ok(())
}
