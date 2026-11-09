// Feature: spec/features/file-search-popup-uses-worktree-path-for-isolated-sessions.feature
//! BUG-541 — @file search in an isolated session lists the session's
//! worktree: the agent view asks the daemon to search in that session.
//!
//! Drives the real `App` (handle_event + dispatch) against the recording
//! `MockBackend` on a paused tokio clock; the typed `@src` reaches the
//! backend through the real popup and dispatch path.

use crate::common::TaskOutcome;
use codelet_test_helpers::{TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, StreamChunk, WorkspaceEntry};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// An agent-view App with workspaces alpha (start) and beta listed and
/// beta chosen on the board.
fn app_showing_beta(mock: &Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.board_store_mut().set_workspaces(vec![
        WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: "/work/beta".into(),
            name: "beta".into(),
            start: false,
        },
    ]);
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
    app.navigator_mut().active_view = ViewMode::Agent;
    app
}

/// Focus session s-1, isolated in a worktree or not.
async fn focus_s1(app: &mut App, isolated: bool) -> TestResult {
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IsolationStateChange {
            is_isolated: isolated,
            worktree_path: isolated.then(|| "/work/beta/.fspec/worktrees/s-1".to_string()),
            base_commit: None,
        },
    ));
    drain(app).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    Ok(())
}

/// Run every parked task and queued action until the App is quiet.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..50 {
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
    Ok(())
}

fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 160, 40);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for character in text.chars() {
        let _ = screen(app);
        let _ = app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char(character),
            KeyModifiers::NONE,
        )));
        drain(app).await?;
    }
    Ok(())
}

fn requests_of(mock: &MockBackend, kind: &str) -> Vec<String> {
    mock.workspace_scope_requests()
        .into_iter()
        .filter(|request| request.starts_with(&format!("{kind}:")))
        .collect()
}

fn no_requests(mock: &MockBackend, kind: &str) -> TestResult {
    let requests = requests_of(mock, kind);
    if requests.is_empty() {
        Ok(())
    } else {
        Err(TestError::from(format!(
            "unexpected {kind} requests: {requests:?}"
        )))
    }
}

// Scenario: The agent view searches an isolated session's files in that session
#[tokio::test(start_paused = true)]
async fn the_agent_view_searches_an_isolated_sessions_files_in_that_session() -> TestResult {
    // @step Given isolated session "s-1" is focused in the agent view
    let mock = Arc::new(MockBackend::new());
    let mut app = app_showing_beta(&mock);
    focus_s1(&mut app, true).await?;
    // @step When the user types "@src"
    type_text(&mut app, "@src").await?;
    // @step Then the file search names session "s-1"
    let session_searches = requests_of(&mock, "search_files_in_session");
    assert!(
        !session_searches.is_empty(),
        "no session file search was made"
    );
    assert!(
        session_searches
            .iter()
            .all(|request| request == "search_files_in_session:s-1"),
        "{session_searches:?}"
    );
    // @step And no workspace file search is made
    no_requests(&mock, "search_files")?;
    Ok(())
}

// Scenario: The agent view searches a non-isolated session's files in that session
// BUG-618: every session's @file search uses its own working directory.
#[tokio::test(start_paused = true)]
async fn the_agent_view_searches_a_non_isolated_sessions_files_in_that_session() -> TestResult {
    // @step Given the board shows workspace "beta"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_showing_beta(&mock);
    // @step And non-isolated session "s-1" is focused in the agent view
    focus_s1(&mut app, false).await?;
    // @step When the user types "@src"
    type_text(&mut app, "@src").await?;
    // @step Then the file search names session "s-1"
    let session_searches = requests_of(&mock, "search_files_in_session");
    assert!(
        !session_searches.is_empty(),
        "no session file search was made"
    );
    assert!(
        session_searches
            .iter()
            .all(|request| request == "search_files_in_session:s-1"),
        "{session_searches:?}"
    );
    // @step And no workspace file search is made
    no_requests(&mock, "search_files")?;
    Ok(())
}

// Scenario: A failed session file search is shown
#[tokio::test(start_paused = true)]
async fn a_failed_session_file_search_is_shown() -> TestResult {
    // @step Given isolated session "s-1" is focused in the agent view
    let mock = Arc::new(MockBackend::new());
    let mut app = app_showing_beta(&mock);
    focus_s1(&mut app, true).await?;
    // @step And the daemon cannot search the files of session "s-1"
    mock.fail_session_file_search("the working directory of session s-1 is unknown");
    // @step When the user types "@src"
    type_text(&mut app, "@src").await?;
    // @step Then the TUI shows that the files could not be searched
    let shown = screen(&mut app);
    assert!(shown.contains("The files could not be searched"), "{shown}");
    Ok(())
}

// Scenario: With no session open, @file search says there are no files to search
// BUG-618 (product owner, 2026-11-08): with no session there is no working
// directory to search, and no other directory is guessed.
#[tokio::test(start_paused = true)]
async fn with_no_session_open_file_search_says_there_are_no_files_to_search() -> TestResult {
    // @step Given the board shows workspace "beta"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_showing_beta(&mock);
    // @step And no session is open in the agent view
    assert_eq!(app.current_session(), None);
    // @step When the user types "@src"
    type_text(&mut app, "@src").await?;
    // @step Then the TUI shows 'no session is open, so there are no files to search'
    let shown = screen(&mut app);
    assert!(
        shown.contains("no session is open, so there are no files to search"),
        "{shown}"
    );
    // @step And no file search is made
    no_requests(&mock, "search_files")?;
    no_requests(&mock, "search_files_in_session")?;
    Ok(())
}
