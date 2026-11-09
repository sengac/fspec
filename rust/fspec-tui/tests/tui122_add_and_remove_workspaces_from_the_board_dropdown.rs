//! TUI-122 — add and remove workspaces from the board workspace dropdown.
//!
//! Feature: spec/features/add-and-remove-workspaces-from-the-board-dropdown.feature

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkspaceEntry};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::workspace_mock::session_in;
use common::MockBackend;

const ALPHA: &str = "/work/alpha";
const BETA: &str = "/work/beta";
const DIALOG: &str = "add-workspace-dialog";

fn entry(path: &str, name: &str, start: bool) -> WorkspaceEntry {
    WorkspaceEntry {
        path: path.into(),
        name: name.into(),
        start,
    }
}

async fn booted_app(
    entries: Vec<WorkspaceEntry>,
) -> Result<(App, Arc<MockBackend>), Box<dyn std::error::Error>> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_workspaces(entries);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await?;
    settle(&mut app).await?;
    Ok((app, mock))
}

async fn settle(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
        }
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.handle_event(&key(KeyCode::Char(character)));
    }
}

fn screen(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    let mut term = Terminal::new(TestBackend::new(120, 24))?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))?;
    let buf: &Buffer = term.backend().buffer();
    Ok((0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

#[tokio::test(flavor = "current_thread")]
async fn adding_a_workspace_from_the_dropdown_lists_and_shows_it() -> TestResult {
    // @step Given the board lists workspace "alpha" and its dropdown is open
    let (mut app, mock) = booted_app(vec![entry(ALPHA, "alpha", true)]).await?;
    app.handle_event(&key(KeyCode::Char('w')));
    settle(&mut app).await?;
    // @step When the user presses "a", types "/work/beta" and presses Enter
    app.handle_event(&key(KeyCode::Char('a')));
    settle(&mut app).await?;
    assert!(app.compositor().contains(DIALOG));
    type_text(&mut app, BETA);
    app.handle_event(&key(KeyCode::Enter));
    settle(&mut app).await?;
    // @step Then the daemon is asked to add "/work/beta"
    assert!(mock
        .workspace_admin_requests()
        .contains(&format!("add:{BETA}")));
    // @step And the dropdown lists "beta" and the board shows it
    assert!(!app.compositor().contains(DIALOG));
    assert!(app
        .board_store()
        .workspaces()
        .iter()
        .any(|e| e.path == BETA));
    assert_eq!(app.board_store().selected_other_workspace(), Some(BETA));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_refused_path_reopens_the_dialog_with_the_reason() -> TestResult {
    // @step Given the daemon refuses "/tmp/plain" as not an fspec project
    let (mut app, mock) = booted_app(vec![entry(ALPHA, "alpha", true)]).await?;
    mock.fail_add_workspace("not an fspec project (no spec/ directory): /tmp/plain");
    // @step When the user adds "/tmp/plain" from the dropdown
    app.handle_event(&key(KeyCode::Char('w')));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Char('a')));
    settle(&mut app).await?;
    type_text(&mut app, "/tmp/plain");
    app.handle_event(&key(KeyCode::Enter));
    settle(&mut app).await?;
    // @step Then the add-workspace dialog is open again with "/tmp/plain"
    assert!(app.compositor().contains(DIALOG));
    let text = screen(&mut app)?;
    assert!(text.contains("/tmp/plain"), "{text}");
    // @step And it shows the daemon's reason
    assert!(text.contains("not an fspec project"), "{text}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn removing_the_highlighted_workspace_from_the_dropdown() -> TestResult {
    // @step Given the board shows workspace "beta" and the dropdown highlights it
    let (mut app, mock) = booted_app(vec![
        entry(ALPHA, "alpha", true),
        entry(BETA, "beta", false),
    ])
    .await?;
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Char('w')));
    settle(&mut app).await?;
    assert_eq!(app.board_store().workspace_dropdown(), Some(1));
    // @step When the user presses "d"
    app.handle_event(&key(KeyCode::Char('d')));
    settle(&mut app).await?;
    // @step Then the daemon is asked to remove "/work/beta"
    assert!(mock
        .workspace_admin_requests()
        .contains(&format!("remove:{BETA}")));
    // @step And the board shows the start workspace "alpha"
    assert!(!app
        .board_store()
        .workspaces()
        .iter()
        .any(|e| e.path == BETA));
    assert_eq!(app.board_store().selected_other_workspace(), None);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_start_workspace_and_a_workspace_with_open_sessions_are_not_removed() -> TestResult {
    // @step Given the dropdown highlights the start workspace, or a workspace with an open session
    let (mut app, mock) = booted_app(vec![
        entry(ALPHA, "alpha", true),
        entry(BETA, "beta", false),
    ])
    .await?;
    app.dispatch(Action::SessionObserved(session_in("s-beta", BETA)));
    app.dispatch(Action::SessionCreated(SessionId::new("s-beta")));
    settle(&mut app).await?;
    // @step When the user presses "d"
    app.handle_event(&key(KeyCode::Char('w')));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Char('d')));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Esc));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Char('w')));
    settle(&mut app).await?;
    app.handle_event(&key(KeyCode::Down));
    settle(&mut app).await?;
    assert_eq!(app.board_store().workspace_dropdown(), Some(1));
    app.handle_event(&key(KeyCode::Char('d')));
    settle(&mut app).await?;
    // @step Then the daemon is not asked to remove it
    assert!(
        mock.workspace_admin_requests().is_empty(),
        "{:?}",
        mock.workspace_admin_requests()
    );
    // @step And an error explains why
    assert!(app.compositor().contains("session-storage-dialog"));
    let text = screen(&mut app)?;
    assert!(text.contains("open session"), "{text}");
    Ok(())
}
