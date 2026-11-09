//! TUI-127 — dialog buttons and menu entries reach their handlers.
//!
//! Feature: spec/features/dialog-buttons-and-menu-entries-reach-their-handlers.feature
//!
//! Harness: the real `App` dispatch over `MockBackend` (the TUI-124 /
//! BUG-552 pattern) on a paused tokio clock. Every key and click goes
//! through `App::handle_event`; every backend reply is awaited through the
//! App's own pending tasks and action bus (`drain`), so nothing depends on
//! wall-clock time. Texts are read from a full-App render.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionChangesSummary, SessionId};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

const MERGE_DIALOG_ID: &str = "merge-confirm-dialog";
const NOTIFICATION_DIALOG_ID: &str = "notification-dialog";
const ADD_WORKSPACE_DIALOG_ID: &str = "add-workspace-dialog";
const WIDTH: u16 = 120;
const HEIGHT: u16 = 30;

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

async fn press(app: &mut App, code: KeyCode) -> TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    drain(app).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch)).await?;
    }
    Ok(())
}

fn render(app: &mut App) -> Buffer {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol().to_string())
        .collect()
}

fn screen(app: &mut App) -> String {
    let buffer = render(app);
    (0..buffer.area.height)
        .map(|y| row_text(&buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_shows(text: &str, needle: &str) -> TestResult {
    if text.contains(needle) {
        Ok(())
    } else {
        Err(TestError::from(format!("missing {needle:?} in:\n{text}")))
    }
}

/// Session s-1 focused on the Agent view, its worktree holding changes.
async fn session_with_worktree_changes() -> Result<(App, Arc<MockBackend>), TestError> {
    let (mut app, mock) = fresh_app();
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 2,
        insertions: 7,
        deletions: 1,
        commits: vec!["abc1234".to_string()],
        files_ignored: 0,
    });
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain(&mut app).await?;
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    drain(&mut app).await?;
    app.navigator_mut().active_view = ViewMode::Agent;
    Ok((app, mock))
}

/// Type `/merge-worktree` + Enter in s-1's composer; the dialog opens.
async fn run_merge_worktree(app: &mut App) -> TestResult {
    type_text(app, "/merge-worktree").await?;
    press(app, KeyCode::Enter).await?;
    if app.compositor().contains(MERGE_DIALOG_ID) {
        Ok(())
    } else {
        Err(TestError::from(format!(
            "/merge-worktree did not open the dialog:\n{}",
            screen(app)
        )))
    }
}

fn session_text(app: &App, id: &SessionId) -> String {
    app.agent_view_store()
        .session_context_for(id)
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| {
            chunk.lines.iter().map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The board with the daemon's (empty) workspace list loaded.
async fn board_with_no_workspaces() -> Result<App, TestError> {
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::WorkspacesLoaded(Vec::new()));
    drain(&mut app).await?;
    app.navigator_mut().active_view = ViewMode::Board;
    Ok(app)
}

/// Scenario: Enter on Merge merges the session's worktree
#[tokio::test(start_paused = true)]
async fn enter_on_merge_merges_the_sessions_worktree() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    let (mut app, mock) = session_with_worktree_changes().await?;

    // @step When the user runs /merge-worktree
    run_merge_worktree(&mut app).await?;

    // @step And presses Enter on "Merge" in the "Merge Worktree" dialog
    assert_shows(&screen(&mut app), "Merge Worktree")?;
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the dialog is closed
    assert!(!app.compositor().contains(MERGE_DIALOG_ID));

    // @step And the backend merges the worktree of s-1 once
    assert_eq!(mock.merge_session_worktree_calls(), 1);

    // @step And nothing is discarded
    assert_eq!(mock.discard_session_worktree_calls(), 0);
    Ok(())
}

/// Scenario: Enter on Discard discards the session's worktree
#[tokio::test(start_paused = true)]
async fn enter_on_discard_discards_the_sessions_worktree() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    let (mut app, mock) = session_with_worktree_changes().await?;

    // @step When the user runs /merge-worktree
    run_merge_worktree(&mut app).await?;

    // @step And presses Right and Enter on "Discard" in the "Merge Worktree" dialog
    press(&mut app, KeyCode::Right).await?;
    press(&mut app, KeyCode::Enter).await?;

    // @step Then the backend discards the worktree of s-1 once
    assert_eq!(mock.discard_session_worktree_calls(), 1);

    // @step And nothing is merged
    assert_eq!(mock.merge_session_worktree_calls(), 0);

    // @step And s-1 shows "[discard] worktree discarded"
    assert_shows(
        &session_text(&app, &sid("s-1")),
        "[discard] worktree discarded",
    )
}

/// Scenario: Clicking Merge merges the session's worktree
#[tokio::test(start_paused = true)]
async fn clicking_merge_merges_the_sessions_worktree() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    let (mut app, mock) = session_with_worktree_changes().await?;

    // @step When the user runs /merge-worktree
    run_merge_worktree(&mut app).await?;

    // @step And clicks "Merge" in the "Merge Worktree" dialog
    let buffer = render(&mut app);
    let title_row = (0..HEIGHT)
        .find(|&y| row_text(&buffer, y).contains("Merge Worktree"))
        .or_fail("the dialog title is painted")?;
    let (x, y) = (title_row + 1..HEIGHT)
        .find_map(|y| {
            let row = row_text(&buffer, y);
            let byte = row.find("Discard")?;
            let merge = row[..byte].rfind("Merge")?;
            Some((row[..merge].chars().count() as u16 + 2, y))
        })
        .or_fail("the Merge button is painted")?;
    let _ = app.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
    drain(&mut app).await?;

    // @step Then the backend merges the worktree of s-1 once
    assert_eq!(mock.merge_session_worktree_calls(), 1);
    assert_eq!(mock.discard_session_worktree_calls(), 0);
    Ok(())
}

/// Scenario: Esc closes the merge dialog and changes nothing
#[tokio::test(start_paused = true)]
async fn esc_closes_the_merge_dialog_and_changes_nothing() -> TestResult {
    // @step Given session s-1 is focused and its worktree has changes
    let (mut app, mock) = session_with_worktree_changes().await?;

    // @step When the user runs /merge-worktree
    run_merge_worktree(&mut app).await?;

    // @step And presses Esc in the "Merge Worktree" dialog
    press(&mut app, KeyCode::Esc).await?;

    // @step Then the dialog is closed
    assert!(!app.compositor().contains(MERGE_DIALOG_ID));

    // @step And nothing is merged or discarded
    assert_eq!(mock.merge_session_worktree_calls(), 0);
    assert_eq!(mock.discard_session_worktree_calls(), 0);
    Ok(())
}

/// Scenario: /debug with no session open says whether capture is on for the next session
#[tokio::test(start_paused = true)]
async fn debug_with_no_session_says_whether_capture_is_on_for_the_next_session() -> TestResult {
    // @step Given no session is open
    let (mut app, _mock) = fresh_app();
    app.navigator_mut().active_view = ViewMode::Agent;
    assert!(app.current_session().is_none());

    // @step When the user picks /debug
    app.dispatch(Action::SlashCommandSelected(
        None,
        SlashCommandAction::Debug,
    ));
    drain(&mut app).await?;

    // @step Then a notification says "Debug capture will be on for the next session"
    assert!(app.compositor().contains(NOTIFICATION_DIALOG_ID));
    assert_shows(
        &screen(&mut app),
        "Debug capture will be on for the next session",
    )?;
    assert!(app.pre_session_debug_enabled());

    // @step When the user picks /debug again
    app.dispatch(Action::SlashCommandSelected(
        None,
        SlashCommandAction::Debug,
    ));
    drain(&mut app).await?;

    // @step Then a notification says "Debug capture is off"
    let text = screen(&mut app);
    assert_shows(&text, "Debug capture is off")?;
    assert!(!text.contains("will be on"), "{text}");
    assert!(!app.pre_session_debug_enabled());
    Ok(())
}

/// Scenario: w with no workspaces listed opens the dropdown so the first workspace can be added
#[tokio::test(start_paused = true)]
async fn w_with_no_workspaces_opens_the_dropdown_so_the_first_can_be_added() -> TestResult {
    // @step Given the daemon lists no workspaces
    let mut app = board_with_no_workspaces().await?;

    // @step When the user presses "w" on the board
    press(&mut app, KeyCode::Char('w')).await?;

    // @step Then the workspace dropdown is open and shows "No workspaces"
    assert!(app.board_store().workspace_dropdown().is_some());
    let text = screen(&mut app);
    assert_shows(&text, "No workspaces")?;

    // @step And its key hint shows "a add" and not "d remove"
    assert_shows(&text, "a add")?;
    assert!(!text.contains("d remove"), "{text}");

    // @step When the user presses "a"
    press(&mut app, KeyCode::Char('a')).await?;

    // @step Then the Add workspace dialog is open
    assert!(app.compositor().contains(ADD_WORKSPACE_DIALOG_ID));
    Ok(())
}

/// Scenario: Settings > Workspaces with no workspaces listed opens the dropdown
#[tokio::test(start_paused = true)]
async fn settings_workspaces_with_no_workspaces_opens_the_dropdown() -> TestResult {
    // @step Given the daemon lists no workspaces
    let mut app = board_with_no_workspaces().await?;

    // @step When the user picks Settings > Workspaces
    app.dispatch(Action::MenuOpenDropdown(2));
    drain(&mut app).await?;
    assert_shows(&screen(&mut app), "Workspaces")?;
    app.dispatch(Action::MenuExecuteItem {
        category: 2,
        row: 3,
    });
    drain(&mut app).await?;

    // @step Then the workspace dropdown is open and shows "No workspaces"
    assert!(app.board_store().workspace_dropdown().is_some());
    assert_shows(&screen(&mut app), "No workspaces")
}
